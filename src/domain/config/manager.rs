use super::schema::{AppConfig, GitLabConfig};
use super::{ConfigError, Result};
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct ConfigDir;

impl ConfigDir {
    pub fn dir() -> PathBuf {
        if let Ok(path) = std::env::var("PMA_CONFIG_DIR") {
            return PathBuf::from(path);
        }

        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());

        PathBuf::from(home).join(".pma")
    }

    pub fn config_path() -> PathBuf {
        Self::dir().join("config.toml")
    }

    pub fn gitlab_path() -> PathBuf {
        Self::dir().join("gitlab.toml")
    }

    pub fn ensure_dir() -> Result<()> {
        let dir = Self::dir();
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }
        Ok(())
    }

    pub fn load_config() -> AppConfig {
        static CONFIG: OnceLock<AppConfig> = OnceLock::new();
        CONFIG.get_or_init(Self::read_config_file).clone()
    }

    fn read_config_file() -> AppConfig {
        let path = Self::config_path();
        if !path.exists() {
            return AppConfig::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_else(|e| {
                eprintln!("警告: 配置文件解析失败 ({}): {}", path.display(), e);
                eprintln!("使用默认配置");
                AppConfig::default()
            }),
            Err(e) => {
                eprintln!("警告: 无法读取配置文件 ({}): {}", path.display(), e);
                AppConfig::default()
            }
        }
    }

    pub fn load_gitlab() -> GitLabConfig {
        let path = Self::gitlab_path();
        if !path.exists() {
            return GitLabConfig::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_else(|e| {
                eprintln!("警告: GitLab 配置文件解析失败 ({}): {}", path.display(), e);
                GitLabConfig::default()
            }),
            Err(e) => {
                eprintln!("警告: 无法读取 GitLab 配置文件 ({}): {}", path.display(), e);
                GitLabConfig::default()
            }
        }
    }

    pub fn save_gitlab(config: &GitLabConfig) -> Result<()> {
        Self::ensure_dir()?;
        let path = Self::gitlab_path();
        let content = toml::to_string_pretty(config)
            .map_err(|e| ConfigError::ParseError(format!("{}", e)))?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ConfigDir::load_config();
        assert_eq!(config.repository.max_depth, 3);
        assert!(!config.repository.skip_dirs.is_empty());
        assert!(!config.sync.skip_push_hosts.is_empty());
    }

    #[test]
    fn test_default_gitlab_config() {
        let config = GitLabConfig::default();
        assert!(config.servers.is_empty());
    }
}
