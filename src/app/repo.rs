use crate::utils;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_MAX_DEPTH: usize = 3;

#[derive(PartialEq)]
pub enum RepoType {
    Regular,
    Submodule,
}

pub struct RepoInfo {
    pub path: PathBuf,
    pub repo_type: RepoType,
}

pub fn find_git_repositories(root_dir: &Path, max_depth: Option<usize>) -> Vec<RepoInfo> {
    find_git_repositories_with_depth(root_dir, max_depth.unwrap_or(DEFAULT_MAX_DEPTH))
}

fn find_git_repositories_with_depth(root_dir: &Path, max_depth: usize) -> Vec<RepoInfo> {
    let mut repos = Vec::new();

    if max_depth == 0 {
        return repos;
    }

    let Ok(entries) = fs::read_dir(root_dir) else {
        return repos;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_str().unwrap_or("");

        // 跳过虚拟环境目录
        if file_name_str == ".venv" {
            continue;
        }

        if file_name_str == ".git" {
            if let Some(parent) = path.parent() {
                let repo_type = if path.is_dir() {
                    RepoType::Regular
                } else {
                    // .git 是文件时表示子模块
                    RepoType::Submodule
                };
                repos.push(RepoInfo {
                    path: parent.to_path_buf(),
                    repo_type,
                });
            }
        } else if path.is_dir() {
            repos.extend(find_git_repositories_with_depth(&path, max_depth - 1));
        }
    }

    repos
}

/// 遍历仓库并对每个仓库执行回调，提供统一的进度输出和路径规范化
pub fn for_each_repo<F>(
    path: &str,
    max_depth: Option<usize>,
    mut callback: F,
) -> anyhow::Result<()>
where
    F: FnMut(&RepoInfo, &Path, usize, usize) -> anyhow::Result<()>,
{
    let root_dir = Path::new(path);

    if !root_dir.exists() {
        anyhow::bail!("目录不存在: {}", path);
    }

    let git_repos = find_git_repositories(root_dir, max_depth);

    if git_repos.is_empty() {
        println!("未找到git仓库");
        return Ok(());
    }

    let total = git_repos.len();

    for (index, repo) in git_repos.iter().enumerate() {
        let repo_path = repo
            .path
            .canonicalize()
            .unwrap_or_else(|_| repo.path.clone());

        let progress = format!("({}/{})", index + 1, total);
        println!(
            "{}>> {}",
            progress.white().bold(),
            utils::format_path(&repo_path).cyan().underline(),
        );

        if repo.repo_type == RepoType::Submodule {
            continue;
        }

        callback(repo, &repo_path, index, total)?;
    }

    Ok(())
}
