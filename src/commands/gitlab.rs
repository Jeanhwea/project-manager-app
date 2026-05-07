use super::{Command, CommandError, CommandResult};
use crate::domain::config::{ConfigDir, GitLabServer};
use crate::domain::git::command::GitCommandRunner;
use crate::domain::git::repository::is_git_repo;
use crate::domain::gitlab::client::GitLabClient;
use crate::domain::gitlab::models::User;
use crate::utils::output::{ItemColor, Output};

use std::collections::HashSet;
use std::io::{self, Write};
use std::path::Path;

/// GitLab command arguments
#[derive(Debug, clap::Subcommand)]
pub enum GitLabArgs {
    /// Login to a GitLab server and save credentials
    Login(LoginArgs),
    /// Clone all repositories from a GitLab group
    #[command(visible_alias = "cl")]
    Clone(CloneArgs),
}

/// Login command arguments
#[derive(Debug, clap::Args)]
pub struct LoginArgs {
    /// GitLab server URL (will prompt if not provided)
    #[arg(
        long,
        short,
        help = "GitLab server URL (e.g. https://gitlab.com, http://192.168.0.110/gitlab/)"
    )]
    pub server: Option<String>,
    /// GitLab Personal Access Token (required, will prompt if not provided)
    #[arg(long, short = 't', help = "GitLab Personal Access Token (required)")]
    pub token: Option<String>,
    /// Default clone protocol
    #[arg(
        long,
        short = 'p',
        value_enum,
        default_value = "ssh",
        help = "Default clone protocol: ssh or https"
    )]
    pub protocol: CloneProtocol,
}

/// Clone command arguments
#[derive(Debug, clap::Args)]
pub struct CloneArgs {
    /// GitLab group path (e.g. "my-org/team" or numeric ID)
    #[arg(help = "GitLab group path (e.g. \"my-org/team\" or numeric ID)")]
    pub group: String,
    /// GitLab server URL (uses saved config if not specified)
    #[arg(
        long,
        short,
        help = "GitLab server URL (uses saved config if not specified)"
    )]
    pub server: Option<String>,
    /// GitLab private token (overrides saved config)
    #[arg(
        long,
        short = 't',
        help = "GitLab private token (overrides saved config)"
    )]
    pub token: Option<String>,
    /// Clone protocol (overrides saved config)
    #[arg(
        long,
        short = 'p',
        value_enum,
        help = "Clone protocol: ssh or https (uses saved config if not specified)"
    )]
    pub protocol: Option<CloneProtocol>,
    /// Output directory for cloned repositories
    #[arg(
        long,
        short = 'o',
        default_value = ".",
        help = "Output directory for cloned repositories"
    )]
    pub output: String,
    /// Include archived projects
    #[arg(long, default_value = "false", help = "Include archived projects")]
    pub include_archived: bool,
    /// Clone submodules recursively
    #[arg(long, default_value = "false", help = "Clone submodules recursively")]
    pub recursive: bool,
    /// Dry run: show what would be changed without making any modifications
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be changed without making any modifications"
    )]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum CloneProtocol {
    Ssh,
    Https,
}

pub struct GitLabCommand;

impl Command for GitLabCommand {
    type Args = GitLabArgs;

    fn execute(args: Self::Args) -> CommandResult {
        match args {
            GitLabArgs::Login(login_args) => execute_login(login_args),
            GitLabArgs::Clone(clone_args) => execute_clone(clone_args),
        }
    }
}

fn execute_login(args: LoginArgs) -> CommandResult {
    let resolved_url = if let Some(ref s) = args.server {
        resolve_base_url(s)
    } else {
        Output::section("GitLab 登录");
        let server_url =
            prompt_input("服务器地址 (例如 https://gitlab.com 或 http://192.168.0.110/gitlab/)")?;
        if server_url.is_empty() {
            return Err(CommandError::Validation("服务器地址不能为空".to_string()));
        }
        resolve_base_url(&server_url)
    };

    let final_token = if let Some(ref t) = args.token {
        t.clone()
    } else {
        Output::item("登录到", &resolved_url);

        let input_token = prompt_input("Personal Access Token")?;

        if input_token.is_empty() {
            return Err(CommandError::Validation(
                "Personal Access Token 不能为空\n\
                 在 GitLab 中创建令牌: Settings -> Access Tokens (需要 api 权限)"
                    .to_string(),
            ));
        }

        input_token
    };

    Output::item("验证连接", &resolved_url);

    let client = GitLabClient::with_url_and_token(&resolved_url, &final_token);
    let user: User = client
        .get_current_user()
        .map_err(|e| CommandError::ExecutionFailed(format!("认证失败: {}", e)))?;

    Output::item_colored(
        "已认证",
        &format!("{} ({})", user.name, user.username),
        ItemColor::Green,
    );

    let protocol_str = match args.protocol {
        CloneProtocol::Ssh => "ssh",
        CloneProtocol::Https => "https",
    };

    let mut gitlab_cfg = ConfigDir::load_gitlab();

    let new_server = GitLabServer {
        url: resolved_url.clone(),
        token: final_token.clone(),
        protocol: protocol_str.to_string(),
    };

    if let Some(existing) = gitlab_cfg
        .servers
        .iter_mut()
        .find(|s| s.url == resolved_url)
    {
        existing.token = new_server.token;
        existing.protocol = new_server.protocol;
    } else {
        gitlab_cfg.servers.push(new_server);
    }

    ConfigDir::save_gitlab(&gitlab_cfg)
        .map_err(|e| CommandError::ExecutionFailed(format!("保存配置失败: {}", e)))?;

    Output::success(&format!("{} 凭据已保存", resolved_url));

    Ok(())
}

fn execute_clone(args: CloneArgs) -> CommandResult {
    let (extracted_server, extracted_group) = parse_gitlab_url(&args.group);

    let final_server = extracted_server.as_deref().or(args.server.as_deref());
    let final_group = extracted_group.as_deref().unwrap_or(&args.group);

    let (resolved_url, saved_token, saved_protocol) =
        resolve_gitlab_config(final_server, args.token.as_deref(), args.protocol)?;

    let resolved_base_url = resolve_base_url(&resolved_url);
    let resolved_token = resolve_token(Some(&saved_token))?;
    let resolved_protocol = saved_protocol;

    Output::item_colored(
        "克隆组",
        &format!("{}/{}", resolved_base_url, final_group),
        ItemColor::Yellow,
    );

    let client = GitLabClient::with_url_and_token(&resolved_base_url, &resolved_token);

    let groups = client
        .get_groups()
        .map_err(|e| CommandError::ExecutionFailed(format!("获取组列表失败: {}", e)))?;

    let group_info = groups
        .into_iter()
        .find(|g| g.full_path == *final_group)
        .ok_or_else(|| CommandError::ExecutionFailed(format!("未找到组: {}", final_group)))?;

    Output::item_colored(
        "组名",
        &format!("{} ({})", group_info.name, group_info.full_path),
        ItemColor::Green,
    );

    let projects = client
        .get_group_projects(group_info.id, true, args.include_archived)
        .map_err(|e| CommandError::ExecutionFailed(format!("获取项目列表失败: {}", e)))?;

    if projects.is_empty() {
        Output::warning("未找到项目");
        return Ok(());
    }

    let project_count = format!(
        "{} 个项目{}",
        projects.len(),
        if args.include_archived {
            " (含已归档)"
        } else {
            ""
        }
    );
    Output::item("发现", &project_count);

    let output_path = Path::new(&args.output);
    if !output_path.exists() {
        if args.dry_run {
            Output::dry_run_header(&format!("创建目录: {}", args.output));
        } else {
            std::fs::create_dir_all(output_path).map_err(CommandError::Io)?;
        }
    }

    let existing_urls = collect_existing_remote_urls(output_path);
    if !existing_urls.is_empty() {
        Output::item("检测", &format!("{} 个已存在的仓库", existing_urls.len()));
    }

    let mut success_count = 0usize;
    let mut skip_count = 0usize;
    let mut fail_count = 0usize;

    let group_prefix = format!("{}/", group_info.full_path);

    for (index, project) in projects.iter().enumerate() {
        let progress = format!("({}/{})", index + 1, projects.len());

        let relative_path = project
            .path_with_namespace
            .strip_prefix(&group_prefix)
            .unwrap_or(&project.path);

        if relative_path.contains('/') {
            Output::skip(&format!(
                "{} {} (子组项目)",
                progress, project.path_with_namespace
            ));
            skip_count += 1;
            continue;
        }

        let ssh_url = project.ssh_url.as_deref().unwrap_or("");
        let http_url = project.http_url.as_deref().unwrap_or("");

        let clone_url = match resolved_protocol {
            CloneProtocol::Ssh => ssh_url,
            CloneProtocol::Https => http_url,
        };

        let project_dir = output_path.join(relative_path);
        let is_archived = project.archived.unwrap_or(false);

        let status = if is_archived { "[归档] " } else { "" };
        Output::message(&format!(
            "{} {} {}",
            progress, status, project.path_with_namespace
        ));

        if project_dir.exists() {
            Output::skip(&format!("{} 已存在，跳过", relative_path));
            skip_count += 1;
            continue;
        }

        if existing_urls.contains(clone_url)
            || existing_urls.contains(ssh_url)
            || existing_urls.contains(http_url)
        {
            Output::skip(&format!("{} URL 已存在，跳过", relative_path));
            skip_count += 1;
            continue;
        }

        if args.dry_run {
            Output::info(&format!("git clone {} {}", clone_url, relative_path));
            if args.recursive {
                Output::info("(含子模块)");
            }
            success_count += 1;
            continue;
        }

        let project_dir_str = project_dir.to_string_lossy().to_string();
        let mut git_args = vec!["clone", clone_url, &project_dir_str];
        if args.recursive {
            git_args.push("--recursive");
        }

        let runner = GitCommandRunner::new();
        match runner.execute_in_dir(&git_args, output_path) {
            Ok(_) => {
                Output::success(&format!("已克隆 {}", relative_path));
                success_count += 1;
            }
            Err(e) => {
                Output::error(&format!("克隆失败 {} - {}", relative_path, e));
                fail_count += 1;
            }
        }
    }

    Output::blank();
    Output::item_colored(
        "汇总",
        &format!(
            "成功 {}, 跳过 {}, 失败 {}",
            success_count, skip_count, fail_count
        ),
        ItemColor::Cyan,
    );

    Ok(())
}

fn prompt_input(prompt: &str) -> Result<String, CommandError> {
    print!("{}: ", prompt);
    io::stdout().flush().map_err(CommandError::Io)?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(CommandError::Io)?;
    Ok(input.trim().to_string())
}

fn resolve_gitlab_config(
    server: Option<&str>,
    token: Option<&str>,
    protocol: Option<CloneProtocol>,
) -> Result<(String, String, CloneProtocol), CommandError> {
    let gitlab_cfg = ConfigDir::load_gitlab();

    if let Some(s) = server {
        let resolved_url = resolve_base_url(s);

        let matching = gitlab_cfg
            .servers
            .iter()
            .find(|srv| resolve_base_url(&srv.url) == resolved_url);

        let resolved_token = token
            .map(|t| t.to_string())
            .or_else(|| matching.map(|m| m.token.clone()))
            .unwrap_or_default();

        let resolved_protocol = protocol
            .or_else(|| matching.and_then(|m| parse_protocol_str(&m.protocol)))
            .unwrap_or(CloneProtocol::Ssh);

        return Ok((resolved_url, resolved_token, resolved_protocol));
    }

    if let Some(first) = gitlab_cfg.servers.first() {
        let resolved_url = resolve_base_url(&first.url);

        let resolved_token = token
            .map(|t| t.to_string())
            .or_else(|| Some(first.token.clone()))
            .unwrap_or_default();

        let resolved_protocol = protocol
            .or_else(|| parse_protocol_str(&first.protocol))
            .unwrap_or(CloneProtocol::Ssh);

        return Ok((resolved_url, resolved_token, resolved_protocol));
    }

    let default_url = "https://gitlab.com".to_string();
    let resolved_token = token.map(|t| t.to_string()).unwrap_or_default();
    let resolved_protocol = protocol.unwrap_or(CloneProtocol::Ssh);
    Ok((default_url, resolved_token, resolved_protocol))
}

fn parse_protocol_str(s: &str) -> Option<CloneProtocol> {
    match s {
        "https" => Some(CloneProtocol::Https),
        "ssh" => Some(CloneProtocol::Ssh),
        _ => None,
    }
}

fn resolve_token(token: Option<&str>) -> Result<String, CommandError> {
    if let Some(t) = token
        && !t.is_empty()
    {
        return Ok(t.to_string());
    }

    if let Ok(t) = std::env::var("GITLAB_TOKEN") {
        return Ok(t);
    }

    if let Ok(t) = std::env::var("GL_TOKEN") {
        return Ok(t);
    }

    Err(CommandError::Validation(
        "未提供 GitLab 访问令牌。\n\
         请通过以下方式之一提供：\n\
         1. pma gitlab login --server <URL> --token <TOKEN>\n\
         2. 命令行参数 --token <TOKEN>\n\
         3. 环境变量 GITLAB_TOKEN 或 GL_TOKEN\n\n\
         在 GitLab 中创建令牌: Settings -> Access Tokens (需要 read_api 权限)"
            .to_string(),
    ))
}

fn resolve_base_url(base_url: &str) -> String {
    let url = base_url.trim_end_matches('/');
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}

fn parse_gitlab_url(input: &str) -> (Option<String>, Option<String>) {
    let input = input.trim_end_matches('/');

    if !input.starts_with("http://") && !input.starts_with("https://") {
        return (None, None);
    }

    let parsed = match url::Url::parse(input) {
        Ok(u) => u,
        Err(_) => return (None, None),
    };

    let host = match parsed.host_str() {
        Some(h) => h,
        None => return (None, None),
    };

    let port = parsed.port().map(|p| format!(":{}", p)).unwrap_or_default();
    let scheme = parsed.scheme();

    let path = parsed.path().trim_start_matches('/');

    if path.is_empty() {
        return (None, None);
    }

    let path_segments: Vec<&str> = path.split('/').collect();

    let common_subpaths = ["gitlab", "gitlab-ce", "gitlab-ee"];

    let (base_path, group_path) =
        if path_segments.len() >= 2 && common_subpaths.contains(&path_segments[0]) {
            let base = path_segments[0];
            let group = path_segments[1..].join("/");
            (format!("/{}", base), group)
        } else {
            (String::new(), path.to_string())
        };

    let server_url = format!("{}://{}{}{}", scheme, host, port, base_path);
    let group_path = group_path.trim_end_matches(".git").to_string();

    if group_path.is_empty() {
        return (None, None);
    }

    (Some(server_url), Some(group_path))
}

fn collect_existing_remote_urls(output_path: &Path) -> HashSet<String> {
    let mut urls = HashSet::new();

    if !output_path.exists() {
        return urls;
    }

    let runner = GitCommandRunner::new();
    if let Ok(entries) = std::fs::read_dir(output_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && is_git_repo(&path)
                && let Ok(remote_urls) = runner.get_remote_urls(&path)
            {
                for url in remote_urls {
                    urls.insert(url.trim_end_matches('/').to_string());
                }
            }
        }
    }

    urls
}
