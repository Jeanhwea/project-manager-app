use crate::app::common::git::{self, RepoWalker};
use crate::app::common::runner::{CommandRunner, DryRunContext};
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

pub fn execute_list(path: &str, max_depth: Option<usize>) -> Result<()> {
    let walker = RepoWalker::new(path, max_depth)?;
    if walker.is_empty() {
        return Ok(());
    }

    walker.walk(|entry| {
        list_branches(entry.path);
        Ok(())
    })
}

pub fn execute_clean(
    path: &str,
    max_depth: Option<usize>,
    remote: bool,
    dry_run: bool,
) -> Result<()> {
    let walker = RepoWalker::new(path, max_depth)?;
    if walker.is_empty() {
        return Ok(());
    }

    let ctx = DryRunContext::new(dry_run);

    walker.walk(|entry| {
        clean_merged_branches(&ctx, entry.path, remote)?;
        Ok(())
    })
}

pub fn execute_switch(
    path: &str,
    max_depth: Option<usize>,
    branch_name: &str,
    create: bool,
    dry_run: bool,
) -> Result<()> {
    let walker = RepoWalker::new(path, max_depth)?;
    if walker.is_empty() {
        return Ok(());
    }

    let ctx = DryRunContext::new(dry_run);

    walker.walk(|entry| {
        switch_branch(&ctx, entry.path, branch_name, create)?;
        Ok(())
    })
}

pub fn execute_rename(
    path: &str,
    max_depth: Option<usize>,
    old_name: &str,
    new_name: &str,
    dry_run: bool,
) -> Result<()> {
    let walker = RepoWalker::new(path, max_depth)?;
    if walker.is_empty() {
        return Ok(());
    }

    let ctx = DryRunContext::new(dry_run);

    walker.walk(|entry| {
        rename_branch(&ctx, entry.path, old_name, new_name)?;
        Ok(())
    })
}

fn list_branches(repo_path: &Path) {
    let current = git::get_current_branch_in_dir(repo_path);

    let local_branches = git::get_local_branches(repo_path);
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

    let remote_branches = git::get_remote_branches(repo_path);
    if !remote_branches.is_empty() {
        println!("  远程分支:");
        for branch in &remote_branches {
            println!("    {}", branch.dimmed());
        }
    }
}

fn switch_branch(
    ctx: &DryRunContext,
    repo_path: &Path,
    branch_name: &str,
    create: bool,
) -> Result<()> {
    let current = git::get_current_branch_in_dir(repo_path);

    if current.as_deref() == Some(branch_name) {
        println!("  {} 已在分支 {} 上", "跳过".dimmed(), branch_name.yellow());
        return Ok(());
    }

    if create {
        let local_branches = git::get_local_branches(repo_path);
        let branch_exists = local_branches.iter().any(|b| b == branch_name);

        if branch_exists {
            println!(
                "  {} 分支 {} 已存在，直接切换",
                "提示".yellow(),
                branch_name.yellow()
            );
            ctx.run_in_dir("git", &["checkout", branch_name], Some(repo_path))?;
        } else {
            ctx.run_in_dir("git", &["checkout", "-b", branch_name], Some(repo_path))?;
            if !ctx.is_dry_run() {
                println!(
                    "  {} 创建并切换到分支 {}",
                    "完成".green(),
                    branch_name.yellow()
                );
            }
        }
    } else {
        let local_branches = git::get_local_branches(repo_path);
        let branch_exists = local_branches.iter().any(|b| b == branch_name);

        if !branch_exists {
            println!(
                "  {} 分支 {} 不存在 (使用 --create 创建新分支)",
                "跳过".red(),
                branch_name.red()
            );
            return Ok(());
        }

        ctx.run_in_dir("git", &["checkout", branch_name], Some(repo_path))?;
        if !ctx.is_dry_run() {
            println!("  {} 切换到分支 {}", "完成".green(), branch_name.yellow());
        }
    }

    Ok(())
}

fn rename_branch(
    ctx: &DryRunContext,
    repo_path: &Path,
    old_name: &str,
    new_name: &str,
) -> Result<()> {
    let local_branches = git::get_local_branches(repo_path);

    if !local_branches.iter().any(|b| b == old_name) {
        println!("  {} 分支 {} 不存在", "跳过".dimmed(), old_name.red());
        return Ok(());
    }

    if local_branches.iter().any(|b| b == new_name) {
        println!("  {} 分支 {} 已存在", "跳过".red(), new_name.red());
        return Ok(());
    }

    let current = git::get_current_branch_in_dir(repo_path);
    let is_current = current.as_deref() == Some(old_name);

    ctx.run_in_dir(
        "git",
        &["branch", "-m", old_name, new_name],
        Some(repo_path),
    )?;

    if !ctx.is_dry_run() {
        if is_current {
            println!(
                "  {} 当前分支 {} -> {}",
                "重命名".green(),
                old_name.red(),
                new_name.yellow()
            );
        } else {
            println!(
                "  {} 分支 {} -> {}",
                "重命名".green(),
                old_name.red(),
                new_name.yellow()
            );
        }
    }

    Ok(())
}

fn clean_merged_branches(ctx: &DryRunContext, repo_path: &Path, remote: bool) -> Result<()> {
    let current = git::get_current_branch_in_dir(repo_path).unwrap_or_else(|| "master".to_string());

    let merged_branches = git::get_merged_branches(repo_path, &current);

    if merged_branches.is_empty() {
        println!("  {}", "无已合并分支".green());
        return Ok(());
    }

    for branch in &merged_branches {
        if ctx.is_dry_run() {
            ctx.print_message(&format!("删除本地分支 {}", branch));
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
        let remote_merged = git::get_remote_merged_branches(repo_path, &current);
        if remote_merged.is_empty() {
            println!("  {}", "无已合并的远程分支".green());
        } else {
            for branch in &remote_merged {
                if ctx.is_dry_run() {
                    ctx.print_message(&format!("删除远程分支 {}", branch));
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
