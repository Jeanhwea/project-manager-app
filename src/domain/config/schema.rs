//! 配置文件的 TOML 数据结构、默认值，以及 `pma config init` 写出的模板内容。
//! 每个字段都带 `#[serde(default)]`，用户只写关心的那几项也能正常加载。

use serde::{Deserialize, Deserializer, Serialize, de::Visitor};

/// 主配置文件 config.toml 的顶层结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub repository: RepositoryConfig,
    #[serde(default)]
    pub remote: RemoteConfig,
    #[serde(default)]
    pub sync: SyncConfig,
}

/// 仓库发现相关的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    /// 向下搜索的最大层数
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    /// 遍历时整棵跳过的目录名
    #[serde(default = "default_skip_dirs")]
    pub skip_dirs: Vec<String>,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            // 依赖目录与构建产物：既不可能藏着要管的仓库，又动辄几万个子目录，必须跳过
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

/// remote 的期望命名规则
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

/// 一条 remote 命名规则，按 `hosts` 匹配 remote url 后决定该 remote 应该叫什么
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteRule {
    /// 命中的 host 列表，任一匹配即生效
    #[serde(default)]
    pub hosts: Vec<String>,
    /// 期望的 remote 名称
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url_prefix: Option<String>,
    /// 仓库路径带这些前缀时改用 `path_prefix_name`，用于同一 host 下区分不同组织
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

/// sync 阶段的行为配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncConfig {
    /// 这些 remote 只 pull 不 push
    #[serde(default = "default_skip_push_remotes")]
    pub skip_push_remotes: Vec<String>,
}

// 这里依赖 derive(Default) 给出空 Vec；若改成手写 Default 并复用本函数会无限递归
fn default_skip_push_remotes() -> Vec<String> {
    SyncConfig::default().skip_push_remotes
}

/// gitlab.toml 的顶层结构，可登记多台 GitLab 服务器
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitlabConfig {
    #[serde(default)]
    pub servers: Vec<GitlabServer>,
}

/// 一台 GitLab 服务器的访问方式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitlabServer {
    #[serde(default, deserialize_with = "deserialize_trimmed_string")]
    pub url: String,
    /// 个人访问令牌，`pma gitlab login` 写入
    #[serde(default)]
    pub token: String,
    /// 克隆仓库使用的协议，`ssh` 或 `https`
    #[serde(default = "default_gitlab_protocol")]
    pub protocol: String,
    /// 本地目录名前缀，便于区分不同服务器上的同名项目
    #[serde(default)]
    pub prefix: String,
}

fn default_gitlab_protocol() -> String {
    "ssh".to_string()
}

// url、token 常从网页或命令行复制进来，尾随空格/换行会让拼接出的地址失效，故统一 trim
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

impl Default for GitlabServer {
    fn default() -> Self {
        Self {
            url: String::new(),
            token: String::new(),
            protocol: "ssh".to_string(),
            prefix: String::new(),
        }
    }
}

/// `pma config init` 写出的 config.toml 模板，内容与各结构的 Default 值保持一致
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
# Skip pushing to these remote names
skip_push_remotes = []
"#
}

/// `pma config init` 写出的 gitlab.toml 模板，只给注释不写真实凭据
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
