use crate::app::common::runner::CommandRunner;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

pub fn execute(path: &str, dry_run: bool) -> Result<()> {
    let project_path = Path::new(path);

    if !project_path.exists() {
        anyhow::bail!("项目路径不存在: {}", path);
    }

    if dry_run {
        if !project_path.join(".git").exists() {
            println!("{}", "[DRY-RUN] 将要执行的操作:".green().bold());
            println!("  {} git init", "[DRY-RUN]".yellow());
            println!("  {} git add .", "[DRY-RUN]".yellow());
            println!("  {} git commit -m snap-000000", "[DRY-RUN]".yellow());
        } else {
            let output = CommandRunner::run_with_success_in_dir(
                "git",
                &["rev-list", "--count", "HEAD"],
                project_path,
            )?;
            let num_commit = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<usize>()?;

            println!("{}", "[DRY-RUN] 将要执行的操作:".green().bold());
            println!("  {} git add .", "[DRY-RUN]".yellow());
            println!(
                "  {} git commit -m snap-{:06}",
                "[DRY-RUN]".yellow(),
                num_commit
            );
        }
        return Ok(());
    }

    if !project_path.join(".git").exists() {
        do_initialize_snapshot(project_path)?;
    } else {
        do_incremental_snapshot(project_path)?;
    }

    Ok(())
}

fn do_initialize_snapshot(work_dir: &Path) -> Result<()> {
    CommandRunner::run_with_success_in_dir("git", &["init"], work_dir)?;
    CommandRunner::run_with_success_in_dir("git", &["add", "."], work_dir)?;
    CommandRunner::run_with_success_in_dir("git", &["commit", "-m", "snap-000000"], work_dir)?;

    Ok(())
}

fn do_incremental_snapshot(work_dir: &Path) -> Result<()> {
    let output = CommandRunner::run_with_success_in_dir(
        "git",
        &["rev-list", "--count", "HEAD"],
        work_dir,
    )?;
    let num_commit = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()?;

    CommandRunner::run_with_success_in_dir("git", &["add", "."], work_dir)?;

    CommandRunner::run_with_success_in_dir(
        "git",
        &["commit", "-m", &format!("snap-{:06}", num_commit)],
        work_dir,
    )?;

    Ok(())
}
