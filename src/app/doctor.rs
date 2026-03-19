use super::git;
use super::runner::CommandRunner;
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

pub fn execute(path: &str, max_depth: Option<usize>, gc: bool) -> Result<()> {
    super::repo::for_each_repo(path, max_depth, |repo, repo_path, _, _| {
        if gc {
            do_git_garbage_collect(repo_path)?;
        }

        let remotes = git::get_remote_info(&repo.path);
        for (remote_name, remote_url) in remotes {
            if let Some(new_name) = git::get_remote_name_by_url(&remote_url) {
                if new_name != remote_name {
                    println!(
                        "  {} => {}: {}",
                        remote_name.yellow(),
                        new_name.yellow(),
                        remote_url
                    );
                    do_rename_git_remote(&repo.path, &remote_name, &new_name)?;
                }
            }
        }

        Ok(())
    })
}

fn do_rename_git_remote(repo_path: &Path, old_name: &str, new_name: &str) -> Result<()> {
    CommandRunner::run_with_success_in_dir(
        "git",
        &["remote", "rename", old_name, new_name],
        repo_path,
    )
    .with_context(|| format!("无法重命名远程仓库 {} -> {}", old_name, new_name))?;
    Ok(())
}

fn do_git_garbage_collect(repo_path: &Path) -> Result<()> {
    CommandRunner::run_with_success_in_dir("git", &["gc"], repo_path)
        .with_context(|| "无法执行 git gc")?;
    Ok(())
}
