use super::{GitError, RepositoryStatus, Result};
use crate::utils::path::canonicalize_path;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]

pub struct Repository {
    pub path: PathBuf,
    pub status: RepositoryStatus,
    pub remotes: Vec<Remote>,
    pub branches: Vec<Branch>,
    pub repo_type: RepoType,
}

/// Git remote repository (re-export from remote module)
pub use super::remote::Remote;

/// Git branch information
#[derive(Debug, Clone)]

pub struct Branch {
    pub name: String,
    pub is_current: bool,
    pub upstream: Option<String>,
}

/// Repository type
#[derive(Debug, Clone, PartialEq)]
pub enum RepoType {
    Regular,
    Submodule,
}

/// Repository discovery information
#[derive(Debug)]
pub struct RepoInfo {
    pub path: PathBuf,
    pub repo_type: RepoType,
}

impl Repository {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();

        if !path.exists() {
            return Err(GitError::RepositoryNotFound(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }

        // Check if it's a Git repository
        let git_path = path.join(".git");
        if !git_path.exists() {
            return Err(GitError::RepositoryNotFound(format!(
                "Not a Git repository: {}",
                path.display()
            )));
        }

        // Determine repository type
        let repo_type = if git_path.is_dir() {
            RepoType::Regular
        } else {
            RepoType::Submodule
        };

        // Create initial repository instance
        let mut repo = Self {
            path: canonicalize_path(&path).map_err(GitError::Io)?,
            status: RepositoryStatus::Unknown,
            remotes: Vec::new(),
            branches: Vec::new(),
            repo_type,
        };

        repo.refresh()?;

        Ok(repo)
    }

    pub fn refresh(&mut self) -> Result<()> {
        // Check repository status
        self.check_status()?;

        // Load remotes
        self.load_remotes()?;

        // Load branches
        self.load_branches()?;

        Ok(())
    }

    pub fn check_status(&mut self) -> Result<()> {
        use super::command::GitCommandRunner;

        let runner = GitCommandRunner::new();
        let output = runner.execute_in_dir(&["status", "--porcelain"], &self.path)?;
        self.status = if output.trim().is_empty() {
            RepositoryStatus::Clean
        } else {
            RepositoryStatus::Dirty
        };
        Ok(())
    }

    fn load_remotes(&mut self) -> Result<()> {
        use super::remote::RemoteManager;
        let manager = RemoteManager::new();
        let remotes = manager.list_remotes(&self.path)?;
        self.remotes = remotes;
        Ok(())
    }

    fn load_branches(&mut self) -> Result<()> {
        use super::command::GitCommandRunner;

        let runner = GitCommandRunner::new();

        // Get current branch
        let _current_branch = runner
            .execute_in_dir(&["branch", "--show-current"], &self.path)
            .unwrap_or_default();

        // Get all local branches
        let branches_output = match runner.execute_in_dir(&["branch", "--list"], &self.path) {
            Ok(output) => output,
            Err(_) => {
                self.branches = Vec::new();
                return Ok(());
            }
        };

        let mut branches = Vec::new();

        for line in branches_output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let (is_current, name) = if let Some(stripped) = line.strip_prefix('*') {
                (true, stripped.trim())
            } else {
                (false, line)
            };

            let upstream = get_upstream_tracking(&self.path, name).ok();

            branches.push(Branch {
                name: name.to_string(),
                is_current,
                upstream,
            });
        }

        self.branches = branches;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn status(&self) -> &RepositoryStatus {
        &self.status
    }

    pub fn remotes(&self) -> &[Remote] {
        &self.remotes
    }

    pub fn branches(&self) -> &[Branch] {
        &self.branches
    }

    pub fn repo_type(&self) -> &RepoType {
        &self.repo_type
    }

    pub fn is_clean(&self) -> bool {
        self.status == RepositoryStatus::Clean
    }

    pub fn is_dirty(&self) -> bool {
        self.status == RepositoryStatus::Dirty
    }

    pub fn current_branch(&self) -> Option<&str> {
        self.branches
            .iter()
            .find(|b| b.is_current)
            .map(|b| b.name.as_str())
    }

    pub fn remote(&self, name: &str) -> Option<&Remote> {
        self.remotes.iter().find(|r| r.name == name)
    }

    pub fn branch(&self, name: &str) -> Option<&Branch> {
        self.branches.iter().find(|b| b.name == name)
    }
}

/// Check if a path is a Git repository by verifying the `.git` directory exists.

pub fn is_git_repo(path: &Path) -> bool {
    path.is_dir() && path.join(".git").is_dir()
}

/// Search for a Git repository by traversing up the directory tree.
/// Returns the first Git repository found, or None if not found.
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

pub fn find_git_repositories(root_dir: &Path, max_depth: usize) -> Result<Vec<RepoInfo>> {
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

fn get_upstream_tracking(path: &Path, branch_name: &str) -> Result<String> {
    use super::command::GitCommandRunner;

    let runner = GitCommandRunner::new();

    match runner.execute_in_dir(
        &[
            "rev-parse",
            "--abbrev-ref",
            &format!("{}@{{upstream}}", branch_name),
        ],
        path,
    ) {
        Ok(upstream) => Ok(upstream),
        Err(_) => Ok(String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_remote_parsing() {
        use crate::domain::git::GitProtocol;
        use crate::domain::git::remote::Remote;

        assert_eq!(
            Remote::parse_url("ssh://git@example.com/repo.git").unwrap(),
            GitProtocol::Ssh
        );
        assert_eq!(
            Remote::parse_url("git@github.com:user/repo.git").unwrap(),
            GitProtocol::Ssh
        );
        assert_eq!(
            Remote::parse_url("http://example.com/repo.git").unwrap(),
            GitProtocol::Http
        );
        assert_eq!(
            Remote::parse_url("https://example.com/repo.git").unwrap(),
            GitProtocol::Https
        );
        assert_eq!(
            Remote::parse_url("git://example.com/repo.git").unwrap(),
            GitProtocol::Git
        );
    }

    #[test]
    fn test_repository_new_invalid_path() {
        let result = Repository::new("/nonexistent/path");
        assert!(result.is_err());
        match result.unwrap_err() {
            GitError::RepositoryNotFound(_) => (),
            _ => panic!("Expected RepositoryNotFound error"),
        }
    }

    #[test]
    fn test_repository_new_not_git_repo() {
        let temp_dir = tempdir().unwrap();
        let result = Repository::new(temp_dir.path());
        assert!(result.is_err());
        match result.unwrap_err() {
            GitError::RepositoryNotFound(_) => (),
            _ => panic!("Expected RepositoryNotFound error"),
        }
    }

    #[test]
    fn test_find_git_repositories_empty_dir() {
        let temp_dir = tempdir().unwrap();
        let repos = find_git_repositories(temp_dir.path(), 3).unwrap();
        assert!(repos.is_empty());
    }

    #[test]
    fn test_find_git_repositories_nested() {
        // This test would require creating a mock Git repository structure
        // For now, just test that the function doesn't panic
        let temp_dir = tempdir().unwrap();
        let _ = find_git_repositories(temp_dir.path(), 1);
        // No panic means test passes
    }

    #[test]
    fn test_repository_methods() {
        // Test that all public methods exist and have correct signatures
        // This is a compile-time test
        let _: Option<&str> = None::<&Repository>.and_then(|repo| repo.current_branch());
        let _: Option<&Remote> = None::<&Repository>.and_then(|repo| repo.remote("origin"));
        let _: Option<&Branch> = None::<&Repository>.and_then(|repo| repo.branch("main"));
        let _: &Path = Path::new(".");
        let _: &RepositoryStatus = &RepositoryStatus::Clean;
    }
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

    pub fn walk<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(&Path, usize, usize) -> Result<()>,
    {
        let total = self.repos.len();
        for (index, repo) in self.repos.iter().enumerate() {
            let abs_path =
                std::fs::canonicalize(&repo.path).unwrap_or_else(|_| repo.path.clone());
            println!(
                "({}/{})>> {}",
                index + 1,
                total,
                crate::utils::path::format_path(&abs_path)
            );
            callback(&repo.path, index, total)?;
        }
        Ok(())
    }

    pub fn repositories(&self) -> &[RepoInfo] {
        &self.repos
    }
}

pub fn for_each_repo<F>(root_path: &Path, max_depth: usize, mut callback: F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    let walker = RepoWalker::new(root_path, max_depth)?;

    if walker.is_empty() {
        return Ok(());
    }

    walker.walk(|path, _index, _total| callback(path))
}
