use super::git;
use super::runner::CommandRunner;
use colored::Colorize;

use super::repo::{ RepoType};

pub fn execute(path: &str, max_depth: Option<usize>) {
    let sync_dir = std::path::Path::new(path);
    let git_repos = super::repo::find_git_repositories(sync_dir, max_depth);

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

        // 打印本地分支
        CommandRunner::run_with_success_in_dir(
            "git",
            &["branch", "--list"],
            repo.path.to_str().unwrap(),
        )
        .unwrap();

        // 打印远程仓库信息
        CommandRunner::run_with_success_in_dir(
            "git",
            &["remote", "-v"],
            repo.path.to_str().unwrap(),
        )
        .unwrap();

        // 执行 git pull 命令
        if let Some(path_str) = repo_path.to_str() {
            // git pull
            if CommandRunner::run_with_success_in_dir("git", &["pull"], path_str).is_err() {
                println!("同步仓库失败: {}", display_path.red());
            }

            // 对每个远程仓库执行 git push
            for (remote, url) in remotes {
                if let Some((protocol, host, path)) = git::parse_git_remote_url(&url) {
                    let skip_push = if protocol == "https" && host == "github.com" {
                        true
                    } else if protocol == "git"
                        && host == "gitee.com"
                        && path.starts_with("red_base")
                    {
                        true
                    } else {
                        false
                    };

                    if skip_push {
                        println!("跳过推送 {} ({})", remote, url.green());
                        continue;
                    }

                    if CommandRunner::run_with_success_in_dir(
                        "git",
                        &["push", &remote, "--all"],
                        path_str,
                    )
                    .is_err()
                    {
                        println!("推送仓库失败: {}", display_path.red());
                    }

                    if CommandRunner::run_with_success_in_dir(
                        "git",
                        &["push", &remote, "--tags"],
                        path_str,
                    )
                    .is_err()
                    {
                        println!("推送仓库失败: {}", display_path.red());
                    }
                }
            }
        } else {
            println!("同步仓库路径无效: {}", display_path.red());
        }
    }
}
