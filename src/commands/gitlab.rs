//! `pma gitlab`：管理 GitLab 服务器凭据并批量克隆 group 下的项目。
//! login 把 URL/token/协议写回 `~/.pma/gitlab.toml`，已有同 URL 记录时只更新凭据；
//! clone 先按 group 查询 GitLab API v4 得到项目清单，再逐个生成 git clone 操作。
//! token 只取自配置文件，不读环境变量；请求统一用 `PRIVATE-TOKEN` 头鉴权。

use crate::commands::Command;
use crate::domain::config::ConfigManager;
use crate::domain::config::schema;
use crate::domain::git::GitOperation;
use crate::engine::plan;
use crate::error::{AppError, Result};
use crate::model::operation::EditOperation;
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase};
use serde::Deserialize;
use std::path::PathBuf;

/// 与 GitLab 交互时的失败分类：HTTP 状态码、请求发送/连接失败、响应解析失败、配置序列化失败
#[derive(Debug, thiserror::Error)]
pub enum GitlabApiError {
    #[error("HTTP {status}，请求路径: {path}")]
    HttpStatus { status: u16, path: String },

    #[error("请求失败")]
    Request {
        #[source]
        source: Box<ureq::Error>,
    },

    #[error("解析 group 列表失败")]
    ParseGroups {
        #[source]
        source: std::io::Error,
    },

    #[error("解析项目列表失败")]
    ParseProjects {
        #[source]
        source: std::io::Error,
    },

    #[error("序列化配置失败: {reason}")]
    SerializeConfig { reason: String },
}

/// `pma gitlab` 的子命令：登录配置（login）与批量克隆（clone）
#[derive(Debug, clap::Subcommand)]
pub enum GitlabArgs {
    Login(LoginArgs),
    Clone(CloneArgs),
}

/// `pma gitlab login`：新增一台 GitLab 服务器，或更新已有服务器的 token 与协议
#[derive(Debug, clap::Args)]
pub struct LoginArgs {
    #[arg(long, short, help = "GitLab server URL (e.g. https://gitlab.com)")]
    pub url: String,
    #[arg(long, short, help = "Personal access token")]
    // token 以明文落盘到 gitlab.toml，故该文件权限与备份策略需要用户自行注意
    pub token: String,
    #[arg(
        long,
        default_value = "ssh",
        help = "Protocol for clone operations (ssh or https)"
    )]
    pub protocol: String,
}

/// `pma gitlab clone`：克隆 group 下的全部项目到当前目录；`group` 既可以是纯路径也可以是完整 URL
#[derive(Debug, clap::Args)]
pub struct CloneArgs {
    #[arg(help = "Group path or full URL (e.g. mygroup or https://gitlab.example.com/mygroup)")]
    pub group: String,
    #[arg(long, short, help = "Server URL override")]
    pub server: Option<String>,
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be changed without making any modifications"
    )]
    pub dry_run: bool,
}

/// login 的采集结果：整份待写回的配置，以及本次是新增还是更新已有服务器
#[derive(Debug)]
pub(crate) struct LoginContext {
    config: schema::GitlabConfig,
    is_update: bool,
}

/// GitLab 项目里 pma 关心的三个字段：完整路径与两种克隆 URL
#[derive(Debug, Clone)]
pub(crate) struct CloneProject {
    pub path_with_namespace: String,
    pub ssh_url_to_repo: String,
    pub http_url_to_repo: String,
}

/// clone 的采集结果：命中的服务器信息与其 group 下的项目清单
#[derive(Debug)]
pub(crate) struct CloneContext {
    server_url: String,
    protocol: String,
    projects: Vec<CloneProject>,
}

impl Command for LoginArgs {
    type Context = LoginContext;
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<LoginContext> {
        let mut config = ConfigManager::load_gitlab();
        let trimmed_url = self.url.trim().to_string();
        // URL 是服务器的唯一键：命中即更新，新服务器的 prefix 留空（只能手工改配置文件）
        let is_update = config.servers.iter().any(|s| s.url == trimmed_url);

        if !is_update {
            config.servers.push(schema::GitlabServer {
                url: trimmed_url,
                token: self.token.clone(),
                protocol: self.protocol.clone(),
                prefix: String::new(),
            });
        } else if let Some(existing) = config.servers.iter_mut().find(|s| s.url == trimmed_url) {
            // 只覆盖凭据与协议，已有的 prefix 不变
            existing.token = self.token.clone();
            existing.protocol = self.protocol.clone();
        }

        Ok(LoginContext { config, is_update })
    }

    fn plan(&self, ctx: &LoginContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();

        if ctx.is_update {
            plan.add_message(DisplayMessage::Warning {
                msg: format!("服务器 {} 已存在，将更新 token", self.url),
            });
        }

        // 整体序列化覆盖写回，而不是在文件里定位某行做增量修改
        let config_content =
            toml::to_string_pretty(&ctx.config).map_err(|e| GitlabApiError::SerializeConfig {
                reason: e.to_string(),
            })?;

        plan.add(EditOperation::WriteFile {
            path: ConfigManager::gitlab_path().to_string_lossy().to_string(),
            content: config_content,
            description: "save gitlab config".to_string(),
        });

        plan.add_message(DisplayMessage::Success {
            msg: format!("已添加 GitLab 服务器: {}", self.url),
        });

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

impl Command for CloneArgs {
    type Context = CloneContext;
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<CloneContext> {
        let config = ConfigManager::load_gitlab();
        let (server, group_path) =
            resolve_server_and_group(&config, &self.group, self.server.as_deref())?;
        let api_base = api_base_url(server);
        let projects = fetch_group_projects(&api_base, &server.token, &group_path)?;

        Ok(CloneContext {
            server_url: server.url.clone(),
            protocol: server.protocol.clone(),
            projects,
        })
    }

    fn plan(&self, ctx: &CloneContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);

        if ctx.projects.is_empty() {
            plan.add_message(DisplayMessage::Warning {
                msg: "未找到项目".to_string(),
            });
            return Ok(plan);
        }

        plan.add_message(DisplayMessage::Header {
            title: "克隆项目".to_string(),
        });
        plan.add_message(DisplayMessage::Item {
            label: "服务器".to_string(),
            value: ctx.server_url.clone(),
        });
        plan.add_message(DisplayMessage::Item {
            label: "协议".to_string(),
            value: ctx.protocol.clone(),
        });
        plan.add_message(DisplayMessage::Item {
            label: "数量".to_string(),
            value: ctx.projects.len().to_string(),
        });

        let mut clone_phase = Phase::new("克隆项目");
        for proj in &ctx.projects {
            // 只有 ssh 走 SSH URL，其余值一律按 HTTPS 处理，避免非法协议值被传给 git
            let url = match ctx.protocol.as_str() {
                "ssh" => &proj.ssh_url_to_repo,
                _ => &proj.http_url_to_repo,
            };
            // 只取 namespace 末段作为目录名；同名项目撞车时目标目录已存在，执行阶段会跳过克隆
            let target_dir = proj
                .path_with_namespace
                .split('/')
                .next_back()
                .unwrap_or(&proj.path_with_namespace)
                .to_string();

            clone_phase.add_message(DisplayMessage::Item {
                label: "项目".to_string(),
                value: proj.path_with_namespace.clone(),
            });

            clone_phase.add(GitOperation::Clone {
                url: url.clone(),
                target_dir: PathBuf::from(&target_dir),
                working_dir: PathBuf::from("."),
            });
        }
        plan.add_phase(clone_phase);

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

/// 分发 `pma gitlab` 子命令
pub fn run(args: GitlabArgs) -> Result<()> {
    match args {
        GitlabArgs::Login(args) => Command::run(&args),
        GitlabArgs::Clone(args) => Command::run(&args),
    }
}

/// 把用户输入的 group 解析成「服务器配置 + group 路径」；完整 URL 形式的输入会反查服务器
fn resolve_server_and_group<'a>(
    config: &'a schema::GitlabConfig,
    group_input: &str,
    server_override: Option<&str>,
) -> Result<(&'a schema::GitlabServer, String)> {
    let input = group_input.trim();

    if input.starts_with("http://") || input.starts_with("https://") {
        return resolve_from_full_url(config, input);
    }

    // 显式 --server 时按配置的原始 URL 精确匹配（不含 prefix），未命中就报错而不是退回第一台
    if let Some(server_url) = server_override {
        let trimmed = server_url.trim();
        let server = config
            .servers
            .iter()
            .find(|s| s.url.trim() == trimmed)
            .ok_or_else(|| AppError::NotFound {
                resource: "GitLab 服务器".into(),
                name: server_url.to_string(),
            })?;
        return Ok((server, input.to_string()));
    }

    // 兜底用第一台服务器：单服务器场景下不必每次都敲 --server
    let server = config.servers.first().ok_or_else(|| AppError::NotFound {
        resource: "GitLab 服务器".into(),
        name: "未配置, 请先执行 pma gitlab login".into(),
    })?;
    Ok((server, input.to_string()))
}

/// 从完整 URL 反查服务器；多个服务器前缀都能匹配时取最长的一个，即配置更具体的那个
fn resolve_from_full_url<'a>(
    config: &'a schema::GitlabConfig,
    full_url: &str,
) -> Result<(&'a schema::GitlabServer, String)> {
    let best_match = config
        .servers
        .iter()
        .filter(|s| {
            let base = api_base_url(s);
            // 比 base 多一层的要求 "base/..." 前缀，防止 https://gitlab.com/foo 误配到 https://gitlab.com/foobar
            let base_with_slash = format!("{}/", base.trim_end_matches('/'));
            full_url.starts_with(base_with_slash.as_str()) || full_url == base
        })
        .max_by_key(|s| api_base_url(s).len());

    match best_match {
        Some(server) => {
            let base = api_base_url(server);
            // full_url 已保证以 base 开头，切掉 base 后剩下的就是 group 路径
            let group_path = full_url[base.len()..].trim_start_matches('/').to_string();
            if group_path.is_empty() {
                // 只给了服务器根 URL，没有 group 段可取
                return Err(AppError::NotFound {
                    resource: "URL 中的 group 路径".into(),
                    name: full_url.to_string(),
                });
            }
            Ok((server, group_path))
        }
        None => Err(AppError::NotFound {
            resource: "GitLab 服务器配置".into(),
            name: full_url.to_string(),
        }),
    }
}

/// GitLab API 的 base URL：服务器 URL 拼上可选的路径前缀 `prefix`（如自建实例的 /gitlab）
fn api_base_url(server: &schema::GitlabServer) -> String {
    let url = server.url.trim_end_matches('/');
    let prefix = server.prefix.trim_matches('/');
    if prefix.is_empty() {
        return url.to_string();
    }
    if let Ok(parsed) = url::Url::parse(url) {
        // URL 里已经带了 prefix（例如 https://host/gitlab）就不再重复拼接
        let path = parsed.path().trim_end_matches('/');
        if path.ends_with(&format!("/{}", prefix)) || path == format!("/{}", prefix) {
            return url.to_string();
        }
    }
    format!("{}/{}", url, prefix)
}

/// `/groups?search=` 返回项里 pma 用到的两个字段
#[derive(Debug, Deserialize)]
struct GitlabGroup {
    id: u64,
    full_path: String,
}

/// `/groups/:id/projects` 返回项里 pma 用到的三个字段
#[derive(Debug, Deserialize)]
struct GitlabProject {
    path_with_namespace: String,
    ssh_url_to_repo: String,
    http_url_to_repo: String,
}

/// 发一个 GitLab API v4 的 GET 请求，`path` 需以 `/` 开头（如 `/groups/1/projects`）
fn gitlab_get(base_url: &str, token: &str, path: &str) -> Result<ureq::Response> {
    let url = format!("{}/api/v4{}", base_url, path);
    ureq::get(&url)
        .set("PRIVATE-TOKEN", token)
        .call()
        // 非 2xx 被 ureq 转成 Status，这里丢掉响应体只保留状态码 + 路径便于定位
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => AppError::GitlabApi(GitlabApiError::HttpStatus {
                status: code,
                path: path.to_string(),
            }),
            _ => AppError::GitlabApi(GitlabApiError::Request {
                source: Box::new(e),
            }),
        })
}

/// 按 group 完整路径查 id；找不到完全一致的路径时报 NotFound
fn find_group_id(base_url: &str, token: &str, group_path: &str) -> Result<u64> {
    // /groups?search= 是模糊匹配，且嵌套 group 的分隔符必须转义成 %2F 才能当作整体路径搜
    let search_path = format!("/groups?search={}", group_path.replace('/', "%2F"));
    let response = gitlab_get(base_url, token, &search_path)?;
    let groups: Vec<GitlabGroup> = response
        .into_json()
        .map_err(|e| GitlabApiError::ParseGroups { source: e })?;

    // 搜索结果里必须再过滤一次：否则 /groups?search=foo 命中 foobar 也会被当成目标 group
    groups
        .iter()
        .find(|g| g.full_path == group_path)
        .map(|g| g.id)
        .ok_or_else(|| AppError::NotFound {
            resource: "GitLab group".into(),
            name: group_path.to_string(),
        })
}

/// 拉取 group 下的项目清单；group 路径不存在时先由 group id 查询报错
fn fetch_group_projects(
    base_url: &str,
    token: &str,
    group_path: &str,
) -> Result<Vec<CloneProject>> {
    let group_id = find_group_id(base_url, token, group_path)?;
    // per_page=100 是 GitLab 上限；这里没有翻页，超过 100 个项目的 group 只会拿到第一页
    let projects_path = format!("/groups/{}/projects?per_page=100", group_id);
    let response = gitlab_get(base_url, token, &projects_path)?;

    let projects: Vec<GitlabProject> = response
        .into_json()
        .map_err(|e| GitlabApiError::ParseProjects { source: e })?;

    Ok(projects
        .into_iter()
        .map(|p| CloneProject {
            path_with_namespace: p.path_with_namespace,
            ssh_url_to_repo: p.ssh_url_to_repo,
            http_url_to_repo: p.http_url_to_repo,
        })
        .collect())
}
