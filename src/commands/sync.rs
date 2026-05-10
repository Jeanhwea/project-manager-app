use crate::domain::git::command::GitCommandRunner;
use crate::domain::git::repository::RepoWalker;
use crate::utils::output::Output;
use anyhow::Result;
use std::path::Path;

#[derive(Debug, clap::Args)]
pub struct SyncArgs {
    #[arg(long, short, default_value = "3")]
    pub max_depth: Option<usize>,
    #[arg(
        default_value = "",
        help = "Path to search, defaults to current directory"
    )]
    pub path: String,
    #[arg(long, short)]
    pub remote: Option<String>,
    #[arg(long, short, default_value = "false")]
    pub fetch: bool,
    #[arg(long, default_value = "false")]
    pub dry_run: bool,
    #[arg(long, default_value = "false")]
    pub prune: bool,
}

pub fn run(args: SyncArgs) -> Result<()> {
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

    let runner = GitCommandRunner::new();
    let total = walker.total();

    for (index, repo_info) in walker.repositories().iter().enumerate() {
        let repo_path = &repo_info.path;
        Output::repo_header(index + 1, total, repo_path);

        if let Err(e) = sync_repo(repo_path, &runner, &args) {
            Output::error(&format!("同步失败: {}", e));
        }
    }

    Ok(())
}

fn sync_repo(repo_path: &Path, runner: &GitCommandRunner, args: &SyncArgs) -> Result<()> {
    let remotes = runner.get_remote_list(repo_path)?;
    if remotes.is_empty() {
        Output::warning("没有配置远程仓库");
        return Ok(());
    }

    let target_remote = args.remote.as_deref().unwrap_or("origin");

    if !remotes.iter().any(|r| r == target_remote) {
        Output::warning(&format!("远程仓库 {} 不存在", target_remote));
        return Ok(());
    }

    if args.fetch {
        let fetch_args = if args.prune {
            vec!["fetch", target_remote, "--prune"]
        } else {
            vec!["fetch", target_remote]
        };

        if args.dry_run {
            Output::skip(&format!("git {}", fetch_args.join(" ")));
        } else {
            runner.execute_streaming(&fetch_args, repo_path)?;
        }
    } else {
        let current_branch = runner.get_current_branch(repo_path)?;

        if args.dry_run {
            Output::skip(&format!("git pull {} {}", target_remote, current_branch));
        } else {
            runner.execute_streaming(&["pull", target_remote, &current_branch], repo_path)?;
        }
    }

    Ok(())
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
