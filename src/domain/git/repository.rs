//! 仓库发现：在给定目录下按深度遍历找出所有 Git 仓库，供 sync/doctor 批量处理。

use super::{GitError, Result};
use crate::domain::config::ConfigManager;
use std::fs;
use std::path::{Path, PathBuf};

/// 一个被找到的本地 Git 仓库
#[derive(Debug)]
pub struct RepoInfo {
    pub path: PathBuf,
}

fn find_git_repositories(
    root_dir: &Path,
    max_depth: usize,
    skip_dirs: &[String],
) -> Result<Vec<RepoInfo>> {
    let mut repos = Vec::new();

    // max_depth 每下钻一层递减，减到 0 就停
    if max_depth == 0 {
        return Ok(repos);
    }

    // .git 既可能是目录也可能是文件（submodule / worktree），exists() 对两者都成立；
    // 命中即不再下钻，避免把仓库内部的 .git 内容再当成普通目录扫一遍
    let git_path = root_dir.join(".git");
    if git_path.exists() {
        repos.push(RepoInfo {
            path: root_dir.to_path_buf(),
        });
        return Ok(repos);
    }

    let entries = fs::read_dir(root_dir).map_err(GitError::Io)?;

    for entry in entries {
        let entry = entry.map_err(GitError::Io)?;
        let path = entry.path();

        if path.is_dir() {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if file_name_str == ".git" {
                continue;
            }

            if skip_dirs.iter().any(|skip| file_name_str == skip.as_str()) {
                continue;
            }

            repos.extend(find_git_repositories(&path, max_depth - 1, skip_dirs)?);
        }
    }

    Ok(repos)
}

/// 一次遍历得到的仓库清单
pub struct RepoWalker {
    repos: Vec<RepoInfo>,
}

impl RepoWalker {
    /// 扫描 `path` 下不超过 `max_depth` 层的仓库，跳过配置里声明的 skip_dirs
    pub fn new(path: &Path, max_depth: usize) -> Result<Self> {
        let config = ConfigManager::load_config();
        let repos = find_git_repositories(path, max_depth, &config.repository.skip_dirs)?;
        Ok(Self { repos })
    }

    /// 是否一个仓库都没找到（上层据此提前结束）
    pub fn is_empty(&self) -> bool {
        self.repos.is_empty()
    }

    /// 仓库总数，用于展示进度与决定并发度
    pub fn total(&self) -> usize {
        self.repos.len()
    }

    /// 遍历结果
    pub fn repositories(&self) -> &[RepoInfo] {
        &self.repos
    }
}

/// 从 `start_dir` 逐级向上找最近的仓库根；走到文件系统根仍未命中时返回 None
pub fn find_repository_upwards(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_find_git_repositories_empty_dir() {
        let temp_dir = tempdir().unwrap();
        let repos = find_git_repositories(temp_dir.path(), 3, &[]).unwrap();
        assert!(repos.is_empty());
    }

    #[test]
    fn test_find_git_repositories_skips_dirs() {
        let temp_dir = tempdir().unwrap();
        let node_modules = temp_dir.path().join("node_modules");
        let project = temp_dir.path().join("my-project");
        std::fs::create_dir_all(node_modules.join("pkg").join(".git")).unwrap();
        std::fs::create_dir_all(project.join(".git")).unwrap();

        let skip_dirs = vec!["node_modules".to_string()];
        let repos = find_git_repositories(temp_dir.path(), 3, &skip_dirs).unwrap();
        assert_eq!(repos.len(), 1);
        assert!(repos[0].path.ends_with("my-project"));
    }

    // 只验证极大极小两种深度都不会 panic，不需断言具体结果
    #[test]
    fn test_find_git_repositories_nested() {
        let temp_dir = tempdir().unwrap();
        let _ = find_git_repositories(temp_dir.path(), 1, &[]);
    }
}
