use super::git;
use super::runner::CommandRunner;
use crate::utils;
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

use super::repo::RepoType;

pub fn execute(path: &str, max_depth: Option<usize>, gc: bool) -> Result<()> {
    let root_dir = Path::new(path);

    if !root_dir.exists() {
        anyhow::bail!("目录不存在: {}", path);
    }

    let git_repos = super::repo::find_git_repositories(root_dir, max_depth);

    if git_repos.is_empty() {
        println!("未找到git仓库");
        return Ok(());
    }

    let total_repos = git_repos.len();

    for (repo_index, repo) in git_repos.iter().enumerate() {
        let repo_path = if let Ok(abs_path) = repo.path.canonicalize() {
            abs_path
        } else {
            repo.path.clone()
        };

        let progress = format!("({}/{})", repo_index + 1, total_repos);
        println!(
            "{}>> {}",
            progress.white().bold(),
            utils::format_path(&repo_path).cyan().underline(),
        );

        // 只对普通 git 仓库执行 git pull，跳过子模块
        if repo.repo_type == RepoType::Submodule {
            continue;
        }

        if gc {
            do_git_garbage_collect(&repo_path)?;
        }

        // 获取远程仓库名称
        let remotes = git::get_remote_info(&repo_path);
        if remotes.is_empty() {
            continue;
        }

        // 打印远程仓库信息
        for (remote_name, remote_url) in remotes {
            match git::get_remote_name_by_url(&remote_url) {
                Some(new_name) if new_name != remote_name => {
                    println!(
                        "  {} => {}: {}",
                        remote_name.yellow(),
                        new_name.yellow(),
                        remote_url
                    );
                    do_rename_git_remote(&repo.path, &remote_name, &new_name)?;
                }
                _ => continue,
            }
        }
    }

    Ok(())
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
