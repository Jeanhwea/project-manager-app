//! Configuration manager module
//!
//! This module implements configuration loading and saving with multi-source support.

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use super::{AppConfig, ConfigError, ConfigManager, ConfigSource, Result};

/// Multi-source configuration manager
pub struct MultiSourceConfigManager {
    config_file_path: Option<PathBuf>,
    env_prefix: String,
    cli_overrides: HashMap<String, String>,
}

impl MultiSourceConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        Self {
            config_file_path: Self::default_config_path(),
            env_prefix: "PMA_".to_string(),
            cli_overrides: HashMap::new(),
        }
    }
    
    /// Create a configuration manager with custom settings
    pub fn with_settings(config_file_path: Option<PathBuf>, env_prefix: String) -> Self {
        Self {
            config_file_path,
            env_prefix,
            cli_overrides: HashMap::new(),
        }
    }
    
    /// Set CLI overrides
    pub fn set_cli_overrides(&mut self, overrides: HashMap<String, String>) {
        self.cli_overrides = overrides;
    }
    
    /// Get the default configuration file path
    fn default_config_path() -> Option<PathBuf> {
        let mut path = dirs::config_dir()?;
        path.push("pma");
        path.push("config.json");
        Some(path)
    }
    
    /// Load configuration from file
    fn load_from_file(&self) -> Result<AppConfig> {
        let path = self.config_file_path.as_ref().ok_or_else(|| {
            ConfigError::FileNotFound("No configuration file path specified".to_string())
        })?;
        
        if !path.exists() {
            return Ok(AppConfig::default());
        }
        
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::Io(e))?;
        
        let config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| ConfigError::ParseError(e))?;
        
        Ok(config)
    }
    
    /// Load configuration from environment variables
    fn load_from_env(&self) -> Result<AppConfig> {
        let mut config = AppConfig::default();
        
        // Git configuration from environment
        if let Ok(skip_push_hosts) = env::var(format!("{}GIT_SKIP_PUSH_HOSTS", self.env_prefix)) {
            config.git.skip_push_hosts = skip_push_hosts
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }
        
        if let Ok(default_protocol) = env::var(format!("{}GIT_DEFAULT_PROTOCOL", self.env_prefix)) {
            config.git.default_protocol = default_protocol;
        }
        
        // GitLab configuration from environment
        if let Ok(server) = env::var(format!("{}GITLAB_SERVER", self.env_prefix)) {
            config.gitlab.server = Some(server);
        }
        
        if let Ok(token) = env::var(format!("{}GITLAB_TOKEN", self.env_prefix)) {
            config.gitlab.token = Some(token);
        }
        
        if let Ok(default_protocol) = env::var(format!("{}GITLAB_DEFAULT_PROTOCOL", self.env_prefix)) {
            config.gitlab.default_protocol = default_protocol;
        }
        
        // Sync configuration from environment
        if let Ok(auto_push) = env::var(format!("{}SYNC_AUTO_PUSH", self.env_prefix)) {
            config.sync.auto_push = auto_push.parse().unwrap_or(false);
        }
        
        if let Ok(auto_pull) = env::var(format!("{}SYNC_AUTO_PULL", self.env_prefix)) {
            config.sync.auto_pull = auto_pull.parse().unwrap_or(true);
        }
        
        // Editor configuration from environment
        if let Ok(dry_run) = env::var(format!("{}EDITOR_DRY_RUN", self.env_prefix)) {
            config.editor.dry_run = dry_run.parse().unwrap_or(false);
        }
        
        if let Ok(skip_push) = env::var(format!("{}EDITOR_SKIP_PUSH", self.env_prefix)) {
            config.editor.skip_push = skip_push.parse().unwrap_or(false);
        }
        
        Ok(config)
    }
    
    /// Apply CLI overrides to configuration
    fn apply_cli_overrides(&self, mut config: AppConfig) -> AppConfig {
        for (key, value) in &self.cli_overrides {
            match key.as_str() {
                "git.skip_push_hosts" => {
                    config.git.skip_push_hosts = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect();
                }
                "git.default_protocol" => {
                    config.git.default_protocol = value.clone();
                }
                "gitlab.server" => {
                    config.gitlab.server = Some(value.clone());
                }
                "gitlab.token" => {
                    config.gitlab.token = Some(value.clone());
                }
                "gitlab.default_protocol" => {
                    config.gitlab.default_protocol = value.clone();
                }
                "sync.auto_push" => {
                    config.sync.auto_push = value.parse().unwrap_or(false);
                }
                "sync.auto_pull" => {
                    config.sync.auto_pull = value.parse().unwrap_or(true);
                }
                "editor.dry_run" => {
                    config.editor.dry_run = value.parse().unwrap_or(false);
                }
                "editor.skip_push" => {
                    config.editor.skip_push = value.parse().unwrap_or(false);
                }
                _ => {
                    // Unknown CLI override, ignore
                }
            }
        }
        
        config
    }
    
    /// Merge configurations with proper precedence
    fn merge_configurations(&self, file_config: AppConfig, env_config: AppConfig) -> AppConfig {
        let mut merged = AppConfig::default();
        
        // Git configuration merging
        merged.git.skip_push_hosts = if !env_config.git.skip_push_hosts.is_empty() {
            env_config.git.skip_push_hosts
        } else {
            file_config.git.skip_push_hosts
        };
        
        merged.git.default_protocol = if env_config.git.default_protocol != "https" {
            env_config.git.default_protocol
        } else {
            file_config.git.default_protocol
        };
        
        // GitLab configuration merging
        merged.gitlab.server = env_config.gitlab.server
            .or(file_config.gitlab.server);
        
        merged.gitlab.token = env_config.gitlab.token
            .or(file_config.gitlab.token);
        
        merged.gitlab.default_protocol = if env_config.gitlab.default_protocol != "https" {
            env_config.gitlab.default_protocol
        } else {
            file_config.gitlab.default_protocol
        };
        
        // Sync configuration merging
        merged.sync.auto_push = env_config.sync.auto_push || file_config.sync.auto_push;
        merged.sync.auto_pull = env_config.sync.auto_pull && file_config.sync.auto_pull;
        
        // Editor configuration merging
        merged.editor.dry_run = env_config.editor.dry_run || file_config.editor.dry_run;
        merged.editor.skip_push = env_config.editor.skip_push || file_config.editor.skip_push;
        
        merged
    }
}

impl ConfigManager for MultiSourceConfigManager {
    fn load() -> Result<AppConfig> {
        let manager = Self::new();
        manager.load_with_manager()
    }
    
    fn save(&self, config: &AppConfig) -> Result<()> {
        let path = self.config_file_path.as_ref().ok_or_else(|| {
            ConfigError::FileNotFound("No configuration file path specified".to_string())
        })?;
        
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ConfigError::Io(e))?;
        }
        
        let content = serde_json::to_string_pretty(config)
            .map_err(|e| ConfigError::ParseError(e))?;
        
        std::fs::write(path, content)
            .map_err(|e| ConfigError::Io(e))?;
        
        Ok(())
    }
    
    fn get_source(&self) -> ConfigSource {
        // Return the highest priority source that has been used
        if !self.cli_overrides.is_empty() {
            ConfigSource::Cli
        } else {
            // Check if environment variables are set
            let env_config = self.load_from_env();
            if env_config.is_ok() {
                // Check if any environment variables actually affected the config
                let default_config = AppConfig::default();
                let env_config = env_config.unwrap();
                
                if env_config.git.skip_push_hosts != default_config.git.skip_push_hosts ||
                   env_config.git.default_protocol != default_config.git.default_protocol ||
                   env_config.gitlab.server != default_config.gitlab.server ||
                   env_config.gitlab.token != default_config.gitlab.token ||
                   env_config.gitlab.default_protocol != default_config.gitlab.default_protocol ||
                   env_config.sync.auto_push != default_config.sync.auto_push ||
                   env_config.sync.auto_pull != default_config.sync.auto_pull ||
                   env_config.editor.dry_run != default_config.editor.dry_run ||
                   env_config.editor.skip_push != default_config.editor.skip_push {
                    return ConfigSource::Environment;
                }
            }
            
            ConfigSource::File
        }
    }
}

impl MultiSourceConfigManager {
    /// Load configuration using this manager instance
    pub fn load_with_manager(&self) -> Result<AppConfig> {
        // Load from file
        let file_config = self.load_from_file()?;
        
        // Load from environment
        let env_config = self.load_from_env()?;
        
        // Merge file and environment configurations
        let merged_config = self.merge_configurations(file_config, env_config);
        
        // Apply CLI overrides (highest precedence)
        let final_config = self.apply_cli_overrides(merged_config);
        
        Ok(final_config)
    }
}

/// Type-safe configuration accessor
pub struct ConfigAccessor {
    config: AppConfig,
}

impl ConfigAccessor {
    /// Create a new configuration accessor
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }
    
    /// Get Git configuration
    pub fn git(&self) -> &GitConfig {
        &self.config.git
    }
    
    /// Get GitLab configuration
    pub fn gitlab(&self) -> &GitLabConfig {
        &self.config.gitlab
    }
    
    /// Get Sync configuration
    pub fn sync(&self) -> &SyncConfig {
        &self.config.sync
    }
    
    /// Get Editor configuration
    pub fn editor(&self) -> &EditorConfig {
        &self.config.editor
    }
    
    /// Get the entire configuration
    pub fn inner(&self) -> &AppConfig {
        &self.config
    }
    
    /// Update configuration
    pub fn update(&mut self, config: AppConfig) {
        self.config = config;
    }
}

// Re-export types from schema for convenience
pub use super::schema::{GitConfig, GitLabConfig, SyncConfig, EditorConfig};

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::tempdir;
    
    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.git.default_protocol, "https");
        assert_eq!(config.gitlab.default_protocol, "https");
        assert!(!config.sync.auto_push);
        assert!(config.sync.auto_pull);
        assert!(!config.editor.dry_run);
        assert!(!config.editor.skip_push);
    }
    
    #[test]
    fn test_config_accessor() {
        let config = AppConfig::default();
        let accessor = ConfigAccessor::new(config);
        
        assert_eq!(accessor.git().default_protocol, "https");
        assert_eq!(accessor.gitlab().default_protocol, "https");
        assert!(!accessor.sync().auto_push);
        assert!(accessor.sync().auto_pull);
        assert!(!accessor.editor().dry_run);
        assert!(!accessor.editor().skip_push);
    }
    
    #[test]
    fn test_file_config_loading() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        
        let config = AppConfig {
            git: GitConfig {
                skip_push_hosts: vec!["example.com".to_string()],
                default_protocol: "ssh".to_string(),
            },
            gitlab: GitLabConfig {
                server: Some("https://gitlab.example.com".to_string()),
                token: Some("test-token".to_string()),
                default_protocol: "ssh".to_string(),
            },
            sync: SyncConfig {
                auto_push: true,
                auto_pull: false,
            },
            editor: EditorConfig {
                dry_run: true,
                skip_push: true,
            },
        };
        
        // Write config to file
        let content = serde_json::to_string_pretty(&config).unwrap();
        std::fs::write(&config_path, content).unwrap();
        
        // Create manager with custom path
        let manager = MultiSourceConfigManager::with_settings(
            Some(config_path),
            "TEST_".to_string(),
        );
        
        // Load config
        let loaded_config = manager.load_from_file().unwrap();
        
        assert_eq!(loaded_config.git.skip_push_hosts, vec!["example.com"]);
        assert_eq!(loaded_config.git.default_protocol, "ssh");
        assert_eq!(loaded_config.gitlab.server, Some("https://gitlab.example.com".to_string()));
        assert_eq!(loaded_config.gitlab.token, Some("test-token".to_string()));
        assert_eq!(loaded_config.gitlab.default_protocol, "ssh");
        assert!(loaded_config.sync.auto_push);
        assert!(!loaded_config.sync.auto_pull);
        assert!(loaded_config.editor.dry_run);
        assert!(loaded_config.editor.skip_push);
    }
    
    #[test]
    fn test_env_config_loading() {
        // Set environment variables
        unsafe {
            env::set_var("TEST_GIT_SKIP_PUSH_HOSTS", "host1.com,host2.com");
            env::set_var("TEST_GIT_DEFAULT_PROTOCOL", "ssh");
            env::set_var("TEST_GITLAB_SERVER", "https://test.gitlab.com");
            env::set_var("TEST_GITLAB_TOKEN", "env-token");
            env::set_var("TEST_GITLAB_DEFAULT_PROTOCOL", "ssh");
            env::set_var("TEST_SYNC_AUTO_PUSH", "true");
            env::set_var("TEST_SYNC_AUTO_PULL", "false");
            env::set_var("TEST_EDITOR_DRY_RUN", "true");
            env::set_var("TEST_EDITOR_SKIP_PUSH", "true");
        }
        
        let manager = MultiSourceConfigManager::with_settings(
            None,
            "TEST_".to_string(),
        );
        
        let env_config = manager.load_from_env().unwrap();
        
        assert_eq!(env_config.git.skip_push_hosts, vec!["host1.com", "host2.com"]);
        assert_eq!(env_config.git.default_protocol, "ssh");
        assert_eq!(env_config.gitlab.server, Some("https://test.gitlab.com".to_string()));
        assert_eq!(env_config.gitlab.token, Some("env-token".to_string()));
        assert_eq!(env_config.gitlab.default_protocol, "ssh");
        assert!(env_config.sync.auto_push);
        assert!(!env_config.sync.auto_pull);
        assert!(env_config.editor.dry_run);
        assert!(env_config.editor.skip_push);
        
        // Clean up environment variables
        unsafe {
            env::remove_var("TEST_GIT_SKIP_PUSH_HOSTS");
            env::remove_var("TEST_GIT_DEFAULT_PROTOCOL");
            env::remove_var("TEST_GITLAB_SERVER");
            env::remove_var("TEST_GITLAB_TOKEN");
            env::remove_var("TEST_GITLAB_DEFAULT_PROTOCOL");
            env::remove_var("TEST_SYNC_AUTO_PUSH");
            env::remove_var("TEST_SYNC_AUTO_PULL");
            env::remove_var("TEST_EDITOR_DRY_RUN");
            env::remove_var("TEST_EDITOR_SKIP_PUSH");
        }
    }
    
    #[test]
    fn test_cli_overrides() {
        let mut manager = MultiSourceConfigManager::new();
        
        let mut overrides = HashMap::new();
        overrides.insert("git.default_protocol".to_string(), "custom".to_string());
        overrides.insert("gitlab.server".to_string(), "https://cli.gitlab.com".to_string());
        overrides.insert("sync.auto_push".to_string(), "true".to_string());
        
        manager.set_cli_overrides(overrides);
        
        let config = AppConfig::default();
        let overridden_config = manager.apply_cli_overrides(config);
        
        assert_eq!(overridden_config.git.default_protocol, "custom");
        assert_eq!(overridden_config.gitlab.server, Some("https://cli.gitlab.com".to_string()));
        assert!(overridden_config.sync.auto_push);
    }
    
    #[test]
    fn test_config_merging() {
        let file_config = AppConfig {
            git: GitConfig {
                skip_push_hosts: vec!["file-host.com".to_string()],
                default_protocol: "file-ssh".to_string(),
            },
            gitlab: GitLabConfig {
                server: Some("https://file.gitlab.com".to_string()),
                token: Some("file-token".to_string()),
                default_protocol: "file-ssh".to_string(),
            },
            sync: SyncConfig {
                auto_push: false,
                auto_pull: true,
            },
            editor: EditorConfig {
                dry_run: false,
                skip_push: false,
            },
        };
        
        let env_config = AppConfig {
            git: GitConfig {
                skip_push_hosts: vec!["env-host.com".to_string()],
                default_protocol: "env-ssh".to_string(),
            },
            gitlab: GitLabConfig {
                server: Some("https://env.gitlab.com".to_string()),
                token: Some("env-token".to_string()),
                default_protocol: "env-ssh".to_string(),
            },
            sync: SyncConfig {
                auto_push: true,
                auto_pull: false,
            },
            editor: EditorConfig {
                dry_run: true,
                skip_push: true,
            },
        };
        
        let manager = MultiSourceConfigManager::new();
        let merged = manager.merge_configurations(file_config, env_config);
        
        // Environment should take precedence
        assert_eq!(merged.git.skip_push_hosts, vec!["env-host.com"]);
        assert_eq!(merged.git.default_protocol, "env-ssh");
        assert_eq!(merged.gitlab.server, Some("https://env.gitlab.com".to_string()));
        assert_eq!(merged.gitlab.token, Some("env-token".to_string()));
        assert_eq!(merged.gitlab.default_protocol, "env-ssh");
        assert!(merged.sync.auto_push); // env true OR file false = true
        assert!(!merged.sync.auto_pull); // env false AND file true = false
        assert!(merged.editor.dry_run); // env true OR file false = true
        assert!(merged.editor.skip_push); // env true OR file false = true
    }
}