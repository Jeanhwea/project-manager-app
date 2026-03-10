use super::git;
use super::runner::CommandRunner;
use crate::utils;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use super::repo::RepoType;

pub fn execute(path: &str, max_depth: Option<usize>, skip_remotes: Vec<String>) -> Result<()> {
    let root_dir = Path::new(path);

    if !root_dir.exists() {
        anyhow::bail!("目录不存在: {}", path);
    }

    let git_repos = super::repo::find_git_repositories(root_dir, max_depth);

    if git_repos.is_empty() {
        println!("未找到git仓库");
        return Ok(());
    }

    if !skip_remotes.is_empty() {
        println!("跳过远程仓库: {:?}", skip_remotes);
    }

    let total_repos = git_repos.len();

    for (repo_index, repo_info) in git_repos.iter().enumerate() {
        let repo_path = if let Ok(abs_path) = repo_info.path.canonicalize() {
            abs_path
        } else {
            repo_info.path.clone()
        };

        let progress = format!("({}/{})", repo_index + 1, total_repos);
        println!(
            "{}>> {}",
            progress.white().bold(),
            utils::format_path(&repo_path).cyan().underline(),
        );

        // 只对普通 git 仓库执行 git pull，跳过子模块
        if repo_info.repo_type == RepoType::Submodule {
            continue;
        }

        // 打印仓库信息
        do_info_repository(&repo_path);

        // 同步仓库
        do_sync_repository(&repo_path, skip_remotes.clone());
    }

    Ok(())
}

fn do_info_repository(repo_path: &Path) {
    // 打印本地分支
    if let Err(e) =
        CommandRunner::run_with_success_in_dir("git", &["branch", "--list"], repo_path)
    {
        println!("  无法获取分支信息: {}", e);
    }

    // 打印远程仓库信息
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

fn do_sync_repository(repo_path: &Path, skip_remotes: Vec<String>) {
    let remotes = git::get_remote_info(repo_path);
    if remotes.is_empty() {
        return;
    }

    // 获取当前分支的跟踪远程仓库
    let (track_remote, track_remote_url) = match get_tracking_remote_info(repo_path, &remotes) {
        Some(info) => info,
        None => return,
    };

    // 拉取远端数据
    if !skip_remotes.contains(&track_remote) {
        do_pull_all_local_branch(repo_path);
    } else {
        println!("  跳过拉取 {} ({})", track_remote, track_remote_url.green());
        return;
    }

    // 对每个远程仓库执行 git push
    for (remote, url) in remotes {
        // 检测是否跳过推送
        if should_skip_push(&remote, &url, &skip_remotes) {
            println!("  跳过推送 {} ({})", remote, url.green());
            continue;
        }

        // 推送所有分支和标签
        do_push_repository(repo_path, &remote);
    }
}

fn should_skip_push(remote: &str, url: &str, skip_remotes: &[String]) -> bool {
    // 检查是否在跳过列表中
    if skip_remotes.iter().any(|s| s.as_str() == remote) {
        return true;
    }
    if let Some((protocol, host, path)) = git::parse_git_remote_url(url) {
        // println!("  解析远程URL: {} {} {}", protocol, host, path);
        if protocol == "https" && (host == "github.com" || host == "githubfast.com") {
            return true;
        }
        if protocol == "git" && host == "gitee.com" && path.starts_with("red_base") {
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
    let (current_branch, local_branches) = match list_local_branches(repo_path) {
        Some(result) => result,
        None => return,
    };
    for branch in local_branches {
        do_pull_repository(&branch, repo_path);
    }

    do_pull_repository(&current_branch, repo_path);
}

fn do_pull_repository(branch: &str, repo_path: &Path) {
    // git checkout branch
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

    // git pull
    if let Err(e) = CommandRunner::run_with_success_in_dir("git", &["pull"], repo_path) {
        println!(
            "  同步仓库失败: {} - {}",
            utils::format_path(repo_path).red(),
            e
        );
    }
}

fn do_push_repository(repo_path: &Path, remote: &str) {
    // 推送所有分支
    if let Err(e) =
        CommandRunner::run_with_success_in_dir("git", &["push", remote, "--all"], repo_path)
    {
        println!(
            "  推送分支失败: {} - {}",
            utils::format_path(repo_path).red(),
            e
        );
    }

    // 推送所有标签
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
