use crate::commands::RepoPathArgs;
use crate::control::command::MultiRepoCommand;
use crate::domain::config::ConfigManager;
use crate::domain::git::collect_context;
use crate::domain::git::repository::RepoWalker;
use crate::error::{AppError, Result};
use crate::model::git::GitContext;
use crate::model::plan::{ExecutionPlan, GitOperation, MessageOperation};
use crate::utils::output::Output;
use std::path::Path;

#[derive(Debug, clap::Args)]
pub struct SyncArgs {
    #[command(flatten)]
    pub repo_path: RepoPathArgs,
    #[arg(long, short, help = "Target remote name (e.g. origin, upstream)")]
    pub remote: Option<String>,
    #[arg(
        long,
        short = 'A',
        default_value = "true",
        help = "Push to all remotes when no remote is specified"
    )]
    pub all_remotes: bool,
    #[arg(
        long,
        short = 'a',
        default_value = "false",
        help = "Sync all branches before pulling (pull current branch first)"
    )]
    pub all_branches: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show commands without executing"
    )]
    pub dry_run: bool,
}

#[derive(Debug)]
pub(crate) struct SyncContext {
    git_ctx: GitContext,
    target_remotes: Vec<String>,
    should_push: bool,
    sync_all_branches: bool,
}

impl MultiRepoCommand for SyncArgs {
    type Context = SyncContext;

    fn context(&self, repo_path: &Path) -> Result<SyncContext> {
        let git_ctx = collect_context(repo_path)?;

        if git_ctx.remotes.is_empty() {
            return Ok(SyncContext {
                git_ctx,
                target_remotes: vec![],
                should_push: false,
                sync_all_branches: self.all_branches,
            });
        }

        let target_remotes =
            resolve_target_remotes(&git_ctx, self.remote.as_deref(), self.all_remotes)?;

        let should_push = !target_remotes.is_empty()
            && target_remotes
                .iter()
                .any(|remote| should_push_to_remote(remote));

        Ok(SyncContext {
            git_ctx,
            target_remotes,
            should_push,
            sync_all_branches: self.all_branches,
        })
    }

    fn plan(&self, ctx: &SyncContext, _repo_path: &Path) -> Result<ExecutionPlan> {
        if ctx.git_ctx.remotes.is_empty() || ctx.target_remotes.is_empty() {
            return skip_plan("无远程仓库");
        }

        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);
        let current_branch = &ctx.git_ctx.current_branch;

        if ctx.sync_all_branches {
            let other_branches: Vec<&crate::model::git::Branch> = ctx
                .git_ctx
                .local_branches()
                .iter()
                .filter(|b| b.name != current_branch.as_str())
                .cloned()
                .collect();

            if !other_branches.is_empty() {
                plan.add(MessageOperation::Header {
                    title: "同步所有本地分支".to_string(),
                });

                for branch in &other_branches {
                    for remote in &ctx.target_remotes {
                        if ctx.git_ctx.has_remote_branch(remote, &branch.name) {
                            plan.add(GitOperation::Checkout {
                                ref_name: branch.name.clone(),
                            });
                            plan.add(GitOperation::Pull {
                                remote: remote.clone(),
                                branch: branch.name.clone(),
                            });
                        } else {
                            plan.add(MessageOperation::Skip {
                                msg: format!("跳过 {} (远程无此分支)", branch.name),
                            });
                        }
                    }
                }

                plan.add(GitOperation::Checkout {
                    ref_name: current_branch.clone(),
                });
            }
        }

        plan.add(MessageOperation::Header {
            title: "同步当前分支".to_string(),
        });

        for remote in &ctx.target_remotes {
            if ctx.git_ctx.has_remote_branch(remote, current_branch) {
                plan.add(GitOperation::Pull {
                    remote: remote.clone(),
                    branch: current_branch.clone(),
                });
            } else {
                plan.add(MessageOperation::Skip {
                    msg: format!("跳过拉取 {}/{} (远程无此分支)", remote, current_branch),
                });
            }
        }

        if ctx.should_push {
            for remote in &ctx.target_remotes {
                plan.add(GitOperation::PushAll {
                    remote: remote.clone(),
                });
                plan.add(GitOperation::PushTags {
                    remote: remote.clone(),
                });
            }
        } else {
            for remote in &ctx.target_remotes {
                plan.add(MessageOperation::Skip {
                    msg: format!("跳过推送到 {} (配置 skip_push_remotes)", remote),
                });
            }
        }

        Ok(plan)
    }
}

pub fn run(args: SyncArgs) -> Result<()> {
    let effective_path = resolve_effective_path(&args.repo_path.path)?;
    let walker = RepoWalker::new(&effective_path, args.repo_path.max_depth)?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    MultiRepoCommand::run(&args, &walker)
}

fn resolve_effective_path(path: &str) -> Result<std::path::PathBuf> {
    if path == "." {
        let cwd = std::env::current_dir()?;
        Ok(find_git_repository_upwards(&cwd).unwrap_or(cwd))
    } else {
        Ok(crate::utils::path::canonicalize_path(path)?)
    }
}

fn resolve_target_remotes(
    git_ctx: &GitContext,
    explicit_remote: Option<&str>,
    all_remotes: bool,
) -> Result<Vec<String>> {
    if let Some(name) = explicit_remote {
        if !git_ctx.has_remote(name) {
            return Err(AppError::not_found(format!("远程仓库 {} 不存在", name)));
        }
        return Ok(vec![name.to_string()]);
    }

    if all_remotes {
        return Ok(git_ctx.remotes.iter().map(|r| r.name.clone()).collect());
    }

    Ok(git_ctx.remotes.iter().map(|r| r.name.clone()).collect())
}

fn should_push_to_remote(remote_name: &str) -> bool {
    let config = ConfigManager::load_config();

    !config
        .sync
        .skip_push_remotes
        .iter()
        .any(|r| r == remote_name)
}

fn skip_plan(msg: &str) -> Result<ExecutionPlan> {
    let mut plan = ExecutionPlan::new();
    plan.add(MessageOperation::Skip {
        msg: msg.to_string(),
    });
    Ok(plan)
}

fn find_git_repository_upwards(start_dir: &Path) -> Option<std::path::PathBuf> {
    let mut current = start_dir.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}
