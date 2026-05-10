use crate::domain::AppError;
use crate::domain::config::ConfigDir;
use crate::domain::config::schema::GitLabServer;
use crate::utils::output::Output;
use anyhow::Context;

#[derive(Debug, clap::Subcommand)]
pub enum GitLabArgs {
    Login(LoginArgs),
    Whoami,
    Groups,
    Projects(ProjectsArgs),
    Clone(CloneArgs),
}

#[derive(Debug, clap::Args)]
pub struct LoginArgs {
    #[arg(long, help = "GitLab server address")]
    pub server: String,
    #[arg(long, help = "GitLab access token")]
    pub token: String,
}

#[derive(Debug, clap::Args)]
pub struct ProjectsArgs {
    #[arg(long, help = "Filter by group name")]
    pub group: Option<String>,
    #[arg(long, default_value = "false", help = "Include archived projects")]
    pub archived: bool,
}

#[derive(Debug, clap::Args)]
pub struct CloneArgs {
    #[arg(help = "Group name to clone projects from")]
    pub group: String,
    #[command(flatten)]
    pub repo_path: crate::commands::RepoPathArgs,
    #[arg(long, default_value = "false", help = "Include archived projects")]
    pub archived: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Only list projects without cloning"
    )]
    pub dry_run: bool,
}

pub fn run(args: GitLabArgs) -> anyhow::Result<()> {
    match args {
        GitLabArgs::Login(args) => do_login(args),
        GitLabArgs::Whoami => do_whoami(),
        GitLabArgs::Groups => do_list_groups(),
        GitLabArgs::Projects(args) => do_list_projects(args),
        GitLabArgs::Clone(args) => do_clone(args),
    }
}

fn do_login(args: LoginArgs) -> anyhow::Result<()> {
    if args.server.is_empty() {
        return Err(AppError::invalid_input("服务器地址不能为空").into());
    }

    if args.token.is_empty() {
        return Err(AppError::invalid_input("Token 不能为空").into());
    }

    let client = crate::domain::gitlab::client::GitLabClient::with_url_and_token(
        &args.server,
        &args.token,
    );

    let user = client
        .get_current_user()
        .context("认证失败")
        .map_err(|e| AppError::config(format!("认证失败: {}", e)))?;

    let mut gitlab_config = ConfigDir::load_gitlab();
    if let Some(existing) = gitlab_config
        .servers
        .iter_mut()
        .find(|s| s.url == args.server)
    {
        existing.token = args.token;
        if !args.server.starts_with("http") {
            existing.url = format!("https://{}", args.server);
        } else {
            existing.url = args.server;
        }
    } else {
        gitlab_config.servers.push(GitLabServer {
            url: if args.server.starts_with("http") {
                args.server
            } else {
                format!("https://{}", args.server)
            },
            token: args.token,
            protocol: "HTTPS".to_string(),
        });
    }

    ConfigDir::save_gitlab(&gitlab_config)
        .map_err(|e| AppError::config(format!("保存配置失败: {}", e)))?;

    Output::success("GitLab 配置已保存");
    Output::item("用户", &user.name);
    Output::item("用户名", &user.username);

    Ok(())
}

fn do_whoami() -> anyhow::Result<()> {
    let client = crate::domain::gitlab::client::GitLabClient::new()
        .map_err(|e| AppError::config(format!("初始化 GitLab 客户端失败: {}", e)))?;

    let user = client.get_current_user()?;

    Output::item("用户", &user.name);
    Output::item("用户名", &user.username);

    Ok(())
}

fn do_list_groups() -> anyhow::Result<()> {
    let client = crate::domain::gitlab::client::GitLabClient::new()
        .map_err(|e| AppError::config(format!("初始化 GitLab 客户端失败: {}", e)))?;

    let groups = client.get_groups()?;

    Output::section("GitLab 组列表:");
    for group in &groups {
        Output::message(&format!("  {} (id: {})", group.full_path, group.id));
    }
    Output::blank();
    Output::item("汇总", &format!("共 {} 个组", groups.len()));

    Ok(())
}

fn do_list_projects(args: ProjectsArgs) -> anyhow::Result<()> {
    let client = crate::domain::gitlab::client::GitLabClient::new()
        .map_err(|e| AppError::config(format!("初始化 GitLab 客户端失败: {}", e)))?;

    let groups = client
        .get_groups()
        .map_err(|e| AppError::config(format!("获取组列表失败: {}", e)))?;

    let target_groups = match &args.group {
        Some(name) => groups
            .into_iter()
            .filter(|g| g.name.contains(name) || g.full_path.contains(name))
            .collect(),
        None => groups,
    };

    Output::section("GitLab 项目列表:");
    let mut total = 0;
    for group in target_groups {
        let projects = client
            .get_group_projects(group.id, true, args.archived)
            .map_err(|e| AppError::config(format!("获取项目列表失败: {}", e)))?;
        if !projects.is_empty() {
            Output::message(&format!(
                "组: {} (共 {} 个项目)",
                group.full_path,
                projects.len()
            ));
            for project in &projects {
                let ssh = project.ssh_url.as_deref().unwrap_or("N/A");
                let http = project.http_url.as_deref().unwrap_or("N/A");
                Output::detail(ssh, http);
            }
            total += projects.len();
        }
    }
    Output::blank();
    Output::item("汇总", &format!("共 {} 个项目", total));

    Ok(())
}

fn do_clone(args: CloneArgs) -> anyhow::Result<()> {
    let client = crate::domain::gitlab::client::GitLabClient::new()
        .map_err(|e| AppError::config(format!("初始化 GitLab 客户端失败: {}", e)))?;

    let groups = client
        .get_groups()
        .map_err(|e| AppError::config(format!("获取组列表失败: {}", e)))?;

    let target_group = groups
        .iter()
        .find(|g| g.name == args.group || g.full_path.contains(&args.group))
        .ok_or_else(|| AppError::not_found(format!("未找到组: {}", args.group)))?;

    let projects = client
        .get_group_projects(target_group.id, true, args.archived)
        .map_err(|e| AppError::config(format!("获取项目列表失败: {}", e)))?;

    let clone_dir = crate::utils::path::canonicalize_path(args.repo_path.path.as_str())?;

    for (idx, project) in projects.iter().enumerate() {
        let project_path = project.path.clone();
        let project_dir = clone_dir.join(&project_path);
        if project_dir.exists() {
            Output::skip(&format!("项目已存在: {}", project_path));
            continue;
        }

        let ssh_url = match &project.ssh_url {
            Some(url) => url.clone(),
            None => {
                Output::warning(&format!("项目 {} 缺少 SSH URL，跳过", project_path));
                continue;
            }
        };

        if args.dry_run {
            Output::skip(&format!("git clone {}", ssh_url));
            continue;
        }

        Output::section(&format!(
            "({}/{}) 克隆项目: {}",
            idx + 1,
            projects.len(),
            project_path
        ));
        Output::message(&format!("URL: {}", ssh_url));
        Output::message(&format!("目标: {}", project_dir.display()));

        let runner = crate::domain::git::command::GitCommandRunner::new();
        match runner.execute_streaming(&["clone", &ssh_url], &clone_dir) {
            Ok(()) => Output::success(&format!("项目已克隆: {}", project_path)),
            Err(e) => Output::error(&format!("克隆失败: {}", e)),
        }
    }

    Output::blank();
    Output::item("汇总", &format!("共 {} 个项目", projects.len()));
    Ok(())
}
