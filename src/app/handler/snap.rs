use crate::app::common::runner::{CommandRunner, DryRunContext};
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

pub fn execute(path: &str, dry_run: bool) -> Result<()> {
    let project_path = Path::new(path);
    let ctx = DryRunContext::new(dry_run);

    if !project_path.exists() {
        anyhow::bail!("项目路径不存在: {}", path);
    }

    if ctx.is_dry_run() {
        ctx.print_header("[DRY-RUN] 将要执行的操作:");
    }

    if !project_path.join(".git").exists() {
        do_initialize_snapshot(&ctx, project_path)?;
    } else {
        do_incremental_snapshot(&ctx, project_path)?;
    }

    Ok(())
}

pub fn execute_list(path: &str) -> Result<()> {
    let project_path = Path::new(path);

    if !project_path.exists() {
        anyhow::bail!("项目路径不存在: {}", path);
    }

    if !project_path.join(".git").exists() {
        println!("{} 项目尚未初始化快照", "提示".yellow());
        return Ok(());
    }

    let output = CommandRunner::run_quiet_in_dir("git", &["log", "--oneline"], project_path)?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let snap_commits: Vec<&str> = stdout
        .lines()
        .filter(|line| line.contains("snap-"))
        .collect();

    if snap_commits.is_empty() {
        println!("{} 无快照记录", "提示".yellow());
        return Ok(());
    }

    println!("{}", "快照历史:".cyan());
    for (index, commit) in snap_commits.iter().enumerate() {
        let parts: Vec<&str> = commit.splitn(2, ' ').collect();
        if parts.len() == 2 {
            let hash = parts[0];
            let message = parts[1];
            println!(
                "  {} {} {}",
                format!("#{}", index).dimmed(),
                hash.yellow(),
                message.green(),
            );
        } else {
            println!("  {} {}", format!("#{}", index).dimmed(), commit);
        }
    }

    let total = snap_commits.len();
    println!();
    println!(
        "{} 共 {} 个快照",
        "汇总".cyan(),
        total.to_string().white().bold()
    );

    Ok(())
}

pub fn execute_restore(path: &str, snapshot: &str, dry_run: bool) -> Result<()> {
    let project_path = Path::new(path);
    let ctx = DryRunContext::new(dry_run);

    if !project_path.exists() {
        anyhow::bail!("项目路径不存在: {}", path);
    }

    if !project_path.join(".git").exists() {
        anyhow::bail!("项目尚未初始化快照，无法恢复");
    }

    let commit_ref = resolve_snapshot_ref(project_path, snapshot)?;

    if ctx.is_dry_run() {
        ctx.print_header("[DRY-RUN] 将要执行的操作:");
        ctx.print_message(&format!("git checkout {}", commit_ref));
        return Ok(());
    }

    let output =
        CommandRunner::run_with_success_in_dir("git", &["checkout", &commit_ref], project_path)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        print!("{}", stdout);
    }

    println!("{} 已恢复到快照 {}", "完成".green(), commit_ref.yellow());
    println!(
        "{} 若要回到最新状态，请执行: git checkout -",
        "提示".yellow()
    );

    Ok(())
}

fn resolve_snapshot_ref(project_path: &Path, snapshot: &str) -> Result<String> {
    if snapshot.starts_with("snap-") {
        let output = CommandRunner::run_quiet_in_dir(
            "git",
            &["log", "--oneline", "--grep", snapshot],
            project_path,
        )?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Some(first_line) = stdout.lines().next() {
            let hash = first_line.split_whitespace().next().unwrap_or(snapshot);
            return Ok(hash.to_string());
        }
    }

    if let Some(index_str) = snapshot.strip_prefix('#')
        && let Ok(index) = index_str.parse::<usize>()
    {
        let output = CommandRunner::run_quiet_in_dir("git", &["log", "--oneline"], project_path)?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let snap_commits: Vec<&str> = stdout
            .lines()
            .filter(|line| line.contains("snap-"))
            .collect();

        if index < snap_commits.len() {
            let hash = snap_commits[index]
                .split_whitespace()
                .next()
                .unwrap_or(snapshot);
            return Ok(hash.to_string());
        } else {
            anyhow::bail!(
                "快照索引 #{} 超出范围 (共 {} 个快照)",
                index,
                snap_commits.len()
            );
        }
    }

    let output = CommandRunner::run_quiet_in_dir(
        "git",
        &["rev-parse", "--verify", snapshot],
        project_path,
    )?;

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if hash.is_empty() {
        anyhow::bail!("无法解析快照引用: {}", snapshot);
    }

    Ok(hash)
}

fn do_initialize_snapshot(ctx: &DryRunContext, work_dir: &Path) -> Result<()> {
    ctx.run_in_dir("git", &["init"], Some(work_dir))?;
    ctx.run_in_dir("git", &["add", "."], Some(work_dir))?;
    ctx.run_in_dir("git", &["commit", "-m", "snap-000000"], Some(work_dir))?;

    Ok(())
}

fn do_incremental_snapshot(ctx: &DryRunContext, work_dir: &Path) -> Result<()> {
    let has_changes = check_pending_changes(work_dir);

    if !has_changes {
        println!("{} 无变更，跳过快照", "提示".dimmed());
        return Ok(());
    }

    let output = CommandRunner::run_with_success_in_dir(
        "git",
        &["rev-list", "--count", "HEAD"],
        work_dir,
    )?;
    let num_commit = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()?;

    ctx.run_in_dir("git", &["add", "."], Some(work_dir))?;
    ctx.run_in_dir(
        "git",
        &["commit", "-m", &format!("snap-{:06}", num_commit)],
        Some(work_dir),
    )?;

    Ok(())
}

fn check_pending_changes(work_dir: &Path) -> bool {
    let output =
        match CommandRunner::run_quiet_in_dir("git", &["status", "--porcelain"], work_dir) {
            Ok(o) => o,
            Err(_) => return true,
        };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return true,
    };

    !stdout.trim().is_empty()
}
