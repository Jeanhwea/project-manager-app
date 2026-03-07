use super::git;
use super::runner::CommandRunner;
use colored::Colorize;
use std::path::Path;

use super::repo::RepoType;

pub fn execute(path: &str, max_depth: Option<usize>) {
    let root_dir = std::path::Path::new(path);
    let git_repos = super::repo::find_git_repositories(root_dir, max_depth);

    if git_repos.is_empty() {
        println!("未找到git仓库");
        return;
    }

    let total_repos = git_repos.len();

    for (repo_index, repo) in git_repos.iter().enumerate() {
        let repo_path = if let Ok(abs_path) = repo.path.canonicalize() {
            abs_path
        } else {
            repo.path.clone()
        };

        // 优化路径显示，移除 Windows UNC 路径前缀
        let mut display_path = repo_path.to_string_lossy().to_string();
        display_path = display_path.trim_start_matches("\\\\?\\").to_string();
        println!(
            "({}/{}) <<= {}",
            repo_index + 1,
            total_repos,
            display_path.cyan()
        );

        // 只对普通 git 仓库执行 git pull，跳过子模块
        if repo.repo_type == RepoType::Submodule {
            continue;
        }

        // 获取远程仓库名称
        let remotes = git::get_remote_info(&repo.path);
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
                    do_rename_git_remote(&repo.path, &remote_name, &new_name);
                }
                _ => continue,
            }
        }
    }
}

fn do_rename_git_remote(repo_path: &Path, old_name: &str, new_name: &str) {
    CommandRunner::run_with_success_in_dir(
        "git",
        &["remote", "rename", old_name, new_name],
        repo_path.to_str().unwrap(),
    )
    .unwrap();
}
