use super::{GitLabError, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

/// GitLab token 管理
pub struct AuthManager;

impl AuthManager {
    pub fn load_token() -> Result<Option<String>> {
        if let Ok(token) = env::var("GITLAB_TOKEN")
            && !token.trim().is_empty()
        {
            return Ok(Some(token));
        }

        if let Some(token) = Self::read_token_file(&Self::config_dir()?.join("gitlab_token"))? {
            return Ok(Some(token));
        }

        if let Some(home) = dirs::home_dir()
            && let Some(token) = Self::read_token_file(&home.join(".gitlab_token"))?
        {
            return Ok(Some(token));
        }

        Ok(None)
    }

    pub fn save_token(token: &str) -> Result<()> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir).map_err(|e| {
            GitLabError::Io(std::io::Error::new(
                e.kind(),
                format!("无法创建配置目录 {}: {}", dir.display(), e),
            ))
        })?;

        let path = dir.join("gitlab_token");
        fs::write(&path, token.trim()).map_err(|e| {
            GitLabError::Io(std::io::Error::new(
                e.kind(),
                format!("无法写入 {}: {}", path.display(), e),
            ))
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&path, perms).map_err(|e| {
                GitLabError::Io(std::io::Error::new(
                    e.kind(),
                    format!("无法设置权限 {}: {}", path.display(), e),
                ))
            })?;
        }

        Ok(())
    }

    pub fn delete_token() -> Result<()> {
        let path = Self::config_dir()?.join("gitlab_token");
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                GitLabError::Io(std::io::Error::new(
                    e.kind(),
                    format!("无法删除 {}: {}", path.display(), e),
                ))
            })?;
        }
        Ok(())
    }

    pub fn has_saved_token() -> Result<bool> {
        Ok(Self::config_dir()?.join("gitlab_token").exists())
    }

    fn config_dir() -> Result<PathBuf> {
        Ok(crate::domain::config::ConfigDir::dir())
    }

    fn read_token_file(path: &PathBuf) -> Result<Option<String>> {
        if !path.exists() {
            return Ok(None);
        }
        let token = fs::read_to_string(path)
            .map_err(|e| {
                GitLabError::Io(std::io::Error::new(
                    e.kind(),
                    format!("无法读取 {}: {}", path.display(), e),
                ))
            })?
            .trim()
            .to_string();
        if token.is_empty() {
            Ok(None)
        } else {
            Ok(Some(token))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_path() {
        assert!(AuthManager::config_dir().is_ok());
    }
}
