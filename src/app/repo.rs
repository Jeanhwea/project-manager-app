use std::fs;
use std::path::Path;

// 定义仓库类型枚举
#[derive(PartialEq)]
pub enum RepoType {
    Regular,   // 普通 git 仓库
    Submodule, // git 子模块
}

// 定义仓库信息结构体
pub struct RepoInfo {
    pub path: std::path::PathBuf,
    pub repo_type: RepoType,
}

pub fn find_git_repositories(dir: &Path, max_depth: Option<usize>) -> Vec<RepoInfo> {
    find_git_repositories_with_depth(dir, max_depth.unwrap_or(3))
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
