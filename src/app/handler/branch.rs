use crate::app::common::git;
use crate::app::common::runner::{CommandRunner, DryRunContext};
use crate::utils;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

pub fn execute_list(path: &str, max_depth: Option<usize>) -> Result<()> {
    let root_dir = Path::new(path);

    if !root_dir.exists() {
        anyhow::bail!("目录不存在: {}", path);
    }

    let git_repos = git::find_git_repositories_or_current(root_dir, max_depth);

    if git_repos.is_empty() {
        println!("未找到git仓库");
        return Ok(());
    }

    let total = git_repos.len();

    for (index, repo) in git_repos.iter().enumerate() {
        let repo_path = repo
            .path
            .canonicalize()
            .unwrap_or_else(|_| repo.path.clone());

        let progress = format!("({}/{})", index + 1, total);
        println!(
            "{}>> {}",
            progress.white().bold(),
            utils::format_path(&repo_path).cyan().underline(),
        );

        if repo.repo_type == git::RepoType::Submodule {
            println!("  {}", "(submodule, 跳过)".dimmed());
            continue;
        }

        list_branches(&repo_path);
    }

    Ok(())
}

pub fn execute_clean(
    path: &str,
    max_depth: Option<usize>,
    remote: bool,
    dry_run: bool,
) -> Result<()> {
    let root_dir = Path::new(path);

    if !root_dir.exists() {
        anyhow::bail!("目录不存在: {}", path);
    }

    let git_repos = git::find_git_repositories_or_current(root_dir, max_depth);

    if git_repos.is_empty() {
        println!("未找到git仓库");
        return Ok(());
    }

    let ctx = DryRunContext::new(dry_run);
    let total = git_repos.len();

    for (index, repo) in git_repos.iter().enumerate() {
        let repo_path = repo
            .path
            .canonicalize()
            .unwrap_or_else(|_| repo.path.clone());

        let progress = format!("({}/{})", index + 1, total);
        println!(
            "{}>> {}",
            progress.white().bold(),
            utils::format_path(&repo_path).cyan().underline(),
        );

        if repo.repo_type == git::RepoType::Submodule {
            println!("  {}", "(submodule, 跳过)".dimmed());
            continue;
        }

        clean_merged_branches(&ctx, &repo_path, remote)?;
    }

    Ok(())
}

fn list_branches(repo_path: &Path) {
    let current = get_current_branch(repo_path);

    let local_branches = get_local_branches(repo_path);
    if !local_branches.is_empty() {
        println!("  本地分支:");
        for branch in &local_branches {
            if Some(branch.as_str()) == current.as_deref() {
                println!("    {} {}", "*".green(), branch.yellow());
            } else {
                println!("     {}", branch);
            }
        }
    }

    let remote_branches = get_remote_branches(repo_path);
    if !remote_branches.is_empty() {
        println!("  远程分支:");
        for branch in &remote_branches {
            println!("    {}", branch.dimmed());
        }
    }
}

fn get_local_branches(repo_path: &Path) -> Vec<String> {
    let output = match CommandRunner::run_quiet_in_dir("git", &["branch", "--list"], repo_path) {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stdout
        .lines()
        .map(|line| line.trim_start_matches('*').trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn get_remote_branches(repo_path: &Path) -> Vec<String> {
    let output = match CommandRunner::run_quiet_in_dir("git", &["branch", "-r"], repo_path) {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|s| !s.is_empty() && !s.contains("->"))
        .collect()
}

fn get_current_branch(repo_path: &Path) -> Option<String> {
    let output =
        CommandRunner::run_quiet_in_dir("git", &["branch", "--show-current"], repo_path).ok()?;

    let branch = String::from_utf8(output.stdout).ok()?;
    let branch = branch.trim();

    if branch.is_empty() {
        None
    } else {
        Some(branch.to_string())
    }
}

fn clean_merged_branches(ctx: &DryRunContext, repo_path: &Path, remote: bool) -> Result<()> {
    let current = get_current_branch(repo_path).unwrap_or_else(|| "master".to_string());

    let merged_branches = get_merged_branches(repo_path, &current);

    if merged_branches.is_empty() {
        println!("  {}", "无已合并分支".green());
        return Ok(());
    }

    for branch in &merged_branches {
        if ctx.is_dry_run() {
            println!("  {} 删除本地分支 {}", "[DRY-RUN]".yellow(), branch.red());
        } else {
            let result = CommandRunner::run_with_success_in_dir(
                "git",
                &["branch", "-d", branch],
                repo_path,
            );
            match result {
                Ok(_) => println!("  {} 本地分支 {}", "已删除".green(), branch.red()),
                Err(e) => println!("  {} 本地分支 {} - {}", "删除失败".red(), branch.red(), e),
            }
        }
    }

    if remote {
        let remote_merged = get_remote_merged_branches(repo_path, &current);
        if remote_merged.is_empty() {
            println!("  {}", "无已合并的远程分支".green());
        } else {
            for branch in &remote_merged {
                if ctx.is_dry_run() {
                    println!("  {} 删除远程分支 {}", "[DRY-RUN]".yellow(), branch.red());
                } else {
                    let result = CommandRunner::run_with_success_in_dir(
                        "git",
                        &["push", "origin", "--delete", branch],
                        repo_path,
                    );
                    match result {
                        Ok(_) => println!("  {} 远程分支 {}", "已删除".green(), branch.red()),
                        Err(e) => {
                            println!("  {} 远程分支 {} - {}", "删除失败".red(), branch.red(), e)
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn get_merged_branches(repo_path: &Path, current_branch: &str) -> Vec<String> {
    let output = match CommandRunner::run_quiet_in_dir(
        "git",
        &["branch", "--merged", current_branch],
        repo_path,
    ) {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stdout
        .lines()
        .map(|line| line.trim_start_matches('*').trim().to_string())
        .filter(|s| !s.is_empty() && s != current_branch)
        .collect()
}

fn get_remote_merged_branches(repo_path: &Path, current_branch: &str) -> Vec<String> {
    let output = match CommandRunner::run_quiet_in_dir(
        "git",
        &["branch", "-r", "--merged", current_branch],
        repo_path,
    ) {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let current_remote = format!("origin/{}", current_branch);

    stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|s| {
            !s.is_empty() && !s.contains("->") && s != &current_remote && s.starts_with("origin/")
        })
        .map(|s| s.strip_prefix("origin/").unwrap_or(&s).to_string())
        .collect()
}
