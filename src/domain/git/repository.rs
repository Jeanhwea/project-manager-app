use super::{GitError, Result};
use crate::domain::config::ConfigManager;
use std::fs;
use std::path::{Path, PathBuf};

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

    if max_depth == 0 {
        return Ok(repos);
    }

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

pub struct RepoWalker {
    repos: Vec<RepoInfo>,
}

impl RepoWalker {
    pub fn new(path: &Path, max_depth: usize) -> Result<Self> {
        let config = ConfigManager::load_config();
        let repos = find_git_repositories(path, max_depth, &config.repository.skip_dirs)?;
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

    #[test]
    fn test_find_git_repositories_nested() {
        let temp_dir = tempdir().unwrap();
        let _ = find_git_repositories(temp_dir.path(), 1, &[]);
    }
}
