use crate::domain::git::{GitError, GitProtocol, Result};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Remote {
    pub name: String,
    pub url: String,
}

impl Remote {
    pub fn parse_url(url: &str) -> Result<GitProtocol> {
        let url = url.trim();
        if url.is_empty() {
            return Err(GitError::InvalidRemoteUrl("Empty URL".to_string()));
        }

        let protocol = if url.starts_with("git@") || url.starts_with("ssh://") {
            GitProtocol::Ssh
        } else if url.starts_with("https://") {
            GitProtocol::Https
        } else if url.starts_with("http://") {
            GitProtocol::Http
        } else if url.starts_with("git://") {
            GitProtocol::Git
        } else {
            if url.contains('@') && url.contains(':') {
                GitProtocol::Ssh
            } else {
                return Err(GitError::InvalidRemoteUrl(format!(
                    "Invalid URL format: {}",
                    url
                )));
            }
        };

        Ok(protocol)
    }

    pub fn extract_host_and_path(url: &str) -> Option<(String, String)> {
        let url = url.trim();
        if url.is_empty() {
            return None;
        }

        let _protocol = if url.starts_with("git@") || url.starts_with("ssh://") {
            GitProtocol::Ssh
        } else if url.starts_with("https://") {
            GitProtocol::Https
        } else if url.starts_with("http://") {
            GitProtocol::Http
        } else if url.starts_with("git://") {
            GitProtocol::Git
        } else {
            return None;
        };

        let (url, separator) = if url.starts_with("git@") {
            (url.replace("git@", ""), ':')
        } else if url.starts_with("ssh://") {
            let stripped = url.replace("ssh://", "");
            let stripped = if stripped.starts_with("git@") {
                stripped.replacen("git@", "", 1)
            } else {
                stripped
            };
            (stripped, '/')
        } else if url.starts_with("https://") {
            (url.replace("https://", ""), '/')
        } else if url.starts_with("http://") {
            (url.replace("http://", ""), '/')
        } else if url.starts_with("git://") {
            (url.replace("git://", ""), '/')
        } else {
            (url.to_string(), ':')
        };

        let parts: Vec<&str> = url.splitn(2, separator).collect();
        if parts.len() != 2 {
            return None;
        }

        let (host, path) = (parts[0].to_string(), parts[1].to_string());
        Some((host, path))
    }
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
    use tempfile::tempdir;

    #[test]
    fn test_remote_parse_url_valid() {
        assert_eq!(
            Remote::parse_url("git@github.com:user/repo.git").unwrap(),
            GitProtocol::Ssh
        );
        assert_eq!(
            Remote::parse_url("https://github.com/user/repo.git").unwrap(),
            GitProtocol::Https
        );
        assert_eq!(
            Remote::parse_url("http://github.com/user/repo.git").unwrap(),
            GitProtocol::Http
        );
        assert_eq!(
            Remote::parse_url("git://github.com/user/repo.git").unwrap(),
            GitProtocol::Git
        );
    }

    #[test]
    fn test_remote_parse_url_invalid() {
        assert!(Remote::parse_url("").is_err());
        assert!(Remote::parse_url("invalid-url").is_err());
    }

    #[test]
    fn test_remote_extract_host_and_path() {
        let (host, path) = Remote::extract_host_and_path("git@github.com:user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");

        let (host, path) =
            Remote::extract_host_and_path("https://github.com/user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");
    }

    #[test]
    fn test_remote_extract_host_and_path_invalid() {
        assert!(Remote::extract_host_and_path("").is_none());
        assert!(Remote::extract_host_and_path("invalid-url").is_none());
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
    fn test_remote_manager_list_remotes_empty() {
        let manager = RemoteManager::new();
        let temp_dir = tempdir().unwrap();

        let repo_path = temp_dir.path();
        let _ = std::process::Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output();

        let remotes = manager.list_remotes(repo_path).unwrap();
        assert!(remotes.is_empty());
    }
}
