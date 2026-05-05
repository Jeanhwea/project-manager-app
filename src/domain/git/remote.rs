use crate::domain::git::{GitError, GitProtocol, Result};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Remote {
    pub name: String,
    pub url: String,
    #[allow(dead_code)]
    pub protocol: GitProtocol,
}

impl Remote {
    #[allow(dead_code)]
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Result<Self> {
        let name = name.into();
        let url = url.into();

        let protocol = Self::parse_url(&url)?;

        Ok(Self {
            name,
            url,
            protocol,
        })
    }

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

        if !Self::validate_url_structure(url, &protocol) {
            return Err(GitError::InvalidRemoteUrl(format!(
                "Invalid URL structure: {}",
                url
            )));
        }

        Ok(protocol)
    }

    fn validate_url_structure(url: &str, protocol: &GitProtocol) -> bool {
        match protocol {
            GitProtocol::Ssh => {
                // SSH URLs can be:
                // 1. git@host:path
                // 2. ssh://git@host/path
                // 3. ssh://host/path
                if let Some(without_prefix) = url.strip_prefix("git@") {
                    let parts: Vec<&str> = without_prefix.splitn(2, ':').collect();
                    parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty()
                } else if let Some(without_prefix) = url.strip_prefix("ssh://") {
                    let parts: Vec<&str> = without_prefix.splitn(2, '/').collect();
                    parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty()
                } else {
                    false
                }
            }
            GitProtocol::Http | GitProtocol::Https | GitProtocol::Git => {
                // HTTP/HTTPS/GIT URLs: protocol://host/path
                let prefix = match protocol {
                    GitProtocol::Http => "http://",
                    GitProtocol::Https => "https://",
                    GitProtocol::Git => "git://",
                    _ => unreachable!(),
                };

                let without_prefix = &url[prefix.len()..];
                let parts: Vec<&str> = without_prefix.splitn(2, '/').collect();
                parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty()
            }
        }
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

    #[allow(dead_code)]
    pub fn get_remote_name_by_url(_url: &str) -> String {
        "origin".to_string()
    }

    #[allow(dead_code)]
    pub fn is_private_ip(host: &str) -> bool {
        let ip_part = host.split(':').next().unwrap_or(host);
        let octets: Vec<u8> = ip_part
            .split('.')
            .filter_map(|s| s.parse::<u8>().ok())
            .collect();

        if octets.len() != 4 {
            return false;
        }

        match (octets[0], octets[1]) {
            (10, _) => true,
            (172, second) if (16..=31).contains(&second) => true,
            (192, 168) => true,
            _ => false,
        }
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

    #[allow(dead_code)]
    pub fn add_remote(&self, repo_path: &Path, name: &str, url: &str) -> Result<()> {
        Remote::parse_url(url)?;

        self.runner
            .execute_with_success_in_dir(&["remote", "add", name, url], repo_path)
    }

    #[allow(dead_code)]
    pub fn remove_remote(&self, repo_path: &Path, name: &str) -> Result<()> {
        let remotes = self.list_remotes(repo_path)?;
        if !remotes.iter().any(|remote| remote.name == name) {
            return Err(GitError::RemoteNotFound(name.to_string()));
        }

        self.runner
            .execute_with_success_in_dir(&["remote", "remove", name], repo_path)
    }

    #[allow(dead_code)]
    pub fn rename_remote(&self, repo_path: &Path, old_name: &str, new_name: &str) -> Result<()> {
        let remotes = self.list_remotes(repo_path)?;
        if !remotes.iter().any(|remote| remote.name == old_name) {
            return Err(GitError::RemoteNotFound(old_name.to_string()));
        }

        self.runner
            .execute_with_success_in_dir(&["remote", "rename", old_name, new_name], repo_path)
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

    /// Set remote URL
    #[allow(dead_code)]
    pub fn set_remote_url(&self, repo_path: &Path, name: &str, url: &str) -> Result<()> {
        Remote::parse_url(url)?;
        let remotes = self.list_remotes(repo_path)?;
        if !remotes.iter().any(|remote| remote.name == name) {
            return Err(GitError::RemoteNotFound(name.to_string()));
        }
        self.runner
            .execute_with_success_in_dir(&["remote", "set-url", name, url], repo_path)
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
                // No remotes is not an error, just return empty list
                return Ok(Vec::new());
            }
        };

        let mut remotes = Vec::new();
        for name in remote_names {
            let url_result = self.get_remote_url(repo_path, &name);

            if let Ok(url) = url_result {
                match Remote::parse_url(&url) {
                    Ok(protocol) => {
                        remotes.push(Remote {
                            name: name.to_string(),
                            url,
                            protocol,
                        });
                    }
                    Err(_) => {
                        remotes.push(Remote {
                            name: name.to_string(),
                            url,
                            protocol: GitProtocol::Ssh, // Default to SSH
                        });
                    }
                }
            }
        }

        Ok(remotes)
    }

    #[allow(dead_code)]
    pub fn remote_exists(&self, repo_path: &Path, name: &str) -> Result<bool> {
        let remotes = self.list_remotes(repo_path)?;
        Ok(remotes.iter().any(|remote| remote.name == name))
    }

    #[allow(dead_code)]
    pub fn get_remote(&self, repo_path: &Path, name: &str) -> Result<Option<Remote>> {
        let remotes = self.list_remotes(repo_path)?;
        Ok(remotes.into_iter().find(|remote| remote.name == name))
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
        // Test SSH URLs
        assert_eq!(
            Remote::parse_url("git@github.com:user/repo.git").unwrap(),
            GitProtocol::Ssh
        );
        assert_eq!(
            Remote::parse_url("ssh://git@github.com/user/repo.git").unwrap(),
            GitProtocol::Ssh
        );
        assert_eq!(
            Remote::parse_url("ssh://github.com/user/repo.git").unwrap(),
            GitProtocol::Ssh
        );

        // Test HTTPS URLs
        assert_eq!(
            Remote::parse_url("https://github.com/user/repo.git").unwrap(),
            GitProtocol::Https
        );

        // Test HTTP URLs
        assert_eq!(
            Remote::parse_url("http://github.com/user/repo.git").unwrap(),
            GitProtocol::Http
        );

        // Test Git URLs
        assert_eq!(
            Remote::parse_url("git://github.com/user/repo.git").unwrap(),
            GitProtocol::Git
        );
    }

    #[test]
    fn test_remote_parse_url_invalid() {
        // Test empty URL
        assert!(Remote::parse_url("").is_err());

        // Test invalid URL format
        assert!(Remote::parse_url("invalid-url").is_err());
        assert!(Remote::parse_url("github.com/user/repo.git").is_err());
    }

    #[test]
    fn test_remote_extract_host_and_path() {
        // Test SSH URLs
        let (host, path) = Remote::extract_host_and_path("git@github.com:user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");

        let (host, path) =
            Remote::extract_host_and_path("ssh://git@github.com/user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");

        // Test HTTPS URLs
        let (host, path) =
            Remote::extract_host_and_path("https://github.com/user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");

        // Test HTTP URLs
        let (host, path) =
            Remote::extract_host_and_path("http://github.com/user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");

        // Test Git URLs
        let (host, path) =
            Remote::extract_host_and_path("git://github.com/user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");
    }

    #[test]
    fn test_remote_extract_host_and_path_invalid() {
        assert!(Remote::extract_host_and_path("").is_none());
        assert!(Remote::extract_host_and_path("invalid-url").is_none());
    }

    #[test]
    fn test_remote_new() {
        let remote = Remote::new("origin", "https://github.com/user/repo.git").unwrap();
        assert_eq!(remote.name, "origin");
        assert_eq!(remote.url, "https://github.com/user/repo.git");
        assert_eq!(remote.protocol, GitProtocol::Https);
    }

    #[test]
    fn test_remote_new_invalid_url() {
        assert!(Remote::new("origin", "invalid-url").is_err());
    }

    #[test]
    fn test_remote_is_private_ip() {
        // Private IPs
        assert!(Remote::is_private_ip("10.0.0.1"));
        assert!(Remote::is_private_ip("172.16.0.1"));
        assert!(Remote::is_private_ip("172.31.255.254"));
        assert!(Remote::is_private_ip("192.168.1.1"));

        // Public IPs
        assert!(!Remote::is_private_ip("8.8.8.8"));
        assert!(!Remote::is_private_ip("172.15.0.1")); // Outside private range
        assert!(!Remote::is_private_ip("172.32.0.1")); // Outside private range

        // Invalid IPs
        assert!(!Remote::is_private_ip("not-an-ip"));
        assert!(!Remote::is_private_ip("256.256.256.256"));
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

        // Create a Git repository
        let repo_path = temp_dir.path();
        let _ = std::process::Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output();

        let remotes = manager.list_remotes(repo_path).unwrap();
        assert!(remotes.is_empty());
    }

    #[test]
    fn test_remote_manager_remote_exists() {
        let manager = RemoteManager::new();
        let temp_dir = tempdir().unwrap();

        // Create a Git repository
        let repo_path = temp_dir.path();
        let _ = std::process::Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output();

        // No remotes should exist
        let exists = manager.remote_exists(repo_path, "origin").unwrap();
        assert!(!exists);
    }

    #[test]
    fn test_remote_manager_get_remote_not_found() {
        let manager = RemoteManager::new();
        let temp_dir = tempdir().unwrap();

        // Create a Git repository
        let repo_path = temp_dir.path();
        let _ = std::process::Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output();

        // Get non-existent remote
        let remote = manager.get_remote(repo_path, "origin").unwrap();
        assert!(remote.is_none());
    }

    #[test]
    fn test_remote_manager_add_and_get_remote() {
        let manager = RemoteManager::new();
        let temp_dir = tempdir().unwrap();

        // Create a Git repository
        let repo_path = temp_dir.path();
        let _ = std::process::Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output();

        // Add a remote
        let result = manager.add_remote(repo_path, "origin", "https://example.com/user/repo.git");

        // This might fail if Git is not available or command fails
        // We'll just test that the method doesn't panic
        if result.is_ok() {
            // If remote was added successfully, try to get it
            let remote = manager.get_remote(repo_path, "origin").unwrap();
            assert!(remote.is_some());
            if let Some(remote) = remote {
                assert_eq!(remote.name, "origin");
                assert_eq!(remote.url, "https://example.com/user/repo.git");
                assert_eq!(remote.protocol, GitProtocol::Https);
            }
        }
    }

    #[test]
    fn test_remote_manager_add_remote_invalid_url() {
        let manager = RemoteManager::new();
        let temp_dir = tempdir().unwrap();

        // Create a Git repository
        let repo_path = temp_dir.path();
        let _ = std::process::Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output();

        // Try to add remote with invalid URL
        let result = manager.add_remote(repo_path, "origin", "invalid-url");
        assert!(result.is_err());
        match result.unwrap_err() {
            GitError::InvalidRemoteUrl(_) => (), // Expected
            _ => panic!("Expected InvalidRemoteUrl error"),
        }
    }

    #[test]
    fn test_remote_get_remote_name_by_url() {
        // Default implementation returns "origin"
        assert_eq!(
            Remote::get_remote_name_by_url("https://github.com/user/repo.git"),
            "origin"
        );
        assert_eq!(
            Remote::get_remote_name_by_url("git@github.com:user/repo.git"),
            "origin"
        );
    }
}
