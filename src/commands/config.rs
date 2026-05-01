//! Configuration command implementation

use super::{Command, CommandResult};
use colored::Colorize;
use std::env;
use std::path::PathBuf;

/// Configuration command arguments
#[derive(Debug)]
pub enum ConfigArgs {
    /// Initialize a default configuration file
    Init,
    /// Show current configuration
    Show,
    /// Show configuration file path
    Path,
}

/// Config command
pub struct ConfigCommand;

impl Command for ConfigCommand {
    type Args = ConfigArgs;

    fn execute(args: Self::Args) -> CommandResult {
        match args {
            ConfigArgs::Init => execute_init(),
            ConfigArgs::Show => execute_show(),
            ConfigArgs::Path => execute_path(),
        }
    }
}

fn execute_init() -> CommandResult {
    let dir = config_dir();
    if dir.exists() {
        return Err(super::CommandError::ExecutionFailed(format!(
            "配置目录已存在: {}",
            dir.display()
        )));
    }

    std::fs::create_dir_all(&dir).map_err(|e| super::CommandError::Io(e))?;

    let config_path = config_path();
    let content = default_config_content();
    std::fs::write(&config_path, content).map_err(|e| super::CommandError::Io(e))?;

    let gitlab_path = gitlab_config_path();
    let gitlab_content = default_gitlab_config_content();
    std::fs::write(&gitlab_path, gitlab_content).map_err(|e| super::CommandError::Io(e))?;

    println!("{} {}", "已创建配置目录:".green(), dir.display());
    println!("  {} {}", "主配置:".dimmed(), config_path.display());
    println!("  {} {}", "GitLab:".dimmed(), gitlab_path.display());
    Ok(())
}

fn execute_show() -> CommandResult {
    let dir = config_dir();
    let cfg = load_config();
    let gitlab_cfg = load_gitlab_config();
    let dir_exists = dir.exists();

    println!(
        "{} {} {}",
        "配置目录:".green(),
        dir.display(),
        if dir_exists {
            "".to_string()
        } else {
            "(未创建, 使用默认值)".yellow().to_string()
        }
    );
    println!();

    println!("{}", "[repository]".cyan());
    println!("  max_depth  = {}", cfg.repository.max_depth);
    println!("  skip_dirs  = {:?}", cfg.repository.skip_dirs);
    println!();

    println!("{}", "[remote]".cyan());
    for rule in &cfg.remote.rules {
        println!("  {} <- {:?}", rule.name.yellow(), rule.hosts);
        if let Some(ref url_prefix) = rule.url_prefix {
            println!("    {} = {}", "url_prefix".dimmed(), url_prefix.dimmed());
        }
        if !rule.path_prefixes.is_empty()
            && let Some(ref prefix_name) = rule.path_prefix_name
        {
            println!("    {} <- {:?}", prefix_name.yellow(), rule.path_prefixes);
        }
    }
    println!();

    println!("{}", "[sync]".cyan());
    println!("  skip_push_hosts = {:?}", cfg.sync.skip_push_hosts);
    println!();

    println!("{}", "[gitlab]".cyan());
    if gitlab_cfg.servers.is_empty() {
        println!(
            "  {}",
            "未配置 GitLab 服务器 (使用 pma gitlab login 添加)".dimmed()
        );
    } else {
        for srv in &gitlab_cfg.servers {
            println!("  {} ({})", srv.url.cyan(), srv.protocol.dimmed());
        }
    }

    Ok(())
}

fn execute_path() -> CommandResult {
    println!("{}", config_dir().display());
    Ok(())
}

// Helper functions from old config module
fn config_dir() -> PathBuf {
    if let Ok(path) = env::var("PMA_CONFIG_DIR") {
        return PathBuf::from(path);
    }

    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    PathBuf::from(home).join(".pma")
}

fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

fn gitlab_config_path() -> PathBuf {
    config_dir().join("gitlab.toml")
}

fn load_config() -> AppConfig {
    // For now, use the old config loading logic
    // TODO: Migrate to use domain::config module
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

fn load_gitlab_config() -> GitLabConfig {
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

fn default_config_content() -> &'static str {
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

fn default_gitlab_config_content() -> &'static str {
    r#"# GitLab server credentials
# Use `pma gitlab login` to add servers
#
# [[servers]]
# url = "https://gitlab.com"
# token = "glpat-xxxx"
# protocol = "ssh"
"#
}

// Types from old config module
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub repository: RepositoryConfig,
    #[serde(default)]
    pub remote: RemoteConfig,
    #[serde(default)]
    pub sync: SyncConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoteConfig {
    #[serde(default = "default_remote_rules")]
    pub rules: Vec<RemoteRule>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct GitLabConfig {
    #[serde(default)]
    pub servers: Vec<GitLabServer>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
