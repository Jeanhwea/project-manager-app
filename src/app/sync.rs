use super::runner::CommandRunner;
use std::fs;
use std::path::Path;

// 定义仓库类型枚举
#[derive(PartialEq)]
enum RepoType {
    Regular,   // 普通 git 仓库
    Submodule, // git 子模块
}

// 定义仓库信息结构体
struct RepoInfo {
    path: std::path::PathBuf,
    repo_type: RepoType,
}

pub fn execute(path: &str) {
    let sync_dir = std::path::Path::new(path);
    let git_repos = find_git_repositories(sync_dir);

    if git_repos.is_empty() {
        println!("未找到git仓库");
        return;
    }

    for repo in git_repos {
        // 只对普通 git 仓库执行 git pull，跳过子模块
        if repo.repo_type == RepoType::Submodule {
            continue;
        }

        let repo_path = if let Ok(abs_path) = repo.path.canonicalize() {
            abs_path
        } else {
            repo.path.clone()
        };

        println!("同步仓库: {}", repo_path.display());

        // 执行 git pull 命令
        if let Some(path_str) = repo_path.to_str() {
            if CommandRunner::run_with_success_in_dir("git", &["pull"], path_str).is_err() {
                println!("同步仓库失败: {}", repo_path.display());
            }
        } else {
            println!("同步仓库路径无效: {}", repo_path.display());
        }
    }
}

fn find_git_repositories(dir: &Path) -> Vec<RepoInfo> {
    find_git_repositories_with_depth(dir, 5) // 默认最大深度为5
}

fn find_git_repositories_with_depth(dir: &Path, max_depth: usize) -> Vec<RepoInfo> {
    let mut repos = Vec::new();

    if max_depth == 0 {
        return repos;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                let file_name = entry.file_name();
                let file_name_str = file_name.to_str().unwrap_or("");

                // 跳过 .venv 目录
                if file_name_str == ".venv" {
                    continue;
                }

                if path.is_dir() {
                    if file_name_str == ".git" {
                        // 找到普通 git 仓库，添加其父目录
                        if let Some(parent) = path.parent() {
                            repos.push(RepoInfo {
                                path: parent.to_path_buf(),
                                repo_type: RepoType::Regular,
                            });
                        }
                    } else {
                        // 递归搜索子目录，深度减1
                        repos.extend(find_git_repositories_with_depth(&path, max_depth - 1));
                    }
                } else if file_name_str == ".git" {
                    // 找到 git 子模块（有 .git 文件而不是目录）
                    if let Some(parent) = path.parent() {
                        repos.push(RepoInfo {
                            path: parent.to_path_buf(),
                            repo_type: RepoType::Submodule,
                        });
                    }
                }
            }
        }
    }

    repos
}
