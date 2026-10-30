//! `pma doctor`：检查每个仓库的健康状况，并在 `--fix` 下生成对应的修复计划。
//! 诊断本身是只读的（见 `domain::git::diagnose`），本模块只决定「什么问题该用什么动作修」。

use crate::commands::MultiRepo;
use crate::commands::RepoPathArgs;
use crate::domain::git::GitOperation;
use crate::domain::git::{Diagnosis, collect_context, diagnose_repo};
use crate::engine::plan;
use crate::error::Result;
use crate::model::git::GitContext;
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase};
use std::path::Path;

/// `pma doctor` 的命令行参数
#[derive(Debug, clap::Args)]
pub struct DoctorArgs {
    #[command(flatten)]
    pub repo_path: RepoPathArgs,
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Automatically fix detected issues"
    )]
    pub fix: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be fixed"
    )]
    pub dry_run: bool,
}

#[derive(Debug)]
pub(crate) struct DoctorContext {
    /// 只有需要修复时才采集，纯检查场景为 None
    git_ctx: Option<GitContext>,
    issues: Vec<Diagnosis>,
}

impl MultiRepo for DoctorArgs {
    type Context = DoctorContext;
    type Plan = ExecutionPlan;

    fn collect(&self, repo_path: &Path) -> Result<DoctorContext> {
        let issues = diagnose_repo(repo_path)?;
        // 修复动作需要 remote / 分支信息，只在这时才多查一次上下文。
        // 注意：不加 --fix 时 git_ctx 为 None，plan 直接返回空计划（--dry-run 需配合 --fix 才有输出）
        let git_ctx = if self.fix && !issues.is_empty() {
            collect_context(repo_path).ok()
        } else {
            None
        };

        Ok(DoctorContext { git_ctx, issues })
    }

    fn plan(&self, ctx: &DoctorContext, repo_path: &Path) -> Result<ExecutionPlan> {
        let Some(git_ctx) = &ctx.git_ctx else {
            return Ok(ExecutionPlan::new());
        };

        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);
        let mut fix_phase = Phase::new("修复问题");

        for issue in &ctx.issues {
            match issue {
                // 远端已删除的跟踪分支，prune 掉本地引用
                Diagnosis::StaleRefs { remote } => {
                    fix_phase.add(GitOperation::PruneRemote {
                        remote: remote.clone(),
                        working_dir: repo_path.to_path_buf(),
                    });
                }
                // 本地分支没跟上远端，补一条 upstream
                Diagnosis::NoRemoteTrackingBranch | Diagnosis::SingleLocalBranch => {
                    fix_phase.add(GitOperation::SetUpstream {
                        // 优先按配置挑首选 remote，实在没有就兜底 origin
                        remote: git_ctx
                            .preferred_remote()
                            .or_else(|| git_ctx.first_remote_name())
                            .unwrap_or_else(|| "origin".to_string()),
                        branch: git_ctx.current_branch.clone(),
                        working_dir: repo_path.to_path_buf(),
                    });
                }
                // 体积超阈值才 gc，日常检查不做这个耗时操作
                Diagnosis::LargeRepo => {
                    fix_phase.add(GitOperation::Gc {
                        working_dir: repo_path.to_path_buf(),
                    });
                }
                // stash 内容属于用户未落地的工作，只能提示不能自动清理
                Diagnosis::StashExists => {
                    fix_phase.add_message(DisplayMessage::Warning {
                        msg: "stash 条目需要手动处理".to_string(),
                    });
                }
                Diagnosis::RemoteNameMismatch {
                    current, expected, ..
                } => {
                    if git_ctx.has_remote(expected) {
                        // 目标名已存在时重命名会失败，跳过并让用户自己处理
                        fix_phase.add_message(DisplayMessage::Warning {
                            msg: format!("目标 remote 名称 {} 已存在，跳过", expected),
                        });
                    } else {
                        fix_phase.add(GitOperation::RenameRemote {
                            old: current.clone(),
                            new: expected.clone(),
                            working_dir: repo_path.to_path_buf(),
                        });
                    }
                }
                // 这两种状态只能由用户决定怎么收场（切回分支 / 自己加 remote）
                Diagnosis::DetachedHead | Diagnosis::NoRemote => {}
            }
        }

        // 没有可修的动作就不要加空 phase，免得输出里出现空标题
        if !fix_phase.is_empty() {
            plan.add_phase(fix_phase);
        }

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

/// `pma doctor` 入口；按仓库串行执行
pub fn run(args: DoctorArgs) -> Result<()> {
    crate::commands::run_multi_repo_cmd(&args, &args.repo_path)
}
