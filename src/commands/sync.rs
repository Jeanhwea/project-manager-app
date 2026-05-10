use crate::control::context::collect_context;
use crate::control::pipeline::Pipeline;
use crate::domain::AppError;
use crate::domain::git::repository::RepoWalker;
use crate::model::git::GitContext;
use crate::model::plan::{ExecutionPlan, GitOperation, MessageOperation};
use crate::utils::output::Output;
use std::path::Path;

#[derive(Debug, clap::Args)]
pub struct SyncArgs {
    #[arg(
        long,
        short,
        default_value = "3",
        help = "Maximum depth to search for repositories"
    )]
    pub max_depth: Option<usize>,
    #[arg(
        default_value = "",
        help = "Path to search, defaults to current directory"
    )]
    pub path: String,
    #[arg(long, short, help = "Target remote name (e.g. origin, upstream)")]
    pub remote: Option<String>,
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show commands without executing"
    )]
    pub dry_run: bool,
}

struct SyncContext {
    git_ctx: GitContext,
    target_remote: String,
}

pub fn run(args: SyncArgs) -> anyhow::Result<()> {
    let effective_path = resolve_effective_path(&args.path)?;
    let walker = RepoWalker::new(&effective_path, args.max_depth.unwrap_or(3))?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    Pipeline::run_multi_repo(&args, &walker, get_context, make_plan)
}

fn resolve_effective_path(path: &str) -> anyhow::Result<std::path::PathBuf> {
    if path.is_empty() {
        let cwd = std::env::current_dir()?;
        Ok(find_git_repository_upwards(&cwd).unwrap_or(cwd))
    } else {
        Ok(crate::utils::path::canonicalize_path(path)?)
    }
}

fn get_context(args: &SyncArgs, repo_path: &Path) -> anyhow::Result<SyncContext> {
    let git_ctx = collect_context(repo_path)?;

    if git_ctx.remotes.is_empty() {
        return Ok(SyncContext {
            git_ctx,
            target_remote: String::new(),
        });
    }

    let target_remote = resolve_target_remote(&git_ctx, args.remote.as_deref())?;

    Ok(SyncContext {
        git_ctx,
        target_remote,
    })
}

fn resolve_target_remote(
    git_ctx: &GitContext,
    explicit_remote: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(name) = explicit_remote {
        if !git_ctx.has_remote(name) {
            return Err(AppError::not_found(format!("远程仓库 {} 不存在", name)).into());
        }
        return Ok(name.to_string());
    }

    git_ctx
        .preferred_remote()
        .or_else(|| git_ctx.first_remote_name())
        .ok_or_else(|| AppError::not_found("无可用远程仓库").into())
}

fn make_plan(args: &SyncArgs, ctx: &SyncContext) -> anyhow::Result<ExecutionPlan> {
    if ctx.git_ctx.remotes.is_empty() || ctx.target_remote.is_empty() {
        return skip_plan("无远程仓库");
    }

    let remote = ctx.target_remote.clone();
    let branch = ctx.git_ctx.current_branch.clone();
    let mut plan = ExecutionPlan::new().with_dry_run(args.dry_run);

    plan.add(GitOperation::Pull { remote: remote.clone(), branch });
    plan.add(GitOperation::PushAll { remote: remote.clone() });
    plan.add(GitOperation::PushTags { remote });

    Ok(plan)
}

fn skip_plan(msg: &str) -> anyhow::Result<ExecutionPlan> {
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
