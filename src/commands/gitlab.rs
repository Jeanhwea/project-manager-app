use crate::control::command::Command;
use crate::domain::config::ConfigManager;
use crate::domain::config::schema;
use crate::error::{AppError, Result};
use crate::model::plan::{EditOperation, ExecutionPlan, GitOperation, MessageOperation};
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
    #[arg(help = "Project path on GitLab (e.g. group/project)")]
    pub project: String,
    #[arg(long, short, help = "GitLab server URL (overrides configured server)")]
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

#[derive(Debug)]
pub(crate) struct CloneContext {
    clone_url: String,
    target_dir: String,
    server_url: String,
    project: String,
    protocol: String,
}

impl Command for LoginArgs {
    type Context = LoginContext;

    fn context(&self) -> Result<LoginContext> {
        let mut config = ConfigManager::load_gitlab();
        let is_update = config.servers.iter().any(|s| s.url == self.url);

        if !is_update {
            config.servers.push(schema::GitLabServer {
                url: self.url.clone(),
                token: self.token.clone(),
                protocol: self.protocol.clone(),
            });
        } else {
            if let Some(existing) = config.servers.iter_mut().find(|s| s.url == self.url) {
                existing.token = self.token.clone();
                existing.protocol = self.protocol.clone();
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
            .map_err(|e| AppError::InvalidInput(format!("序列化配置失败: {}", e)))?;

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

        let server = if let Some(url) = &self.server {
            config
                .servers
                .iter()
                .find(|s| &s.url == url)
                .ok_or_else(|| AppError::not_found(format!("GitLab 服务器 {} 未配置", url)))?
        } else {
            config.servers.first().ok_or_else(|| {
                AppError::not_found("未配置 GitLab 服务器，请先执行 pma gitlab login")
            })?
        };

        let clone_url = match server.protocol.as_str() {
            "ssh" => format!("git@{}:{}.git", extract_host(&server.url)?, self.project),
            _ => format!("{}/{}.git", server.url.trim_end_matches('/'), self.project),
        };

        let target_dir = self
            .project
            .split('/')
            .next_back()
            .unwrap_or(&self.project)
            .to_string();

        Ok(CloneContext {
            clone_url,
            target_dir,
            server_url: server.url.clone(),
            project: self.project.clone(),
            protocol: server.protocol.clone(),
        })
    }

    fn plan(&self, ctx: &CloneContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);

        plan.add(MessageOperation::Header {
            title: "克隆项目".to_string(),
        });
        plan.add(MessageOperation::Item {
            label: "服务器".to_string(),
            value: ctx.server_url.clone(),
        });
        plan.add(MessageOperation::Item {
            label: "项目".to_string(),
            value: ctx.project.clone(),
        });
        plan.add(MessageOperation::Item {
            label: "协议".to_string(),
            value: ctx.protocol.clone(),
        });
        plan.add(MessageOperation::Item {
            label: "URL".to_string(),
            value: ctx.clone_url.clone(),
        });

        plan.add(GitOperation::Clone {
            url: ctx.clone_url.clone(),
            dir: PathBuf::from(&ctx.target_dir),
        });

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
        .map_err(|_| AppError::invalid_input(format!("无效的 URL: {}", url)))?;
    parsed
        .host_str()
        .map(String::from)
        .ok_or_else(|| AppError::invalid_input(format!("URL 中缺少主机名: {}", url)))
}
