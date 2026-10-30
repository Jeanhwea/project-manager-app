//! 多仓库并行调度：固定大小的 thread pool 配共享游标领任务，最后统一汇总打印。
//! 串行场景不走这里（见 `commands::multi_repo` 的串行分支），本模块只在 jobs > 1 时被调用。

use crate::commands::MultiRepo;
use crate::domain::git::repository::RepoInfo;
use crate::model::plan::ExecutionResult;
use crate::utils::output;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Instant;

/// 单个仓库的最终处理状态
pub(crate) enum RepoStatus {
    Success,
    Failed(String),
    Skipped(String),
}

/// 一个仓库的执行结果与状态，暂存在共享队列里等待汇总阶段消费
pub(crate) struct RepoOutcome {
    path: PathBuf,
    result: ExecutionResult,
    status: RepoStatus,
}

/// 跨 worker 线程共享的进度计数器
pub(crate) struct SyncProgress {
    completed: AtomicUsize,
    in_flight: AtomicUsize,
}

impl SyncProgress {
    pub(crate) fn new() -> Self {
        Self {
            completed: AtomicUsize::new(0),
            in_flight: AtomicUsize::new(0),
        }
    }

    /// 标记一个仓库开始处理
    pub(crate) fn begin(&self) {
        self.in_flight.fetch_add(1, Ordering::SeqCst);
    }

    /// 回报本仓库处理完毕后的完成序号（从 1 开始计数）
    pub(crate) fn complete(&self) -> usize {
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        self.completed.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// 尚未处理完的仓库数（含正在处理的那个）
    pub(crate) fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::SeqCst)
    }
}

/// 算出实际并发度：`jobs == 0` 时按 CPU 核数自动取值，最后不超过仓库总数
pub(crate) fn resolve_jobs(jobs: usize, total: usize) -> usize {
    let jobs = if jobs == 0 {
        thread::available_parallelism().map_or(1, |n| n.get())
    } else {
        jobs
    };
    jobs.min(total)
}

/// 用固定大小的线程池并行处理多个仓库。
///
/// worker 通过共享游标领取仓库下标，仓库的输出先缓冲到内存、整块打印以避免交错，
/// 结果统一收集后进汇总；全部 worker 结束（由 `thread::scope` 保证）再打印总结 footer。
pub(crate) fn run_parallel<C: MultiRepo + Sync>(cmd: &C, repos: &[RepoInfo], jobs: usize) {
    let total = repos.len();
    // 共享游标 + 各 worker 自取下标：比预先均分成 jobs 段更能吸收慢仓库带来的倾斜
    let cursor = AtomicUsize::new(0);
    let progress = SyncProgress::new();
    let outcomes = Mutex::new(Vec::with_capacity(total));

    let started = Instant::now();
    output::parallel_start(total, jobs);

    // scope 保证汇总前所有 worker 都已退出，因此可以安全借用外部的 repos / cmd
    thread::scope(|scope| {
        for _ in 0..jobs {
            scope.spawn(|| {
                loop {
                    let index = cursor.fetch_add(1, Ordering::SeqCst);
                    if index >= total {
                        break;
                    }
                    progress.begin();
                    let outcome =
                        process_repo_buffered(cmd, index, total, &repos[index].path, &progress);
                    // worker panic 会让锁中毒，这里照常取出数据继续汇总而不是二次 panic
                    outcomes
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push(outcome);
                }
            });
        }
    });

    let outcomes = outcomes
        .into_inner()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    print_summary(outcomes, total, started.elapsed());
}

/// 处理单个仓库：先把该仓库的输出缓冲下来，收尾时整块 flush，
/// 避免并发下多个仓库的输出行交错
fn process_repo_buffered<C: MultiRepo>(
    cmd: &C,
    index: usize,
    total: usize,
    repo_path: &Path,
    progress: &SyncProgress,
) -> RepoOutcome {
    output::capture_start();
    output::repo_header(index + 1, total, repo_path);

    let mut result = ExecutionResult::new();
    let status = match cmd.collect(repo_path) {
        Ok(ctx) => match cmd.plan(&ctx, repo_path) {
            Ok(plan) => match cmd.execute(&plan) {
                Ok(executed) => {
                    let status = if executed.is_success() {
                        RepoStatus::Success
                    } else {
                        RepoStatus::Failed(failure_reason(&executed))
                    };
                    result = executed;
                    status
                }
                Err(e) => {
                    output::error_chain(&e);
                    RepoStatus::Failed(first_line(&e.to_string()))
                }
            },
            Err(e) => {
                output::error_chain(&e);
                RepoStatus::Failed(first_line(&e.to_string()))
            }
        },
        Err(e) => {
            // collect 阶段失败说明这个仓库压根不满足处理前提（如非 git 仓库），按跳过而非失败计
            output::warning(&format!("跳过 {}: {}", repo_path.display(), e));
            RepoStatus::Skipped(first_line(&e.to_string()))
        }
    };

    let lines = output::capture_finish();
    let done = progress.complete();
    let count = output::ProgressCount {
        done,
        total,
        in_flight: progress.in_flight(),
    };
    output::flush_block(&lines, Some(count));

    RepoOutcome {
        path: repo_path.to_path_buf(),
        result,
        status,
    }
}

/// 取第一条错误的描述，并剥掉 `[工作目录] ` 前缀，汇总里只显示命令行
pub(crate) fn failure_reason(result: &ExecutionResult) -> String {
    let Some(err) = result.errors().first() else {
        return "执行失败".to_string();
    };
    let description = err.description();
    description
        .strip_prefix('[')
        .and_then(|rest| rest.split_once("] "))
        .map_or_else(|| description.to_string(), |(_, body)| body.to_string())
}

/// 取首个非空行，避免多行错误信息把汇总页撑乱
pub(crate) fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(text)
        .to_string()
}

/// 打印汇总页：成功数、操作总数、耗时，以及失败/跳过的明细清单
fn print_summary(outcomes: Vec<RepoOutcome>, total: usize, elapsed: std::time::Duration) {
    let mut merged = ExecutionResult::new();
    let mut succeeded = 0usize;
    let mut failures: Vec<(PathBuf, String)> = Vec::new();
    let mut skips: Vec<(PathBuf, String)> = Vec::new();

    for outcome in outcomes {
        let RepoOutcome {
            path,
            result,
            status,
        } = outcome;
        merged.merge(result);
        match status {
            RepoStatus::Success => succeeded += 1,
            RepoStatus::Failed(reason) => failures.push((path, reason)),
            RepoStatus::Skipped(reason) => skips.push((path, reason)),
        }
    }

    output::section("执行汇总");
    output::item("仓库", &format!("{}/{}", succeeded, total));
    output::item("操作", &merged.executed_count().to_string());
    output::item("耗时", &crate::utils::format_duration(elapsed.as_secs()));

    if !failures.is_empty() {
        output::item("失败", &format!("{} 个", failures.len()));
        for (path, reason) in &failures {
            output::error(&format!(
                "{} ({})",
                crate::utils::path::format_path(path),
                reason
            ));
        }
    }

    if !skips.is_empty() {
        output::item("跳过", &format!("{} 个", skips.len()));
        for (path, reason) in &skips {
            output::warning(&format!(
                "{} ({})",
                crate::utils::path::format_path(path),
                reason
            ));
        }
    }
}
