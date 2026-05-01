//! Configuration schema module
//!
//! Defines the structures for `~/.pma/config.toml` and `~/.pma/gitlab.toml`.

use serde::{Deserialize, Serialize};

// ── config.toml ────────────────────────────────────────────────────

/// Main application configuration (`~/.pma/config.toml`)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub repository: RepositoryConfig,
    #[serde(default)]
    pub remote: RemoteConfig,
    #[serde(default)]
    pub sync: SyncConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default = "default_skip_dirs")]
    pub skip_dirs: Vec<String>,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            max_depth: default_max_depth(),
            skip_dirs: default_skip_dirs(),
        }
    }
}

fn default_max_depth() -> usize {
    3
}

fn default_skip_dirs() -> Vec<String> {
    vec![
        ".venv", "venv", "env", ".env", "node_modules", "__pycache__", ".tox", ".mypy_cache",
        ".pytest_cache", ".ruff_cache", "dist", "build", "target", ".gradle", ".idea", ".vscode",
        ".fleet", ".cache", ".next", ".nuxt", ".svelte-kit", ".angular", "bower_components",
        ".terraform", ".cargo",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    #[serde(default = "default_remote_rules")]
    pub rules: Vec<RemoteRule>,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            rules: default_remote_rules(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRule {
    #[serde(default)]
    pub hosts: Vec<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url_prefix: Option<String>,
    #[serde(default)]
    pub path_prefixes: Vec<String>,
    #[serde(default)]
    pub path_prefix_name: Option<String>,
}

fn default_remote_rules() -> Vec<RemoteRule> {
    vec![
        RemoteRule {
            hosts: vec!["github.com".into(), "githubfast.com".into()],
            name: "github".into(),
            url_prefix: None,
            path_prefixes: vec![],
            path_prefix_name: None,
        },
        RemoteRule {
            hosts: vec!["gitana.jeanhwea.io".into()],
            name: "gitana".into(),
            url_prefix: None,
            path_prefixes: vec![],
            path_prefix_name: None,
        },
        RemoteRule {
            hosts: vec!["gitee.com".into()],
            name: "gitee".into(),
            url_prefix: None,
            path_prefixes: vec![
                "red_8/".into(),
                "redtool/".into(),
                "red_base/".into(),
                "teampuzzle/".into(),
            ],
            path_prefix_name: Some("redinf".into()),
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    #[serde(default = "default_skip_push_hosts")]
    pub skip_push_hosts: Vec<String>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            skip_push_hosts: default_skip_push_hosts(),
        }
    }
}

fn default_skip_push_hosts() -> Vec<String> {
    vec![
        "github.com".into(),
        "githubfast.com".into(),
        "gitee.com".into(),
    ]
}

// ── gitlab.toml ────────────────────────────────────────────────────

/// GitLab configuration (`~/.pma/gitlab.toml`)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitLabConfig {
    #[serde(default)]
    pub servers: Vec<GitLabServer>,
}

/// A single GitLab server entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabServer {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub token: String,
    #[serde(default = "default_gitlab_protocol")]
    pub protocol: String,
}

fn default_gitlab_protocol() -> String {
    "ssh".to_string()
}

impl Default for GitLabServer {
    fn default() -> Self {
        Self {
            url: String::new(),
            token: String::new(),
            protocol: default_gitlab_protocol(),
        }
    }
}

// ── default file content (for `pma config init`) ───────────────────

pub fn default_config_content() -> &'static str {
    r#"[repository]
# Maximum depth to search for git repositories
max_depth = 3
# Directory names to skip when searching
skip_dirs = [".venv", "venv", "env", ".env", "node_modules", "__pycache__", ".tox", ".mypy_cache", ".pytest_cache", ".ruff_cache", "dist", "build", "target", ".gradle", ".idea", ".vscode", ".fleet", ".cache", ".next", ".nuxt", ".svelte-kit", ".angular", "bower_components", ".terraform", ".cargo"]

[[remote.rules]]
# Map host patterns to remote names
hosts = ["github.com", "githubfast.com"]
name = "github"

[[remote.rules]]
hosts = ["gitana.jeanhwea.io"]
name = "gitana"

[[remote.rules]]
hosts = ["gitee.com"]
name = "gitee"
# When the repository path starts with any of these prefixes, use a different name
path_prefixes = ["red_8/", "redtool/", "red_base/", "teampuzzle/"]
path_prefix_name = "redinf"

[sync]
# Skip pushing to these hosts when using HTTPS protocol
skip_push_hosts = ["github.com", "githubfast.com", "gitee.com"]
"#
}

pub fn default_gitlab_config_content() -> &'static str {
    r#"# GitLab server credentials
# Use `pma gitlab login` to add servers
#
# [[servers]]
# url = "https://gitlab.com"
# token = "glpat-xxxx"
# protocol = "ssh"
"#
}
