use super::runner::CommandRunner;
use anyhow::Result;
use std::path::Path;

pub fn execute(path: &str) -> Result<()> {
    let project_path = Path::new(path);

    if !project_path.exists() {
        anyhow::bail!("项目路径不存在: {}", path);
    }

    if !project_path.join(".git").exists() {
        do_init_snapshot(project_path)?;
    } else {
        do_incremental_snapshot(project_path)?;
    }

    Ok(())
}

fn do_init_snapshot(work_dir: &Path) -> Result<()> {
    CommandRunner::run_with_success_in_dir("git", &["init"], work_dir)?;
    CommandRunner::run_with_success_in_dir("git", &["add", "."], work_dir)?;
    CommandRunner::run_with_success_in_dir("git", &["commit", "-m", "init"], work_dir)?;

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
        &["commit", "-m", &format!("snap-{:06}", num_commit + 1)],
        work_dir,
    )?;

    Ok(())
}
