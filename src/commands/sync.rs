use super::{Command, CommandResult};
use crate::domain::context::AppContext;
use crate::domain::git::remote::RemoteManager;
use crate::domain::git::repository::{RepoWalker, find_git_repository_upwards};
use crate::domain::runner::DryRunContext;
use crate::utils::error::ErrorHandler;
use crate::utils::output::Output;
use crate::utils::path::format_path;
use std::path::{Path, PathBuf};

/// Sync command arguments
#[derive(Debug, clap::Args)]
pub struct SyncArgs {
    /// Maximum depth to search for repositories
    #[arg(
        long,
        short,
        default_value = "3",
        help = "Maximum depth to search for repositories"
    )]
    pub max_depth: Option<usize>,
    /// Remotes to skip
    #[arg(long, short, help = "Remotes to skip")]
    pub skip_remotes: Vec<String>,
    /// Whether to pull all local branches
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Whether to pull all local branches"
    )]
    pub all_branch: bool,
    /// Path to the directory to search for repositories, defaults to current directory
    #[arg(
        default_value = "",
        help = "Path to the directory to search for repositories, defaults to current directory"
    )]
    pub path: String,
    /// Dry run: show what would be changed without making any modifications
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be changed without making any modifications"
    )]
    pub dry_run: bool,
    /// Only fetch from remotes, do not pull or push
    #[arg(
        long,
        short = 'f',
        default_value = "false",
        help = "Only fetch from remotes, do not pull or push"
    )]
    pub fetch_only: bool,
    /// Use rebase instead of merge when pulling
    #[arg(
        long,
        default_value = "false",
        help = "Use rebase instead of merge when pulling"
    )]
    pub rebase: bool,
}

/// Sync command
pub struct SyncCommand;

impl Command for SyncCommand {
    type Args = SyncArgs;

    fn execute(args: Self::Args) -> CommandResult {
        execute_sync(args)
    }
}

fn execute_sync(args: SyncArgs) -> CommandResult {
    let search_path = if args.path.is_empty() || args.path == "." {
        std::env::current_dir()?
    } else {
        PathBuf::from(&args.path)
    };

    let effective_path =
        find_git_repository_upwards(&search_path).unwrap_or_else(|| search_path.clone());

    let walker = RepoWalker::new(&effective_path, args.max_depth.unwrap_or(3))?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    if !args.skip_remotes.is_empty() {
        Output::info(&format!("跳过远程仓库: {:?}", args.skip_remotes));
    }

    let ctx = DryRunContext::new(args.dry_run);
    ctx.print_header("[DRY-RUN] 将要同步的仓库:");

    walker.walk(|repo_path, _index, _total| {
        do_info_repository(repo_path)?;

        if !ctx.is_dry_run() && !is_workdir_clean(repo_path)? {
            let runner = AppContext::global().git_runner();
            runner.execute_with_success_in_dir(&["status"], repo_path)?;
            Output::warning(&format!(
                "无法同步不干净工作目录: {}",
                format_path(repo_path)
            ));
            return Ok(());
        }

        do_sync_repository(
            &ctx,
            repo_path,
            args.all_branch,
            &args.skip_remotes,
            args.fetch_only,
            args.rebase,
        );

        Ok(())
    })?;

    Ok(())
}

fn do_info_repository(repo_path: &Path) -> Result<(), crate::domain::git::GitError> {
    let runner = AppContext::global().git_runner();

    if let Err(e) = runner.execute_with_success_in_dir(&["branch", "--list"], repo_path) {
        ErrorHandler::print_error("无法获取分支信息", &e);
    }

    if let Err(e) = runner.execute_with_success_in_dir(&["remote", "-v"], repo_path) {
        ErrorHandler::print_error("无法获取远程信息", &e);
    }

    Ok(())
}

fn get_tracking_remote_info(
    repo_path: &Path,
    remotes: &[(String, String)],
) -> Option<(String, String)> {
    let runner = AppContext::global().git_runner();
    let output = runner
        .execute_quiet_in_dir(&["rev-parse", "--abbrev-ref", "HEAD@{upstream}"], repo_path)
        .ok()?;

    let upstream = String::from_utf8(output.stdout).ok()?;
    let (remote, _) = upstream.trim().split_once('/')?;
    let (_, url) = remotes.iter().find(|(r, _)| r == remote)?;

    Some((remote.to_string(), url.clone()))
}

/// Perform sync operations on a repository
fn do_sync_repository(
    ctx: &DryRunContext,
    repo_path: &Path,
    all_branch: bool,
    skip_remotes: &[String],
    fetch_only: bool,
    rebase: bool,
) {
    let remote_manager = RemoteManager::new();
    let remotes = match remote_manager.list_remotes(repo_path) {
        Ok(remotes) => remotes
            .into_iter()
            .map(|r| (r.name, r.url))
            .collect::<Vec<_>>(),
        Err(_) => {
            ctx.print_message("无法获取远程仓库信息");
            return;
        }
    };

    if remotes.is_empty() {
        ctx.print_message("无远程仓库");
        return;
    }

    if fetch_only {
        // fetch_only 模式：只需要 fetch 所有远程
        for (remote, url) in &remotes {
            if skip_remotes.iter().any(|s| s.as_str() == *remote) {
                Output::skip(&format!("git fetch {} ({})", remote, url));
            } else {
                ctx.run_in_dir("git", &["fetch", remote], Some(repo_path))
                    .unwrap_or_else(|e| ErrorHandler::print_error_anyhow("拉取仓库失败", &e));
            }
        }
        return;
    }

    if all_branch {
        // all_branch 模式：切换到每个本地分支并 pull
        if ctx.is_dry_run() {
            if rebase {
                ctx.print_message("git pull --rebase (all branches)");
            } else {
                ctx.print_message("git pull (all branches)");
            }
        } else {
            do_pull_all_local_branch(repo_path, rebase);
        }
    } else {
        // 普通模式：只 pull 当前分支
        let Some((track_remote, track_remote_url)) =
            get_tracking_remote_info(repo_path, &remotes)
        else {
            ctx.print_message("无跟踪分支信息，跳过 pull");
            return;
        };

        if skip_remotes.contains(&track_remote) {
            Output::skip(&format!("git pull {} ({})", track_remote, track_remote_url));
        } else {
            let args = if rebase {
                vec!["pull", "--rebase"]
            } else {
                vec!["pull"]
            };
            ctx.run_in_dir("git", &args, Some(repo_path))
                .unwrap_or_else(|e| ErrorHandler::print_error_anyhow("同步仓库失败", &e));
        }
    }

    for (remote, url) in remotes {
        if should_skip_push(&remote, &url, skip_remotes) {
            Output::skip(&format!("git push {} ({})", remote, url));
            continue;
        }
        ctx.run_in_dir("git", &["push", &remote, "--all"], Some(repo_path))
            .unwrap_or_else(|e| ErrorHandler::print_error_anyhow("推送分支失败", &e));
        ctx.run_in_dir("git", &["push", &remote, "--tags"], Some(repo_path))
            .unwrap_or_else(|e| ErrorHandler::print_error_anyhow("推送标签失败", &e));
    }
}

fn should_skip_push(remote: &str, url: &str, skip_remotes: &[String]) -> bool {
    if skip_remotes.iter().any(|s| s.as_str() == remote) {
        return true;
    }

    let config = AppContext::global().config();

    if let Some((protocol, host, path)) = parse_git_remote_url(url) {
        use crate::domain::git::GitProtocol;

        if protocol == GitProtocol::Https
            && config.sync.skip_push_hosts.iter().any(|h| h == &host)
        {
            return true;
        }
        if protocol == GitProtocol::Ssh && host == "gitee.com" && path.starts_with("red_base") {
            return true;
        }
    } else {
        Output::warning(&format!("未知协议或主机: {}", url));
    }

    false
}

/// Parse Git remote URL into protocol, host, and path
fn parse_git_remote_url(url: &str) -> Option<(crate::domain::git::GitProtocol, String, String)> {
    use crate::domain::git::remote::Remote;

    let protocol = Remote::parse_url(url).ok()?;
    let (host, path) = Remote::extract_host_and_path(url)?;

    Some((protocol, host, path))
}

fn list_local_branches(repo_path: &Path) -> Option<(String, Vec<String>)> {
    let runner = AppContext::global().git_runner();
    let output = runner
        .execute_quiet_in_dir(&["branch", "--list"], repo_path)
        .ok()?;
    let stdout = String::from_utf8(output.stdout).ok()?;
    let lines: Vec<_> = stdout.lines().collect();

    let current_branch = lines
        .iter()
        .find(|line| line.starts_with("* "))?
        .trim_start_matches("* ")
        .to_string();

    let local_branches = lines
        .iter()
        .filter(|line| !line.starts_with("*"))
        .map(|line| line.trim().to_string())
        .collect();

    Some((current_branch, local_branches))
}

/// Pull all local branches
fn do_pull_all_local_branch(repo_path: &Path, rebase: bool) {
    let Some((current_branch, local_branches)) = list_local_branches(repo_path) else {
        return;
    };

    if local_branches.is_empty() {
        do_pull_repository(repo_path, rebase);
        return;
    }

    for branch in &local_branches {
        do_pull_repository_branch(branch, repo_path, rebase);
    }
    do_pull_repository_branch(&current_branch, repo_path, rebase);
}

fn do_pull_repository_branch(branch: &str, repo_path: &Path, rebase: bool) {
    let runner = AppContext::global().git_runner();
    if let Err(e) = runner.execute_with_success_in_dir(&["checkout", branch], repo_path) {
        let context = format!("切换分支失败: {}", format_path(repo_path));
        ErrorHandler::print_error(&context, &e);
        return;
    }
    do_pull_repository(repo_path, rebase);
}

fn do_pull_repository(repo_path: &Path, rebase: bool) {
    let args = if rebase {
        vec!["pull", "--rebase"]
    } else {
        vec!["pull"]
    };
    let runner = AppContext::global().git_runner();
    if let Err(e) = runner.execute_with_success_in_dir(&args, repo_path) {
        let context = format!("同步仓库失败: {}", format_path(repo_path));
        ErrorHandler::print_error(&context, &e);
    }
}

fn is_workdir_clean(repo_path: &Path) -> Result<bool, crate::domain::git::GitError> {
    let runner = AppContext::global().git_runner();
    let output = runner.execute_in_dir(&["status", "--porcelain"], repo_path)?;
    Ok(output.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_dry_run_context() {
        let ctx = DryRunContext::new(true);
        assert!(ctx.is_dry_run());

        let ctx = DryRunContext::new(false);
        assert!(!ctx.is_dry_run());
    }

    #[test]
    fn test_sync_args_structure() {
        let args = SyncArgs {
            max_depth: Some(3),
            skip_remotes: vec!["origin".to_string()],
            all_branch: true,
            path: ".".to_string(),
            dry_run: true,
            fetch_only: false,
            rebase: true,
        };

        assert_eq!(args.max_depth, Some(3));
        assert_eq!(args.skip_remotes, vec!["origin"]);
        assert!(args.all_branch);
        assert_eq!(args.path, ".");
        assert!(args.dry_run);
        assert!(!args.fetch_only);
        assert!(args.rebase);
    }

    #[test]
    fn test_parse_git_remote_url() {
        let result = parse_git_remote_url("git@github.com:user/repo.git");
        assert!(result.is_some());
        if let Some((protocol, host, path)) = result {
            use crate::domain::git::GitProtocol;
            assert_eq!(protocol, GitProtocol::Ssh);
            assert_eq!(host, "github.com");
            assert_eq!(path, "user/repo.git");
        }

        let result = parse_git_remote_url("https://github.com/user/repo.git");
        assert!(result.is_some());
        if let Some((protocol, host, path)) = result {
            use crate::domain::git::GitProtocol;
            assert_eq!(protocol, GitProtocol::Https);
            assert_eq!(host, "github.com");
            assert_eq!(path, "user/repo.git");
        }

        let result = parse_git_remote_url("invalid-url");
        assert!(result.is_none());
    }

    #[test]
    fn test_should_skip_push() {
        assert!(should_skip_push(
            "origin",
            "https://example.com/repo.git",
            &["origin".to_string()]
        ));

        assert!(!should_skip_push(
            "origin",
            "https://example.com/repo.git",
            &["upstream".to_string()]
        ));
    }

    #[test]
    fn test_is_workdir_clean() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        let _ = std::process::Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output();

        let _result = is_workdir_clean(repo_path);
    }
}
