use crate::control::plan::run_plan;
use crate::domain::AppError;
use crate::model::plan::{ExecutionPlan, GitOperation};
use crate::utils::output::Output;

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

pub fn run(args: GitLabArgs) -> anyhow::Result<()> {
    match args {
        GitLabArgs::Login(args) => execute_login(args),
        GitLabArgs::Clone(args) => execute_clone(args),
    }
}

fn execute_login(args: LoginArgs) -> anyhow::Result<()> {
    let mut config = crate::domain::config::ConfigManager::load_gitlab();

    if let Some(existing) = config.servers.iter_mut().find(|s| s.url == args.url) {
        Output::warning(&format!("服务器 {} 已存在，将更新 token", args.url));
        existing.token = args.token.clone();
        existing.protocol = args.protocol.clone();
    } else {
        config
            .servers
            .push(crate::domain::config::schema::GitLabServer {
                url: args.url.clone(),
                token: args.token.clone(),
                protocol: args.protocol.clone(),
            });
    }

    crate::domain::config::ConfigManager::save_gitlab(&config)?;

    Output::success(&format!("已添加 GitLab 服务器: {}", args.url));
    Ok(())
}

fn execute_clone(args: CloneArgs) -> anyhow::Result<()> {
    let config = crate::domain::config::ConfigManager::load_gitlab();

    let server = if let Some(url) = &args.server {
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
        "ssh" => format!("git@{}:{}.git", extract_host(&server.url)?, args.project),
        _ => format!("{}/{}.git", server.url.trim_end_matches('/'), args.project),
    };

    let target_dir = args
        .project
        .split('/')
        .next_back()
        .unwrap_or(&args.project)
        .to_string();

    Output::header("克隆项目");
    Output::item("服务器", &server.url);
    Output::item("项目", &args.project);
    Output::item("协议", &server.protocol);
    Output::item("URL", &clone_url);

    let mut plan = ExecutionPlan::new().with_dry_run(args.dry_run);
    plan.add(GitOperation::Clone {
        url: clone_url,
        dir: std::path::PathBuf::from(&target_dir),
    });
    run_plan(&plan)
}

fn extract_host(url: &str) -> anyhow::Result<String> {
    let parsed = url::Url::parse(url)
        .map_err(|_| AppError::invalid_input(format!("无效的 URL: {}", url)))?;
    parsed
        .host_str()
        .map(String::from)
        .ok_or_else(|| AppError::invalid_input(format!("URL 中缺少主机名: {}", url)).into())
}
