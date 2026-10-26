use crate::commands::MultiRepo;
use crate::domain::git::repository::RepoInfo;
use crate::model::plan::ExecutionResult;
use crate::utils::output;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Instant;

pub(crate) enum RepoStatus {
    Success,
    Failed(String),
    Skipped(String),
}

pub(crate) struct RepoOutcome {
    path: PathBuf,
    result: ExecutionResult,
    status: RepoStatus,
}

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

    pub(crate) fn begin(&self) {
        self.in_flight.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn complete(&self) -> usize {
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        self.completed.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub(crate) fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::SeqCst)
    }
}

/// Determine the effective parallelism level.
pub(crate) fn resolve_jobs(jobs: usize, total: usize) -> usize {
    let jobs = if jobs == 0 {
        thread::available_parallelism().map_or(1, |n| n.get())
    } else {
        jobs
    };
    jobs.min(total)
}

/// Run a multi-repo command in parallel using a fixed-size thread pool.
///
/// Pre-allocates sequential indices for each repository, dispatches them to `jobs` worker
/// threads via `std::thread::scope`. Each worker picks the next available index, processes
/// the repository with buffered output, and aggregates results into a `Mutex<Vec<RepoOutcome>>`.
/// After all workers complete, prints a summary footer.
pub(crate) fn run_parallel<C: MultiRepo + Sync>(cmd: &C, repos: &[RepoInfo], jobs: usize) {
    let total = repos.len();
    let cursor = AtomicUsize::new(0);
    let progress = SyncProgress::new();
    let outcomes = Mutex::new(Vec::with_capacity(total));

    let started = Instant::now();
    output::parallel_start(total, jobs);

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

pub(crate) fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(text)
        .to_string()
}

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
