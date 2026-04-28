use crate::app::common::git;
use crate::app::common::runner::{CommandRunner, DryRunContext};
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

const REQUIRED_TOOLS: &[(&str, &str)] = &[("git", "版本控制工具，所有仓库操作的核心依赖")];

fn check_command_exists(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

fn check_dependencies() -> Result<()> {
    println!("{}", "检查依赖工具...".cyan());

    let mut missing = Vec::new();
    for (cmd, desc) in REQUIRED_TOOLS {
        if check_command_exists(cmd) {
            println!("  {} {}", "✔".green(), cmd);
        } else {
            println!("  {} {} ({})", "✘".red(), cmd.red(), desc);
            missing.push(*cmd);
        }
    }

    if !missing.is_empty() {
        anyhow::bail!("缺少必要的命令行工具: {}", missing.join(", "));
    }

    println!("{}", "所有依赖工具已就绪".green());
    println!();
    Ok(())
}

pub fn execute(
    path: &str,
    max_depth: Option<usize>,
    gc: bool,
    rename: bool,
    fix: bool,
    dry_run: bool,
) -> Result<()> {
    check_dependencies()?;

    let ctx = DryRunContext::new(dry_run);

    crate::app::common::git::for_each_repo(path, max_depth, |repo_path| {
        let mut issues = Vec::new();

        check_detached_head(repo_path, &mut issues);
        check_stale_remote_refs(repo_path, &mut issues);
        check_large_repo(repo_path, &mut issues);
        check_missing_upstream(repo_path, &mut issues);
        check_stash(repo_path, &mut issues);

        if !issues.is_empty() {
            println!("  {}", "发现问题:".yellow());
            for issue in &issues {
                println!("    {} {}", "⚠".yellow(), issue);
            }
        } else {
            println!("  {} {}", "✔".green(), "仓库健康".green());
        }

        if fix && !issues.is_empty() {
            fix_issues(&ctx, repo_path, &issues)?;
        }

        if gc {
            do_git_garbage_collect(&ctx, repo_path)?;
        }

        if rename {
            for (remote_name, remote_url) in git::get_remote_info(repo_path) {
                if let Some(new_name) = git::get_remote_name_by_url(&remote_url)
                    && new_name != remote_name
                {
                    println!(
                        "  {} => {}: {}",
                        remote_name.yellow(),
                        new_name.yellow(),
                        remote_url
                    );
                    do_rename_git_remote(&ctx, repo_path, &remote_name, &new_name)?;
                }
            }
        }

        Ok(())
    })
}

fn check_detached_head(repo_path: &Path, issues: &mut Vec<String>) {
    let output =
        match CommandRunner::run_quiet_in_dir("git", &["branch", "--show-current"], repo_path) {
            Ok(o) => o,
            Err(_) => return,
        };

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        issues.push("HEAD 处于 detached 状态".to_string());
    }
}

fn check_stale_remote_refs(repo_path: &Path, issues: &mut Vec<String>) {
    let remotes = git::get_remote_name(repo_path);
    if remotes.is_empty() {
        return;
    }

    for remote in &remotes {
        let output = match CommandRunner::run_quiet_in_dir(
            "git",
            &["remote", "show", remote],
            repo_path,
        ) {
            Ok(o) => o,
            Err(_) => continue,
        };

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("does not appear to be a git repository")
            || stderr.contains("could not read from remote repository")
            || stderr.contains("fatal:")
        {
            issues.push(format!("远程仓库 '{}' 不可达", remote));
        }
    }

    let stale_output = match CommandRunner::run_quiet_in_dir("git", &["branch", "-r"], repo_path)
    {
        Ok(o) => o,
        Err(_) => return,
    };

    let stale_stdout = String::from_utf8_lossy(&stale_output.stdout);
    let stale_branches: Vec<&str> = stale_stdout
        .lines()
        .filter(|line| line.contains(": gone"))
        .map(|line| line.trim())
        .collect();

    if !stale_branches.is_empty() {
        issues.push(format!(
            "存在 {} 个陈旧的远程跟踪分支",
            stale_branches.len()
        ));
    }
}

fn check_large_repo(repo_path: &Path, issues: &mut Vec<String>) {
    let output =
        match CommandRunner::run_quiet_in_dir("git", &["count-objects", "-vH"], repo_path) {
            Ok(o) => o,
            Err(_) => return,
        };

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if let Some(size_str) = line.strip_prefix("size-pack:") {
            let size_str = size_str.trim();
            if let Some(num_part) = size_str.split_whitespace().next()
                && let Ok(size) = num_part.parse::<f64>()
            {
                let unit = size_str.split_whitespace().nth(1).unwrap_or("bytes");
                let size_mb = match unit {
                    "GiB" => size * 1024.0,
                    "MiB" => size,
                    "KiB" => size / 1024.0,
                    _ => size / (1024.0 * 1024.0),
                };
                if size_mb > 500.0 {
                    issues.push(format!("仓库体积较大 ({}), 建议执行 git gc", size_str));
                }
            }
        }
    }
}

fn check_missing_upstream(repo_path: &Path, issues: &mut Vec<String>) {
    let branch = match get_current_branch(repo_path) {
        Some(b) => b,
        None => return,
    };

    let output = CommandRunner::run_quiet_in_dir(
        "git",
        &[
            "rev-parse",
            "--abbrev-ref",
            &format!("{}@{{upstream}}", branch),
        ],
        repo_path,
    );

    if output.is_err() {
        let remotes = git::get_remote_name(repo_path);
        if !remotes.is_empty() {
            issues.push(format!("当前分支 '{}' 没有设置上游跟踪分支", branch));
        }
    }
}

fn check_stash(repo_path: &Path, issues: &mut Vec<String>) {
    let output = match CommandRunner::run_quiet_in_dir("git", &["stash", "list"], repo_path) {
        Ok(o) => o,
        Err(_) => return,
    };

    let stash_count = String::from_utf8_lossy(&output.stdout).lines().count();

    if stash_count > 5 {
        issues.push(format!(
            "stash 列表中有 {} 个条目，可能需要清理",
            stash_count
        ));
    }
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

fn fix_issues(ctx: &DryRunContext, repo_path: &Path, issues: &[String]) -> Result<()> {
    println!("  {}", "修复问题:".cyan());

    for issue in issues {
        if issue.contains("陈旧的远程跟踪分支") {
            println!("  {} 清理陈旧的远程跟踪分支", "修复".green());
            ctx.run_in_dir("git", &["remote", "prune", "origin"], Some(repo_path))
                .with_context(|| "无法清理陈旧的远程跟踪分支")?;
        } else if issue.contains("detached") {
            println!(
                "  {} 无法自动修复 detached HEAD，请手动切换到分支",
                "跳过".yellow()
            );
        } else if issue.contains("上游跟踪分支") {
            if let Some(branch) = get_current_branch(repo_path) {
                let upstream = format!("origin/{}", branch);
                println!(
                    "  {} 设置 {} 的上游为 {}",
                    "修复".green(),
                    branch.yellow(),
                    upstream.yellow()
                );
                ctx.run_in_dir(
                    "git",
                    &["branch", "--set-upstream-to", &upstream, &branch],
                    Some(repo_path),
                )
                .with_context(|| format!("无法设置 {} 的上游跟踪分支", branch))?;
            }
        } else if issue.contains("体积较大") {
            println!("  {} 执行 git gc --aggressive", "修复".green());
            ctx.run_in_dir("git", &["gc", "--aggressive"], Some(repo_path))
                .with_context(|| "无法执行 git gc --aggressive")?;
        } else if issue.contains("stash") {
            println!(
                "  {} stash 条目较多，请手动清理 (git stash drop/pop)",
                "提示".yellow()
            );
        }
    }

    Ok(())
}

fn do_rename_git_remote(
    ctx: &DryRunContext,
    repo_path: &Path,
    old_name: &str,
    new_name: &str,
) -> Result<()> {
    let existing_remotes = git::get_remote_info(repo_path);
    let conflict = existing_remotes.iter().find(|(name, _)| name == new_name);

    if let Some((_, conflict_url)) = conflict {
        let alt_name = git::get_remote_name_by_url(conflict_url)
            .unwrap_or_else(|| format!("{}-old", new_name));

        if alt_name == new_name {
            anyhow::bail!(
                "远程仓库 {} 的 URL ({}) 推断名称仍为 {}，无法解决冲突",
                new_name,
                conflict_url,
                alt_name
            );
        }

        println!("  {} => {}", new_name.yellow(), alt_name.yellow(),);
        ctx.run_in_dir(
            "git",
            &["remote", "rename", new_name, &alt_name],
            Some(repo_path),
        )
        .with_context(|| format!("无法重命名远程仓库 {} -> {}", new_name, alt_name))?;
    }

    ctx.run_in_dir(
        "git",
        &["remote", "rename", old_name, new_name],
        Some(repo_path),
    )
    .with_context(|| format!("无法重命名远程仓库 {} -> {}", old_name, new_name))?;
    Ok(())
}

fn do_git_garbage_collect(ctx: &DryRunContext, repo_path: &Path) -> Result<()> {
    ctx.run_in_dir("git", &["gc"], Some(repo_path))
        .with_context(|| "无法执行 git gc")?;
    Ok(())
}
