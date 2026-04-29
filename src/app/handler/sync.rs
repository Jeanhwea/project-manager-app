use crate::app::common::config;
use crate::app::common::git::{self, GitProtocol, RepoWalker};
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
    fetch_only: bool,
    rebase: bool,
) -> Result<()> {
    let walker = RepoWalker::new(path, max_depth)?;
    if walker.is_empty() {
        return Ok(());
    }

    if !skip_remotes.is_empty() {
        println!("跳过远程仓库: {:?}", skip_remotes);
    }

    let ctx = DryRunContext::new(dry_run);
    ctx.print_header("[DRY-RUN] 将要同步的仓库:");

    walker.walk(|entry| {
        let repo_path = entry.path;

        do_info_repository(repo_path);

        if !ctx.is_dry_run() && !git::is_workdir_clean(repo_path) {
            CommandRunner::run_with_success_in_dir("git", &["status"], repo_path)?;
            println!(
                "  无法同步不干净工作目录: {}",
                utils::format_path(repo_path).red()
            );
            return Ok(());
        }

        do_sync_repository(
            &ctx,
            repo_path,
            all_branch,
            &skip_remotes,
            fetch_only,
            rebase,
        );

        Ok(())
    })
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
    fetch_only: bool,
    rebase: bool,
) {
    let remotes = git::get_remote_info(repo_path);
    if remotes.is_empty() {
        ctx.print_message("无远程仓库");
        return;
    }

    let Some((track_remote, track_remote_url)) = get_tracking_remote_info(repo_path, &remotes)
    else {
        ctx.print_message("无跟踪分支信息");
        return;
    };

    if skip_remotes.contains(&track_remote) {
        println!(
            "  {} git pull {} ({})",
            "[SKIP]".dimmed(),
            track_remote,
            track_remote_url.green()
        );
    } else if fetch_only {
        ctx.run_in_dir("git", &["fetch", &track_remote], Some(repo_path))
            .unwrap_or_else(|e| println!("  拉取仓库失败: {}", e));
    } else if all_branch {
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
        let args = if rebase {
            vec!["pull", "--rebase"]
        } else {
            vec!["pull"]
        };
        ctx.run_in_dir("git", &args, Some(repo_path))
            .unwrap_or_else(|e| println!("  同步仓库失败: {}", e));
    }

    if fetch_only {
        for (remote, url) in &remotes {
            if skip_remotes.iter().any(|s| s.as_str() == *remote) {
                println!(
                    "  {} git fetch {} ({})",
                    "[SKIP]".dimmed(),
                    remote,
                    url.green()
                );
            } else if *remote != track_remote {
                ctx.run_in_dir("git", &["fetch", remote], Some(repo_path))
                    .unwrap_or_else(|e| println!("  拉取仓库失败: {}", e));
            }
        }
        return;
    }

    for (remote, url) in remotes {
        if should_skip_push(&remote, &url, skip_remotes) {
            println!(
                "  {} git push {} ({})",
                "[SKIP]".dimmed(),
                remote,
                url.green()
            );
            continue;
        }
        ctx.run_in_dir("git", &["push", &remote, "--all"], Some(repo_path))
            .unwrap_or_else(|e| println!("  推送分支失败: {}", e));
        ctx.run_in_dir("git", &["push", &remote, "--tags"], Some(repo_path))
            .unwrap_or_else(|e| println!("  推送标签失败: {}", e));
    }
}

fn should_skip_push(remote: &str, url: &str, skip_remotes: &[String]) -> bool {
    if skip_remotes.iter().any(|s| s.as_str() == remote) {
        return true;
    }
    let cfg = config::load();
    if let Some((protocol, host, path)) = git::parse_git_remote_url(url) {
        if protocol == GitProtocol::Https && cfg.sync.skip_push_hosts.iter().any(|h| h == &host) {
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
    do_pull_repository(repo_path, rebase);
}

fn do_pull_repository(repo_path: &Path, rebase: bool) {
    let args = if rebase {
        vec!["pull", "--rebase"]
    } else {
        vec!["pull"]
    };
    if let Err(e) = CommandRunner::run_with_success_in_dir("git", &args, repo_path) {
        println!(
            "  同步仓库失败: {} - {}",
            utils::format_path(repo_path).red(),
            e
        );
    }
}
