use colored::Colorize;
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
    vec![
        ".venv".to_string(),
        "venv".to_string(),
        "env".to_string(),
        ".env".to_string(),
        "node_modules".to_string(),
        "__pycache__".to_string(),
        ".tox".to_string(),
        ".mypy_cache".to_string(),
        ".pytest_cache".to_string(),
        ".ruff_cache".to_string(),
        "dist".to_string(),
        "build".to_string(),
        "target".to_string(),
        ".gradle".to_string(),
        ".idea".to_string(),
        ".vscode".to_string(),
        ".fleet".to_string(),
        ".cache".to_string(),
        ".next".to_string(),
        ".nuxt".to_string(),
        ".svelte-kit".to_string(),
        ".angular".to_string(),
        "bower_components".to_string(),
        ".terraform".to_string(),
        ".cargo".to_string(),
    ]
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
    pub url_prefix: Option<String>,
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
            url_prefix: None,
            path_prefixes: vec![],
            path_prefix_name: None,
        },
        RemoteRule {
            hosts: vec!["gitana.jeanhwea.io".to_string()],
            name: "gitana".to_string(),
            url_prefix: None,
            path_prefixes: vec![],
            path_prefix_name: None,
        },
        RemoteRule {
            hosts: vec!["gitee.com".to_string()],
            name: "gitee".to_string(),
            url_prefix: None,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitLabConfig {
    #[serde(default)]
    pub servers: Vec<GitLabServer>,
}

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

pub fn config_dir() -> PathBuf {
    if let Ok(path) = env::var("PMA_CONFIG_DIR") {
        return PathBuf::from(path);
    }

    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    PathBuf::from(home).join(".pma")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn gitlab_config_path() -> PathBuf {
    config_dir().join("gitlab.toml")
}

fn legacy_config_path() -> PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    PathBuf::from(home).join(".pma.toml")
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct LegacyAppConfig {
    #[serde(default)]
    repository: RepositoryConfig,
    #[serde(default)]
    remote: RemoteConfig,
    #[serde(default)]
    sync: SyncConfig,
    #[serde(default)]
    gitlab: GitLabConfig,
}

fn migrate_legacy_config() {
    let legacy = legacy_config_path();
    if !legacy.exists() {
        return;
    }

    let dir = config_dir();
    if dir.exists() {
        return;
    }

    if let Ok(content) = std::fs::read_to_string(&legacy) {
        let legacy_cfg: LegacyAppConfig = match toml::from_str(&content) {
            Ok(c) => c,
            Err(_) => return,
        };

        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("警告: 无法创建配置目录 ({}): {}", dir.display(), e);
            return;
        }

        let main_cfg = AppConfig {
            repository: legacy_cfg.repository,
            remote: legacy_cfg.remote,
            sync: legacy_cfg.sync,
        };

        if let Ok(toml_str) = toml::to_string_pretty(&main_cfg) {
            let _ = std::fs::write(config_path(), toml_str);
        }

        if !legacy_cfg.gitlab.servers.is_empty()
            && let Ok(toml_str) = toml::to_string_pretty(&legacy_cfg.gitlab)
        {
            let _ = std::fs::write(gitlab_config_path(), toml_str);
        }

        let backup_path = legacy.with_extension("toml.bak");
        if let Err(e) = std::fs::rename(&legacy, &backup_path) {
            eprintln!("警告: 无法备份旧配置文件: {}", e);
        } else {
            println!(
                "{} 旧配置已迁移到 {}，备份于 {}",
                "迁移:".yellow(),
                dir.display(),
                backup_path.display()
            );
        }
    }
}

pub fn load() -> AppConfig {
    migrate_legacy_config();

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
    let dir = config_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    let path = config_path();
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn load_gitlab() -> GitLabConfig {
    migrate_legacy_config();

    let path = gitlab_config_path();
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

pub fn save_gitlab(config: &GitLabConfig) -> anyhow::Result<()> {
    let dir = config_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    let path = gitlab_config_path();
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

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

# Example: GitLab with relative URL root (e.g. http://host/gitlab/)
# [[remote.rules]]
# hosts = ["192.168.0.110", "192.168.0.110:2222"]
# name = "gitlab"
# url_prefix = "gitlab/"
# path_prefixes = ["hujinghui/"]
# path_prefix_name = "hujinghui"

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
