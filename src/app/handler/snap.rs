use crate::app::common::runner::{CommandRunner, DryRunContext};
use anyhow::Result;
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

fn do_initialize_snapshot(ctx: &DryRunContext, work_dir: &Path) -> Result<()> {
    ctx.run_in_dir("git", &["init"], Some(work_dir))?;
    ctx.run_in_dir("git", &["add", "."], Some(work_dir))?;
    ctx.run_in_dir("git", &["commit", "-m", "snap-000000"], Some(work_dir))?;

    Ok(())
}

fn do_incremental_snapshot(ctx: &DryRunContext, work_dir: &Path) -> Result<()> {
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
