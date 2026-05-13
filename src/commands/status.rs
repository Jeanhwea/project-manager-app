use crate::commands::RepoPathArgs;
use crate::control::command::MultiRepoCommand;
use crate::domain::git::collect_context;
use crate::error::Result;
use crate::model::git::GitContext;
use crate::model::plan::{ExecutionPlan, MessageOperation};
use std::path::Path;

#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    #[command(flatten)]
    pub repo_path: RepoPathArgs,
}

#[derive(Debug)]
pub(crate) struct StatusContext {
    git_ctx: GitContext,
}

impl MultiRepoCommand for StatusArgs {
    type Context = StatusContext;

    fn context(&self, repo_path: &Path) -> Result<StatusContext> {
        let git_ctx = collect_context(repo_path)?;
        Ok(StatusContext { git_ctx })
    }

    fn plan(&self, ctx: &StatusContext, repo_path: &Path) -> Result<ExecutionPlan> {
        let _ = repo_path;
        let mut plan = ExecutionPlan::new();

        plan.add(MessageOperation::Item {
            label: "分支".to_string(),
            value: ctx.git_ctx.current_branch.clone(),
        });

        if ctx.git_ctx.has_uncommitted_changes {
            plan.add(MessageOperation::Warning {
                msg: "有未提交的变更".to_string(),
            });
        } else {
            plan.add(MessageOperation::Success {
                msg: "工作区干净".to_string(),
            });
        }

        if !ctx.git_ctx.remotes.is_empty() {
            for remote in &ctx.git_ctx.remotes {
                plan.add(MessageOperation::Detail {
                    label: remote.name.clone(),
                    value: remote.url.clone(),
                });
            }
        }

        if !ctx.git_ctx.tags.is_empty() {
            let latest_tag = ctx.git_ctx.tags[0].name.clone();
            plan.add(MessageOperation::Item {
                label: "最新标签".to_string(),
                value: latest_tag,
            });
        }

        Ok(plan)
    }
}

pub fn run(args: StatusArgs) -> Result<()> {
    crate::commands::run_multi_repo(&args, &args.repo_path)
}
