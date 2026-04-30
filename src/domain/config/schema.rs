//! Configuration schema module
//!
//! This module defines the application configuration schema.

use serde::{Deserialize, Serialize};

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub git: GitConfig,
    pub gitlab: GitLabConfig,
    pub sync: SyncConfig,
    pub editor: EditorConfig,
}

/// Git configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    pub skip_push_hosts: Vec<String>,
    pub default_protocol: String,
}

/// GitLab configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabConfig {
    pub server: Option<String>,
    pub token: Option<String>,
    pub default_protocol: String,
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub auto_push: bool,
    pub auto_pull: bool,
}

/// Editor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub dry_run: bool,
    pub skip_push: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            git: GitConfig {
                skip_push_hosts: Vec::new(),
                default_protocol: "https".to_string(),
            },
            gitlab: GitLabConfig {
                server: None,
                token: None,
                default_protocol: "https".to_string(),
            },
            sync: SyncConfig {
                auto_push: false,
                auto_pull: true,
            },
            editor: EditorConfig {
                dry_run: false,
                skip_push: false,
            },
        }
    }
}