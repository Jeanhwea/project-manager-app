//! `pma status`：打印单个仓库的当前分支、工作区是否干净、remote 列表与最新标签。

use crate::commands::MultiRepo;
use crate::commands::RepoPathArgs;
use crate::domain::git::collect_context;
use crate::engine::plan;
use crate::error::Result;
use crate::model::git::GitContext;
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult};
use std::path::Path;

/// `pma status` 的命令行参数
#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    #[command(flatten)]
    pub repo_path: RepoPathArgs,
}

/// 一个仓库的只读状态快照
#[derive(Debug)]
pub(crate) struct StatusContext {
    git_ctx: GitContext,
}

impl MultiRepo for StatusArgs {
    type Context = StatusContext;
    type Plan = ExecutionPlan;

    fn collect(&self, repo_path: &Path) -> Result<StatusContext> {
        let git_ctx = collect_context(repo_path)?;
        Ok(StatusContext { git_ctx })
    }

    fn plan(&self, ctx: &StatusContext, repo_path: &Path) -> Result<ExecutionPlan> {
        // status 只展示上下文里已有的信息，路径本身用不到
        let _ = repo_path;
        let mut plan = ExecutionPlan::new();

        plan.add_message(DisplayMessage::Item {
            label: "分支".to_string(),
            value: ctx.git_ctx.current_branch.clone(),
        });

        if ctx.git_ctx.has_uncommitted_changes {
            plan.add_message(DisplayMessage::Warning {
                msg: "有未提交的变更".to_string(),
            });
        } else {
            plan.add_message(DisplayMessage::Success {
                msg: "工作区干净".to_string(),
            });
        }

        if !ctx.git_ctx.remotes.is_empty() {
            for remote in &ctx.git_ctx.remotes {
                plan.add_message(DisplayMessage::Detail {
                    label: remote.name.clone(),
                    value: remote.url.clone(),
                });
            }
        }

        if !ctx.git_ctx.tags.is_empty() {
            // collect_context 按 creatordate 倒序取 tag，第一个即最新标签
            let latest_tag = ctx.git_ctx.tags[0].name.clone();
            plan.add_message(DisplayMessage::Item {
                label: "最新标签".to_string(),
                value: latest_tag,
            });
        }

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

/// `pma status` 入口；按仓库串行执行
pub fn run(args: StatusArgs) -> Result<()> {
    crate::commands::run_multi_repo_cmd(&args, &args.repo_path)
}
