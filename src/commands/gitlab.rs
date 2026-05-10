use crate::domain::config::ConfigDir;
use crate::domain::config::schema::GitLabServer;
use crate::domain::gitlab::client::GitLabClient;
use crate::utils::output::Output;
use anyhow::{Context, Result};
use std::path::Path;

#[derive(Debug, clap::Subcommand)]
pub enum GitLabArgs {
    /// Login to a GitLab server
    Login(LoginArgs),
    /// Clone all projects from a GitLab group
    Clone(CloneArgs),
}

#[derive(Debug, clap::Args)]
pub struct LoginArgs {
    /// GitLab server URL (e.g. https://gitlab.com)
    #[arg(help = "GitLab server URL (e.g. https://gitlab.com)")]
    pub url: String,
    /// Personal access token
    #[arg(help = "Personal access token")]
    pub token: String,
    /// Protocol for cloning (ssh or http)
    #[arg(
        long,
        default_value = "ssh",
        help = "Protocol for cloning (ssh or http)"
    )]
    pub protocol: String,
}

#[derive(Debug, clap::Args)]
pub struct CloneArgs {
    /// Group name or path to clone from
    #[arg(help = "Group name or path to clone from")]
    pub group: String,
    /// GitLab server URL (defaults to the first configured server)
    #[arg(long, help = "GitLab server URL")]
    pub server: Option<String>,
    /// Target directory for cloning
    #[arg(
        long,
        short,
        default_value = ".",
        help = "Target directory for cloning"
    )]
    pub target: String,
    /// Maximum depth to search for repositories
    #[arg(long, short, default_value = "3")]
    pub max_depth: Option<usize>,
}

pub fn run(args: GitLabArgs) -> Result<()> {
    match args {
        GitLabArgs::Login(args) => execute_login(args),
        GitLabArgs::Clone(args) => execute_clone(args),
    }
}

fn execute_login(args: LoginArgs) -> Result<()> {
    if args.url.is_empty() {
        anyhow::bail!("服务器地址不能为空");
    }

    if args.token.is_empty() {
        anyhow::bail!("Token 不能为空");
    }

    let client = GitLabClient::with_url_and_token(&args.url, &args.token);

    Output::info(&format!("验证 {} 的认证信息...", args.url));

    let _user = client.get_current_user().context("认证失败")?;

    Output::success("认证成功");

    let server = GitLabServer {
        url: args.url.clone(),
        token: args.token,
        protocol: args.protocol,
    };

    let mut gitlab_config = ConfigDir::load_gitlab();
    if let Some(existing) = gitlab_config.servers.iter_mut().find(|s| s.url == args.url) {
        *existing = server;
        Output::info("已更新现有服务器配置");
    } else {
        gitlab_config.servers.push(server);
        Output::info("已添加新服务器配置");
    }

    ConfigDir::save_gitlab(&gitlab_config).context("保存配置失败")?;

    Output::success(&format!("已保存 GitLab 服务器配置: {}", args.url));
    Ok(())
}

fn execute_clone(args: CloneArgs) -> Result<()> {
    let gitlab_config = ConfigDir::load_gitlab();

    let server = if let Some(ref url) = args.server {
        gitlab_config
            .servers
            .iter()
            .find(|s| &s.url == url)
            .ok_or_else(|| anyhow::anyhow!("未找到服务器配置: {}", url))?
    } else {
        gitlab_config
            .servers
            .first()
            .ok_or_else(|| anyhow::anyhow!("未配置 GitLab 服务器，请先使用 pma gitlab login"))?
    };

    let client = GitLabClient::with_url_and_token(&server.url, &server.token);

    Output::info(&format!("获取组 {} 的项目列表...", args.group));

    let groups = client.get_groups().context("获取组列表失败")?;

    let group = groups
        .iter()
        .find(|g| g.full_path == args.group || g.name == args.group)
        .ok_or_else(|| anyhow::anyhow!("未找到组: {}", args.group))?;

    let projects = client
        .get_group_projects(group.id, true, false)
        .context("获取项目列表失败")?;

    if projects.is_empty() {
        Output::warning(&format!("组 {} 中没有项目", args.group));
        return Ok(());
    }

    Output::info(&format!("找到 {} 个项目", projects.len()));

    let target_path = Path::new(&args.target);
    std::fs::create_dir_all(target_path)?;

    for project in &projects {
        let clone_url = match server.protocol.as_str() {
            "http" | "https" => project.http_url.clone().unwrap_or_default(),
            _ => project.ssh_url.clone().unwrap_or_default(),
        };

        let project_path = target_path.join(&project.path_with_namespace);

        if project_path.exists() {
            Output::skip(&format!("{} (已存在)", project.path_with_namespace));
            continue;
        }

        Output::cmd(&format!(
            "git clone {} {}",
            clone_url,
            project_path.display()
        ));

        if let Some(parent) = project_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let output = std::process::Command::new("git")
            .args(["clone", &clone_url, &project_path.to_string_lossy()])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                Output::success(&format!("已克隆: {}", project.path_with_namespace));
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                Output::error(&format!(
                    "克隆失败: {} - {}",
                    project.path_with_namespace,
                    stderr.trim()
                ));
            }
            Err(e) => {
                Output::error(&format!(
                    "克隆失败: {} - {}",
                    project.path_with_namespace, e
                ));
            }
        }
    }

    Output::success(&format!("完成! 共处理 {} 个项目", projects.len()));
    Ok(())
}
