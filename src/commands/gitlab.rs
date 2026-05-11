use crate::control::command::Command;
use crate::domain::config::ConfigManager;
use crate::domain::config::schema;
use crate::error::{AppError, Result};
use crate::model::plan::{EditOperation, ExecutionPlan, GitOperation, MessageOperation};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub enum GitLabArgs {
    Login(LoginArgs),
    Clone(CloneArgs),
}

#[derive(Debug, clap::Args)]
pub struct LoginArgs {
    #[arg(long, short, help = "GitLab server URL (e.g. https://gitlab.com)")]
    pub url: String,
    #[arg(long, short, help = "Personal access token")]
    pub token: String,
    #[arg(
        long,
        default_value = "ssh",
        help = "Protocol for clone operations (ssh or https)"
    )]
    pub protocol: String,
}

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

#[derive(Debug)]
pub(crate) struct LoginContext {
    config: schema::GitLabConfig,
    is_update: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CloneProject {
    pub path_with_namespace: String,
    pub ssh_url_to_repo: String,
    pub http_url_to_repo: String,
}

#[derive(Debug)]
pub(crate) struct CloneContext {
    server_url: String,
    protocol: String,
    projects: Vec<CloneProject>,
}

impl Command for LoginArgs {
    type Context = LoginContext;

    fn context(&self) -> Result<LoginContext> {
        let mut config = ConfigManager::load_gitlab();
        let trimmed_url = self.url.trim().to_string();
        let is_update = config.servers.iter().any(|s| s.url == trimmed_url);

        if !is_update {
            config.servers.push(schema::GitLabServer {
                url: trimmed_url,
                token: self.token.clone(),
                protocol: self.protocol.clone(),
                prefix: String::new(),
            });
        } else if let Some(existing) = config.servers.iter_mut().find(|s| s.url == trimmed_url) {
            existing.token = self.token.clone();
            existing.protocol = self.protocol.clone();
        }

        Ok(LoginContext { config, is_update })
    }

    fn plan(&self, ctx: &LoginContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();

        if ctx.is_update {
            plan.add(MessageOperation::Warning {
                msg: format!("服务器 {} 已存在，将更新 token", self.url),
            });
        }

        let config_content = toml::to_string_pretty(&ctx.config)
            .map_err(|e| AppError::Io(std::io::Error::other(format!("序列化配置失败: {}", e))))?;

        plan.add(EditOperation::WriteFile {
            path: ConfigManager::gitlab_path().to_string_lossy().to_string(),
            content: config_content,
            description: "save gitlab config".to_string(),
        });

        plan.add(MessageOperation::Success {
            msg: format!("已添加 GitLab 服务器: {}", self.url),
        });

        Ok(plan)
    }
}

impl Command for CloneArgs {
    type Context = CloneContext;

    fn context(&self) -> Result<CloneContext> {
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
            plan.add(MessageOperation::Warning {
                msg: "未找到项目".to_string(),
            });
            return Ok(plan);
        }

        plan.add(MessageOperation::Header {
            title: "克隆项目".to_string(),
        });
        plan.add(MessageOperation::Item {
            label: "服务器".to_string(),
            value: ctx.server_url.clone(),
        });
        plan.add(MessageOperation::Item {
            label: "协议".to_string(),
            value: ctx.protocol.clone(),
        });
        plan.add(MessageOperation::Item {
            label: "数量".to_string(),
            value: ctx.projects.len().to_string(),
        });

        for proj in &ctx.projects {
            let url = match ctx.protocol.as_str() {
                "ssh" => &proj.ssh_url_to_repo,
                _ => &proj.http_url_to_repo,
            };
            let target_dir = proj
                .path_with_namespace
                .split('/')
                .next_back()
                .unwrap_or(&proj.path_with_namespace)
                .to_string();

            plan.add(MessageOperation::Item {
                label: "项目".to_string(),
                value: proj.path_with_namespace.clone(),
            });

            plan.add(GitOperation::Clone {
                url: url.clone(),
                dir: PathBuf::from(&target_dir),
            });
        }

        Ok(plan)
    }
}

pub fn run(args: GitLabArgs) -> Result<()> {
    match args {
        GitLabArgs::Login(args) => Command::run(&args),
        GitLabArgs::Clone(args) => Command::run(&args),
    }
}

fn resolve_server_and_group<'a>(
    config: &'a schema::GitLabConfig,
    group_input: &str,
    server_override: Option<&str>,
) -> Result<(&'a schema::GitLabServer, String)> {
    let input = group_input.trim();

    if input.starts_with("http://") || input.starts_with("https://") {
        return resolve_from_full_url(config, input);
    }

    if let Some(server_url) = server_override {
        let trimmed = server_url.trim();
        let server = config
            .servers
            .iter()
            .find(|s| s.url.trim() == trimmed)
            .ok_or_else(|| AppError::not_found(format!("GitLab 服务器 {} 未配置", server_url)))?;
        return Ok((server, input.to_string()));
    }

    let server = config
        .servers
        .first()
        .ok_or_else(|| AppError::not_found("未配置 GitLab 服务器，请先执行 pma gitlab login"))?;
    Ok((server, input.to_string()))
}

fn resolve_from_full_url<'a>(
    config: &'a schema::GitLabConfig,
    full_url: &str,
) -> Result<(&'a schema::GitLabServer, String)> {
    let best_match = config
        .servers
        .iter()
        .filter(|s| {
            let base = api_base_url(s);
            let base_with_slash = format!("{}/", base.trim_end_matches('/'));
            full_url.starts_with(base_with_slash.as_str()) || full_url == base
        })
        .max_by_key(|s| api_base_url(s).len());

    match best_match {
        Some(server) => {
            let base = api_base_url(server);
            let group_path = full_url[base.len()..].trim_start_matches('/').to_string();
            if group_path.is_empty() {
                return Err(AppError::not_found(format!(
                    "URL '{}' 中未包含 group 路径",
                    full_url
                )));
            }
            Ok((server, group_path))
        }
        None => Err(AppError::not_found(format!(
            "未找到匹配 URL '{}' 的 GitLab 服务器配置",
            full_url
        ))),
    }
}

fn api_base_url(server: &schema::GitLabServer) -> String {
    let url = server.url.trim_end_matches('/');
    let prefix = server.prefix.trim_matches('/');
    if prefix.is_empty() {
        return url.to_string();
    }
    if let Ok(parsed) = url::Url::parse(url) {
        let path = parsed.path().trim_end_matches('/');
        if path.ends_with(&format!("/{}", prefix)) || path == format!("/{}", prefix) {
            return url.to_string();
        }
    }
    format!("{}/{}", url, prefix)
}

#[derive(Debug, Deserialize)]
struct GitLabGroup {
    id: u64,
    full_path: String,
}

#[derive(Debug, Deserialize)]
struct GitLabProject {
    path_with_namespace: String,
    ssh_url_to_repo: String,
    http_url_to_repo: String,
}

fn gitlab_get(base_url: &str, token: &str, path: &str) -> Result<ureq::Response> {
    let url = format!("{}/api/v4{}", base_url, path);
    ureq::get(&url)
        .set("PRIVATE-TOKEN", token)
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => {
                AppError::gitlab_api(format!("HTTP {}，请求路径: {}", code, path))
            }
            _ => AppError::gitlab_api(format!("请求失败: {}", e)),
        })
}

fn find_group_id(base_url: &str, token: &str, group_path: &str) -> Result<u64> {
    let search_path = format!("/groups?search={}", group_path.replace('/', "%2F"));
    let response = gitlab_get(base_url, token, &search_path)?;
    let groups: Vec<GitLabGroup> = response
        .into_json()
        .map_err(|e| AppError::gitlab_api(format!("解析 group 列表失败: {}", e)))?;

    groups
        .iter()
        .find(|g| g.full_path == group_path)
        .map(|g| g.id)
        .ok_or_else(|| AppError::not_found(format!("未找到 group '{}'", group_path)))
}

fn fetch_group_projects(
    base_url: &str,
    token: &str,
    group_path: &str,
) -> Result<Vec<CloneProject>> {
    let group_id = find_group_id(base_url, token, group_path)?;
    let projects_path = format!("/groups/{}/projects?per_page=100", group_id);
    let response = gitlab_get(base_url, token, &projects_path)?;

    let projects: Vec<GitLabProject> = response
        .into_json()
        .map_err(|e| AppError::gitlab_api(format!("解析项目列表失败: {}", e)))?;

    Ok(projects
        .into_iter()
        .map(|p| CloneProject {
            path_with_namespace: p.path_with_namespace,
            ssh_url_to_repo: p.ssh_url_to_repo,
            http_url_to_repo: p.http_url_to_repo,
        })
        .collect())
}
