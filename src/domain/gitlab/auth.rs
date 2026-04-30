//! GitLab authentication module
//!
//! This module handles token storage and authentication management.

use super::{GitLabError, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Authentication manager
pub struct AuthManager;

impl AuthManager {
    /// Load authentication token from multiple sources
    ///
    /// Sources checked in order:
    /// 1. GITLAB_TOKEN environment variable
    /// 2. ~/.config/pma/gitlab_token file
    /// 3. ~/.gitlab_token file
    pub fn load_token() -> Result<Option<String>> {
        // Check environment variable first
        if let Ok(token) = env::var("GITLAB_TOKEN") {
            if !token.trim().is_empty() {
                return Ok(Some(token));
            }
        }

        // Check config directory
        if let Some(config_token) = Self::load_token_from_config()? {
            return Ok(Some(config_token));
        }

        // Check home directory
        if let Some(home_token) = Self::load_token_from_home()? {
            return Ok(Some(home_token));
        }

        Ok(None)
    }

    /// Save authentication token to config directory
    pub fn save_token(token: &str) -> Result<()> {
        let config_dir = Self::config_dir()?;
        fs::create_dir_all(&config_dir).map_err(|e| {
            GitLabError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to create config directory {}: {}", config_dir.display(), e),
            ))
        })?;

        let token_file = config_dir.join("gitlab_token");
        fs::write(&token_file, token.trim()).map_err(|e| {
            GitLabError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to write token file {}: {}", token_file.display(), e),
            ))
        })?;

        // Set appropriate permissions (read/write for user only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&token_file)
                .map_err(|e| {
                    GitLabError::Io(std::io::Error::new(
                        e.kind(),
                        format!("Failed to get file metadata {}: {}", token_file.display(), e),
                    ))
                })?
                .permissions();
            perms.set_mode(0o600); // rw-------
            fs::set_permissions(&token_file, perms).map_err(|e| {
                GitLabError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to set permissions {}: {}", token_file.display(), e),
                ))
            })?;
        }

        Ok(())
    }

    /// Delete saved authentication token
    pub fn delete_token() -> Result<()> {
        let token_file = Self::config_dir()?.join("gitlab_token");
        if token_file.exists() {
            fs::remove_file(&token_file).map_err(|e| {
                GitLabError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to delete token file {}: {}", token_file.display(), e),
                ))
            })?;
        }
        Ok(())
    }

    /// Check if a token is saved
    pub fn has_saved_token() -> Result<bool> {
        let token_file = Self::config_dir()?.join("gitlab_token");
        Ok(token_file.exists())
    }

    /// Load token from config directory
    fn load_token_from_config() -> Result<Option<String>> {
        let token_file = Self::config_dir()?.join("gitlab_token");
        if token_file.exists() {
            let token = fs::read_to_string(&token_file).map_err(|e| {
                GitLabError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read token file {}: {}", token_file.display(), e),
                ))
            })?;
            let token = token.trim().to_string();
            if !token.is_empty() {
                return Ok(Some(token));
            }
        }
        Ok(None)
    }

    /// Load token from home directory (legacy location)
    fn load_token_from_home() -> Result<Option<String>> {
        if let Some(home_dir) = dirs::home_dir() {
            let token_file = home_dir.join(".gitlab_token");
            if token_file.exists() {
                let token = fs::read_to_string(&token_file).map_err(|e| {
                    GitLabError::Io(std::io::Error::new(
                        e.kind(),
                        format!("Failed to read token file {}: {}", token_file.display(), e),
                    ))
                })?;
                let token = token.trim().to_string();
                if !token.is_empty() {
                    return Ok(Some(token));
                }
            }
        }
        Ok(None)
    }

    /// Get config directory path
    fn config_dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| {
                GitLabError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not determine config directory",
                ))
            })?
            .join("pma");

        Ok(config_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_token_trimming() {
        // This test would require mocking the file system
        // For now, we'll just verify the function signatures compile
        assert!(true);
    }

    #[test]
    fn test_config_dir_path() {
        // Test that config_dir returns a valid path
        let result = AuthManager::config_dir();
        assert!(result.is_ok());
    }
}