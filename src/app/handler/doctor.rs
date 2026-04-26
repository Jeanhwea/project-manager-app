use crate::app::common::git;
use crate::app::common::runner::DryRunContext;
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
    dry_run: bool,
) -> Result<()> {
    check_dependencies()?;

    let ctx = DryRunContext::new(dry_run);

    crate::app::common::git::for_each_repo(path, max_depth, |repo_path| {
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
