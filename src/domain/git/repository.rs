use super::{GitError, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum RepoType {
    Regular,
    Submodule,
}

#[derive(Debug)]
pub struct RepoInfo {
    pub path: PathBuf,
    pub repo_type: RepoType,
}

pub fn find_git_repository_upwards(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir;

    loop {
        let git_path = current.join(".git");
        if git_path.exists() {
            return Some(current.to_path_buf());
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => return None,
        }
    }
}

fn find_git_repositories(root_dir: &Path, max_depth: usize) -> Result<Vec<RepoInfo>> {
    let mut repos = Vec::new();

    if max_depth == 0 {
        return Ok(repos);
    }

    let git_path = root_dir.join(".git");
    if git_path.exists() {
        let repo_type = if git_path.is_dir() {
            RepoType::Regular
        } else {
            RepoType::Submodule
        };

        repos.push(RepoInfo {
            path: root_dir.to_path_buf(),
            repo_type,
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

            repos.extend(find_git_repositories(&path, max_depth - 1)?);
        }
    }

    Ok(repos)
}

pub struct RepoWalker {
    repos: Vec<RepoInfo>,
}

impl RepoWalker {
    pub fn new(path: &Path, max_depth: usize) -> Result<Self> {
        let repos = find_git_repositories(path, max_depth)?;
        Ok(Self { repos })
    }

    pub fn is_empty(&self) -> bool {
        self.repos.is_empty()
    }

    pub fn total(&self) -> usize {
        self.repos.len()
    }

    pub fn repositories(&self) -> &[RepoInfo] {
        &self.repos
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_find_git_repositories_empty_dir() {
        let temp_dir = tempdir().unwrap();
        let repos = find_git_repositories(temp_dir.path(), 3).unwrap();
        assert!(repos.is_empty());
    }

    #[test]
    fn test_find_git_repositories_nested() {
        let temp_dir = tempdir().unwrap();
        let _ = find_git_repositories(temp_dir.path(), 1);
    }
}
