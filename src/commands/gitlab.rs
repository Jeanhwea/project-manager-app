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
    #[arg(help = "Group path on GitLab (e.g. group or group/subgroup)")]
    pub group: String,
    #[arg(long, short, help = "GitLab server URL (overrides configured server)")]
    pub server: Option<String>,
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be changed without making any modifications"
    )]
    pub dry_run: bool,
}

fn parse_server_and_group(input: &str) -> (Option<String>, String) {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix("http://") {
        if let Some(idx) = rest.find('/') {
            let host_part = &rest[..idx];
            let path_part = &rest[idx + 1..];
            let prefix = path_part.split('/').next().unwrap_or("");
            let group = path_part.strip_prefix(prefix).unwrap_or(path_part).trim_start_matches('/');
            return (
                Some(format!("http://{}/{}", host_part, prefix)),
                group.to_string(),
            );
        }
        return (Some(format!("http://{}", rest)), String::new());
    }
    if let Some(rest) = trimmed.strip_prefix("https://") {
        if let Some(idx) = rest.find('/') {
            let host_part = &rest[..idx];
            let path_part = &rest[idx + 1..];
            let prefix = path_part.split('/').next().unwrap_or("");
            let group = path_part.strip_prefix(prefix).unwrap_or(path_part).trim_start_matches('/');
            return (
                Some(format!("https://{}/{}", host_part, prefix)),
                group.to_string(),
            );
        }
        return (Some(format!("https://{}", rest)), String::new());
    }
    (None, trimmed.to_string())
}

fn extract_prefix(url: &str) -> String {
    let trimmed = url.trim();
    if let Some(rest) = trimmed.strip_prefix("http://") {
        if let Some(idx) = rest.find('/') {
            return rest[idx + 1..].trim_end_matches('/').to_string();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("https://") {
        if let Some(idx) = rest.find('/') {
            return rest[idx + 1..].trim_end_matches('/').to_string();
        }
    }
    String::new()
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
    token: String,
    protocol: String,
    projects: Vec<CloneProject>,
}

impl Command for LoginArgs {
    type Context = LoginContext;

    fn context(&self) -> Result<LoginContext> {
        let mut config = ConfigManager::load_gitlab();
        let trimmed_url = self.url.trim().to_string();
        let is_update = config.servers.iter().any(|s| s.url == trimmed_url);

        let prefix = extract_prefix(&self.url);

        if !is_update {
            config.servers.push(schema::GitLabServer {
                url: trimmed_url,
                token: self.token.clone(),
                protocol: self.protocol.clone(),
                prefix,
            });
        } else {
            if let Some(existing) = config.servers.iter_mut().find(|s| s.url == trimmed_url) {
                existing.token = self.token.clone();
                existing.protocol = self.protocol.clone();
                existing.prefix = prefix;
            }
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
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("序列化配置失败: {}", e))))?;

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

        let (server_override, group_path) = parse_server_and_group(&self.group);

        let server = if let Some(url) = server_override.as_ref().or(self.server.as_ref()) {
            let trimmed = url.trim();
            config
                .servers
                .iter()
                .find(|s| s.url.trim() == trimmed)
                .ok_or_else(|| AppError::not_found(format!("GitLab 服务器 {} 未配置", url)))?
        } else {
            config.servers.first().ok_or_else(|| {
                AppError::not_found("未配置 GitLab 服务器，请先执行 pma gitlab login")
            })?
        };

        let projects =
            fetch_group_projects(&server.url, &server.token, &server.prefix, &group_path)?;

        Ok(CloneContext {
            server_url: server.url.clone(),
            token: server.token.clone(),
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

fn extract_host(url: &str) -> Result<String> {
    let parsed = url::Url::parse(url)
        .map_err(|_| AppError::not_supported(format!("无效的 URL: {}", url)))?;
    parsed
        .host_str()
        .map(String::from)
        .ok_or_else(|| AppError::not_supported(format!("URL 中缺少主机名: {}", url)))
}

#[derive(Debug, Deserialize)]
struct GitLabProject {
    path_with_namespace: String,
    ssh_url_to_repo: String,
    http_url_to_repo: String,
}

fn fetch_group_projects(
    base_url: &str,
    token: &str,
    prefix: &str,
    group_path: &str,
) -> Result<Vec<CloneProject>> {
    let mut url_base = base_url.trim_end_matches('/').to_string();
    if !prefix.is_empty() {
        url_base.push('/');
        url_base.push_str(prefix.trim_matches('/'));
    }
    let api_url = format!(
        "{}/api/v4/groups/{}/projects?per_page=100",
        url_base,
        percent_encode(group_path)
    );

    let response = ureq::get(&api_url)
        .set("PRIVATE-TOKEN", token)
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => AppError::gitlab_api(format!(
                "HTTP {}，请检查 group 路径 '{}' 和 token 是否正确",
                code, group_path
            )),
            _ => AppError::gitlab_api(format!("请求失败: {}", e)),
        })?;

    let projects: Vec<GitLabProject> = response
        .into_json()
        .map_err(|e| AppError::gitlab_api(format!("响应解析失败: {}", e)))?;

    Ok(projects
        .into_iter()
        .map(|p| CloneProject {
            path_with_namespace: p.path_with_namespace,
            ssh_url_to_repo: p.ssh_url_to_repo,
            http_url_to_repo: p.http_url_to_repo,
        })
        .collect())
}

fn percent_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
