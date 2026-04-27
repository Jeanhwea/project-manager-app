use crate::app::common::git::{self, GitProtocol, RepoType};
use crate::app::common::runner::{CommandRunner, DryRunContext};
use crate::utils;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

pub fn execute(
    path: &str,
    max_depth: Option<usize>,
    all_branch: bool,
    skip_remotes: Vec<String>,
    dry_run: bool,
) -> Result<()> {
    let root_dir = Path::new(path);

    if !root_dir.exists() {
        anyhow::bail!("目录不存在: {}", path);
    }

    let git_repos =
        crate::app::common::git::find_git_repositories_or_current(root_dir, max_depth);

    if git_repos.is_empty() {
        println!("未找到git仓库");
        return Ok(());
    }

    if !skip_remotes.is_empty() {
        println!("跳过远程仓库: {:?}", skip_remotes);
    }

    let total = git_repos.len();
    let ctx = DryRunContext::new(dry_run);

    if ctx.is_dry_run() {
        ctx.print_header("[DRY-RUN] 将要同步的仓库:");
    }

    for (index, repo) in git_repos.iter().enumerate() {
        let repo_path = utils::canonicalize_path(&repo.path)
            .unwrap_or_else(|_| repo.path.clone());

        let progress = format!("({}/{})", index + 1, total);
        println!(
            "{}>> {}",
            progress.white().bold(),
            utils::format_path(&repo_path).cyan().underline(),
        );

        if repo.repo_type == RepoType::Submodule {
            continue;
        }

        do_info_repository(&repo_path);

        if !ctx.is_dry_run() && !is_workdir_clean(&repo_path) {
            CommandRunner::run_with_success_in_dir("git", &["status"], &repo_path)?;
            println!(
                "  无法同步不干净工作目录: {}",
                utils::format_path(&repo_path).red()
            );
            continue;
        }

        do_sync_repository(&ctx, &repo_path, all_branch, &skip_remotes);
    }

    Ok(())
}

fn is_workdir_clean(repo_path: &Path) -> bool {
    CommandRunner::run_quiet_in_dir("git", &["status", "--porcelain"], repo_path)
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .is_some_and(|s| s.is_empty())
}

fn do_info_repository(repo_path: &Path) {
    if let Err(e) =
        CommandRunner::run_with_success_in_dir("git", &["branch", "--list"], repo_path)
    {
        println!("  无法获取分支信息: {}", e);
    }
    if let Err(e) = CommandRunner::run_with_success_in_dir("git", &["remote", "-v"], repo_path) {
        println!("  无法获取远程信息: {}", e);
    }
}

fn get_tracking_remote_info(
    repo_path: &Path,
    remotes: &[(String, String)],
) -> Option<(String, String)> {
    let output = CommandRunner::run_quiet_in_dir(
        "git",
        &["rev-parse", "--abbrev-ref", "HEAD@{upstream}"],
        repo_path,
    )
    .ok()?;

    let upstream = String::from_utf8(output.stdout).ok()?;
    let (remote, _) = upstream.trim().split_once('/')?;
    let (_, url) = remotes.iter().find(|(r, _)| r == remote)?;

    Some((remote.to_string(), url.clone()))
}

fn do_sync_repository(
    ctx: &DryRunContext,
    repo_path: &Path,
    all_branch: bool,
    skip_remotes: &[String],
) {
    let remotes = git::get_remote_info(repo_path);
    if remotes.is_empty() {
        if ctx.is_dry_run() {
            println!("  {}", "无远程仓库".yellow());
        }
        return;
    }

    let Some((track_remote, track_remote_url)) = get_tracking_remote_info(repo_path, &remotes)
    else {
        if ctx.is_dry_run() {
            println!("  {}", "无跟踪分支信息".yellow());
        }
        return;
    };

    if skip_remotes.contains(&track_remote) {
        println!(
            "  {} pull {} ({})",
            "[SKIP]".dimmed(),
            track_remote,
            track_remote_url.green()
        );
    } else if all_branch {
        if ctx.is_dry_run() {
            println!("  {} pull (all branches)", "[DRY-RUN]".yellow());
        } else {
            do_pull_all_local_branch(repo_path);
        }
    } else {
        if ctx.is_dry_run() {
            println!("  {} pull", "[DRY-RUN]".yellow());
        } else {
            do_pull_repository(repo_path);
        }
    }

    for (remote, url) in remotes {
        if should_skip_push(&remote, &url, skip_remotes) {
            println!("  {} push {} ({})", "[SKIP]".dimmed(), remote, url.green());
            continue;
        }
        if ctx.is_dry_run() {
            println!("  {} push {} --all", "[DRY-RUN]".yellow(), remote);
            println!("  {} push {} --tags", "[DRY-RUN]".yellow(), remote);
        } else {
            do_push_repository(repo_path, &remote);
        }
    }
}

fn should_skip_push(remote: &str, url: &str, skip_remotes: &[String]) -> bool {
    if skip_remotes.iter().any(|s| s.as_str() == remote) {
        return true;
    }
    if let Some((protocol, host, path)) = git::parse_git_remote_url(url) {
        if protocol == GitProtocol::Https
            && (host == "github.com" || host == "githubfast.com" || host == "gitee.com")
        {
            return true;
        }
        if protocol == GitProtocol::Ssh && host == "gitee.com" && path.starts_with("red_base") {
            return true;
        }
    } else {
        println!("  未知协议或主机: {}", url);
    }
    false
}

fn list_local_branches(repo_path: &Path) -> Option<(String, Vec<String>)> {
    let output = CommandRunner::run_quiet_in_dir("git", &["branch", "--list"], repo_path).ok()?;
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

fn do_pull_all_local_branch(repo_path: &Path) {
    let Some((current_branch, local_branches)) = list_local_branches(repo_path) else {
        return;
    };

    if local_branches.is_empty() {
        do_pull_repository(repo_path);
        return;
    }

    for branch in &local_branches {
        do_pull_repository_branch(branch, repo_path);
    }
    do_pull_repository_branch(&current_branch, repo_path);
}

fn do_pull_repository_branch(branch: &str, repo_path: &Path) {
    if let Err(e) =
        CommandRunner::run_with_success_in_dir("git", &["checkout", branch], repo_path)
    {
        println!(
            "  切换分支失败: {} - {}",
            utils::format_path(repo_path).red(),
            e
        );
        return;
    }
    do_pull_repository(repo_path);
}

fn do_pull_repository(repo_path: &Path) {
    if let Err(e) = CommandRunner::run_with_success_in_dir("git", &["pull"], repo_path) {
        println!(
            "  同步仓库失败: {} - {}",
            utils::format_path(repo_path).red(),
            e
        );
    }
}

fn do_push_repository(repo_path: &Path, remote: &str) {
    if let Err(e) =
        CommandRunner::run_with_success_in_dir("git", &["push", remote, "--all"], repo_path)
    {
        println!(
            "  推送分支失败: {} - {}",
            utils::format_path(repo_path).red(),
            e
        );
    }
    if let Err(e) =
        CommandRunner::run_with_success_in_dir("git", &["push", remote, "--tags"], repo_path)
    {
        println!(
            "  推送标签失败: {} - {}",
            utils::format_path(repo_path).red(),
            e
        );
    }
}
