use serde::{Deserialize, Deserializer, Serialize, de::Visitor};

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
            max_depth: 3,
            skip_dirs: vec![
                ".venv",
                "venv",
                "env",
                ".env",
                "node_modules",
                "__pycache__",
                ".tox",
                ".mypy_cache",
                ".pytest_cache",
                ".ruff_cache",
                "dist",
                "build",
                "target",
                ".gradle",
                ".idea",
                ".vscode",
                ".fleet",
                ".cache",
                ".next",
                ".nuxt",
                ".svelte-kit",
                ".angular",
                "bower_components",
                ".terraform",
                ".cargo",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }
}

fn default_max_depth() -> usize {
    3
}

fn default_skip_dirs() -> Vec<String> {
    RepositoryConfig::default().skip_dirs
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
            ..Default::default()
        },
        RemoteRule {
            hosts: vec!["gitana.jeanhwea.io".into()],
            name: "gitana".into(),
            ..Default::default()
        },
        RemoteRule {
            hosts: vec!["gitee.com".into()],
            name: "gitee".into(),
            path_prefixes: vec![
                "red_8/".into(),
                "redtool/".into(),
                "red_base/".into(),
                "teampuzzle/".into(),
            ],
            path_prefix_name: Some("redinf".into()),
            ..Default::default()
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
            skip_push_hosts: vec![
                "github.com".into(),
                "githubfast.com".into(),
                "gitee.com".into(),
            ],
        }
    }
}

fn default_skip_push_hosts() -> Vec<String> {
    SyncConfig::default().skip_push_hosts
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitLabConfig {
    #[serde(default)]
    pub servers: Vec<GitLabServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitLabServer {
    #[serde(default, deserialize_with = "deserialize_trimmed_string")]
    pub url: String,
    #[serde(default)]
    pub token: String,
    #[serde(default = "default_gitlab_protocol")]
    pub protocol: String,
    #[serde(default)]
    pub prefix: String,
}

fn default_gitlab_protocol() -> String {
    "ssh".to_string()
}

fn deserialize_trimmed_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct TrimmedStringVisitor;

    impl<'de> Visitor<'de> for TrimmedStringVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value.trim().to_string())
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value.trim().to_string())
        }
    }

    deserializer.deserialize_string(TrimmedStringVisitor)
}

impl Default for GitLabServer {
    fn default() -> Self {
        Self {
            url: String::new(),
            token: String::new(),
            protocol: "ssh".to_string(),
            prefix: String::new(),
        }
    }
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
# token = "glpat-XXXXXXXXXXXXXXXXXXX"
# protocol = "ssh"
# prefix = "gitlab"
"#
}
