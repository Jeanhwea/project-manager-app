use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

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

fn default_max_depth() -> usize {
    3
}

fn default_skip_dirs() -> Vec<String> {
    vec![".venv".to_string()]
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            max_depth: default_max_depth(),
            skip_dirs: default_skip_dirs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    #[serde(default = "default_remote_rules")]
    pub rules: Vec<RemoteRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRule {
    #[serde(default)]
    pub hosts: Vec<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path_prefixes: Vec<String>,
    #[serde(default)]
    pub path_prefix_name: Option<String>,
}

fn default_remote_rules() -> Vec<RemoteRule> {
    vec![
        RemoteRule {
            hosts: vec!["github.com".to_string(), "githubfast.com".to_string()],
            name: "github".to_string(),
            path_prefixes: vec![],
            path_prefix_name: None,
        },
        RemoteRule {
            hosts: vec!["gitana.jeanhwea.io".to_string()],
            name: "gitana".to_string(),
            path_prefixes: vec![],
            path_prefix_name: None,
        },
        RemoteRule {
            hosts: vec!["gitee.com".to_string()],
            name: "gitee".to_string(),
            path_prefixes: vec![
                "red_8/".to_string(),
                "redtool/".to_string(),
                "red_base/".to_string(),
                "teampuzzle/".to_string(),
            ],
            path_prefix_name: Some("redinf".to_string()),
        },
    ]
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            rules: default_remote_rules(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    #[serde(default = "default_skip_push_hosts")]
    pub skip_push_hosts: Vec<String>,
}

fn default_skip_push_hosts() -> Vec<String> {
    vec![
        "github.com".to_string(),
        "githubfast.com".to_string(),
        "gitee.com".to_string(),
    ]
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            skip_push_hosts: default_skip_push_hosts(),
        }
    }
}

pub fn config_path() -> PathBuf {
    if let Ok(path) = env::var("PMA_CONFIG") {
        return PathBuf::from(path);
    }

    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    PathBuf::from(home).join(".pma.toml")
}

pub fn load() -> AppConfig {
    let path = config_path();
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

#[allow(dead_code)]
pub fn save(config: &AppConfig) -> anyhow::Result<()> {
    let path = config_path();
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn default_config_content() -> &'static str {
    r#"[repository]
# Maximum depth to search for git repositories
max_depth = 3
# Directory names to skip when searching
skip_dirs = [".venv"]

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
