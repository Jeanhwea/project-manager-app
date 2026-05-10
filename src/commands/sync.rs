use crate::control::context::collect_context;
use crate::control::plan::run_plan;
use crate::domain::AppError;
use crate::domain::git::repository::RepoWalker;
use crate::model::plan::{ExecutionPlan, GitOperation};
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

pub fn run(args: SyncArgs) -> anyhow::Result<()> {
    let effective_path = if args.path.is_empty() {
        let cwd = std::env::current_dir()?;
        find_git_repository_upwards(&cwd).unwrap_or(cwd)
    } else {
        crate::utils::path::canonicalize_path(&args.path)?
    };

    let walker = RepoWalker::new(&effective_path, args.max_depth.unwrap_or(3))?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    let total = walker.total();

    for (index, repo_info) in walker.repositories().iter().enumerate() {
        let repo_path = &repo_info.path;
        Output::repo_header(index + 1, total, repo_path);

        if let Err(e) = sync_repo(repo_path, &args) {
            Output::error(&format!("同步失败: {}", e));
        }
    }

    Ok(())
}

fn sync_repo(repo_path: &Path, args: &SyncArgs) -> anyhow::Result<()> {
    let ctx = collect_context(repo_path)?;
    if ctx.remotes.is_empty() {
        return Ok(());
    }

    let target_remote = match args.remote.as_deref() {
        Some(name) => {
            if !ctx.has_remote(name) {
                return Err(AppError::not_found(format!("远程仓库 {} 不存在", name)).into());
            }
            name.to_string()
        }
        None => ctx
            .remotes
            .first()
            .expect("remotes should not be empty")
            .name
            .clone(),
    };

    let mut plan = ExecutionPlan::new().with_dry_run(args.dry_run);
    plan.add(GitOperation::Pull {
        remote: target_remote.clone(),
        branch: ctx.current_branch.clone(),
    });
    plan.add(GitOperation::PushAll {
        remote: target_remote.clone(),
    });
    plan.add(GitOperation::PushTags {
        remote: target_remote,
    });

    run_plan(&plan)
}

fn find_git_repository_upwards(start_dir: &Path) -> Option<std::path::PathBuf> {
    let mut current = start_dir.to_path_buf();
    loop {
        let git_dir = current.join(".git");
        if git_dir.exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}
