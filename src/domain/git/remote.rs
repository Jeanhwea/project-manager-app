use crate::domain::git::{GitError, Result};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Remote {
    pub name: String,
    pub url: String,
}

pub struct RemoteManager {
    runner: crate::domain::git::command::GitCommandRunner,
}

impl RemoteManager {
    pub fn new() -> Self {
        Self {
            runner: crate::domain::git::command::GitCommandRunner::new(),
        }
    }

    pub fn get_remote_url(&self, repo_path: &Path, name: &str) -> Result<String> {
        let output = self
            .runner
            .execute_in_dir(&["remote", "get-url", name], repo_path)?;

        if output.trim().is_empty() {
            Err(GitError::RemoteNotFound(name.to_string()))
        } else {
            Ok(output)
        }
    }

    pub fn list_remotes(&self, repo_path: &Path) -> Result<Vec<Remote>> {
        let remote_names_result = self.runner.execute_in_dir(&["remote"], repo_path);

        let remote_names: Vec<String> = match remote_names_result {
            Ok(output) => output
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            Err(_) => {
                return Ok(Vec::new());
            }
        };

        let mut remotes = Vec::new();
        for name in remote_names {
            if let Ok(url) = self.get_remote_url(repo_path, &name) {
                remotes.push(Remote {
                    name: name.to_string(),
                    url,
                });
            }
        }

        Ok(remotes)
    }
}

impl Default for RemoteManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::git::{GitProtocol, detect_protocol, extract_host_and_path};

    #[test]
    fn test_remote_parse_url_valid() {
        assert_eq!(
            detect_protocol("git@github.com:user/repo.git").unwrap(),
            GitProtocol::Ssh
        );
        assert_eq!(
            detect_protocol("https://github.com/user/repo.git").unwrap(),
            GitProtocol::Https
        );
        assert_eq!(
            detect_protocol("http://github.com/user/repo.git").unwrap(),
            GitProtocol::Http
        );
        assert_eq!(
            detect_protocol("git://github.com/user/repo.git").unwrap(),
            GitProtocol::Git
        );
    }

    #[test]
    fn test_remote_parse_url_invalid() {
        assert!(detect_protocol("").is_err());
        assert!(detect_protocol("invalid-url").is_err());
    }

    #[test]
    fn test_remote_extract_host_and_path() {
        let (host, path) = extract_host_and_path("git@github.com:user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");

        let (host, path) = extract_host_and_path("https://github.com/user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");
    }

    #[test]
    fn test_remote_extract_host_and_path_invalid() {
        assert!(extract_host_and_path("").is_none());
        assert!(extract_host_and_path("invalid-url").is_none());
    }

    #[test]
    fn test_remote_manager_new() {
        let _manager = RemoteManager::new();
    }

    #[test]
    fn test_remote_manager_default() {
        let _manager = RemoteManager::default();
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_remote_manager_list_remotes_empty() {
        let manager = RemoteManager::new();
        let temp_dir = tempdir::tempdir().unwrap();

        let repo_path = temp_dir.path();
        let _ = std::process::Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output();

        let remotes = manager.list_remotes(repo_path).unwrap();
        assert!(remotes.is_empty());
    }
}
