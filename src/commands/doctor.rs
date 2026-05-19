use crate::commands::MultiRepo;
use crate::commands::{RepoPathArgs, init_repo_walker};
use crate::domain::git::{Diagnosis, collect_context, diagnose_repo};
use crate::engine::plan;
use crate::error::{AppError, Result};
use crate::model::git::GitContext;
use crate::model::operation::GitOperation;
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase};
use std::path::Path;

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
    git_ctx: Option<GitContext>,
    issues: Vec<Diagnosis>,
}

impl MultiRepo for DoctorArgs {
    type Context = DoctorContext;
    type Plan = ExecutionPlan;

    fn collect(&self, repo_path: &Path) -> Result<DoctorContext> {
        let issues = diagnose_repo(repo_path).map_err(|e| {
            AppError::release(format!("诊断仓库 {} 失败: {}", repo_path.display(), e))
        })?;
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
                Diagnosis::StaleRefs { remote } => {
                    fix_phase.add(GitOperation::PruneRemote {
                        remote: remote.clone(),
                        working_dir: repo_path.to_path_buf(),
                    });
                }
                Diagnosis::NoRemoteTrackingBranch | Diagnosis::SingleLocalBranch => {
                    fix_phase.add(GitOperation::SetUpstream {
                        remote: git_ctx
                            .preferred_remote()
                            .or_else(|| git_ctx.first_remote_name())
                            .unwrap_or_else(|| "origin".to_string()),
                        branch: git_ctx.current_branch.clone(),
                        working_dir: repo_path.to_path_buf(),
                    });
                }
                Diagnosis::LargeRepo => {
                    fix_phase.add(GitOperation::Gc {
                        working_dir: repo_path.to_path_buf(),
                    });
                }
                Diagnosis::StashExists => {
                    fix_phase.add_message(DisplayMessage::Warning {
                        msg: "stash 条目需要手动处理".to_string(),
                    });
                }
                Diagnosis::RemoteNameMismatch {
                    current, expected, ..
                } => {
                    if git_ctx.has_remote(expected) {
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
                Diagnosis::DetachedHead | Diagnosis::NoRemote => {}
            }
        }

        if !fix_phase.is_empty() {
            plan.add_phase(fix_phase);
        }

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

pub fn run(args: DoctorArgs) -> Result<()> {
    let Some(walker) = init_repo_walker(&args.repo_path)? else {
        return Ok(());
    };

    crate::commands::run_multi_repo(&args, &walker)
}
