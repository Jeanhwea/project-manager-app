use super::{Command, CommandError, CommandResult};
use crate::domain::config::{ConfigDir, GitLabServer};
use crate::domain::gitlab::client::GitLabClient;
use crate::domain::gitlab::models::User;
use crate::utils::git::{get_remote_urls, git_command, is_git_repo};

use colored::Colorize;
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::Path;

/// GitLab command arguments
#[derive(Debug)]
pub enum GitLabArgs {
    /// Login to a GitLab server and save credentials
    Login(LoginArgs),
    /// Clone all repositories from a GitLab group
    Clone(CloneArgs),
}

/// Login command arguments
#[derive(Debug)]
pub struct LoginArgs {
    /// GitLab server URL
    pub server: Option<String>,
    /// GitLab Personal Access Token
    pub token: Option<String>,
    /// Default clone protocol
    pub protocol: CloneProtocol,
}

/// Clone command arguments
#[derive(Debug)]
pub struct CloneArgs {
    /// GitLab group path (e.g. "my-org/team" or numeric ID)
    pub group: String,
    /// GitLab server URL (uses saved config if not specified)
    pub server: Option<String>,
    /// GitLab private token (overrides saved config)
    pub token: Option<String>,
    /// Clone protocol (overrides saved config)
    pub protocol: Option<CloneProtocol>,
    /// Output directory for cloned repositories
    pub output: String,
    /// Include archived projects
    pub include_archived: bool,
    /// Clone submodules recursively
    pub recursive: bool,
    /// Dry run: show what would be changed without making any modifications
    pub dry_run: bool,
}

/// Clone protocol enumeration
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum CloneProtocol {
    Ssh,
    Https,
}

/// GitLab command
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

/// Execute login command
fn execute_login(args: LoginArgs) -> CommandResult {
    let resolved_url = if let Some(ref s) = args.server {
        resolve_base_url(s)
    } else {
        println!("{}", "GitLab 登录".cyan().bold());
        println!();
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
        println!("{} {}", "登录到:".cyan(), resolved_url.dimmed());
        println!();

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

    println!("{} {}", "验证连接:".cyan(), resolved_url.dimmed());

    let client = GitLabClient::with_url_and_token(&resolved_url, &final_token);
    let user: User = client
        .get_current_user()
        .map_err(|e| CommandError::ExecutionFailed(format!("Failed to authenticate: {}", e)))?;

    println!(
        "{} {} ({})",
        "已认证:".green(),
        user.name.green().bold(),
        user.username.yellow()
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
        .map_err(|e| CommandError::ExecutionFailed(format!("Failed to save config: {}", e)))?;

    println!("{} {} 凭据已保存", "保存:".green(), resolved_url.cyan());

    Ok(())
}

/// Execute clone command
fn execute_clone(args: CloneArgs) -> CommandResult {
    // Try to extract server and group from URL
    let (extracted_server, extracted_group) = parse_gitlab_url(&args.group);

    let final_server = extracted_server.as_deref().or(args.server.as_deref());
    let final_group = extracted_group.as_deref().unwrap_or(&args.group);

    let (resolved_url, saved_token, saved_protocol) =
        resolve_gitlab_config(final_server, args.token.as_deref(), args.protocol)?;

    let resolved_base_url = resolve_base_url(&resolved_url);
    let resolved_token = resolve_token(Some(&saved_token))?;
    let resolved_protocol = saved_protocol;

    println!(
        "{} {} {}/",
        "克隆组:".cyan(),
        resolved_base_url.dimmed(),
        final_group.yellow().bold()
    );

    let client = GitLabClient::with_url_and_token(&resolved_base_url, &resolved_token);

    // Get group information
    let groups = client
        .get_groups()
        .map_err(|e| CommandError::ExecutionFailed(format!("Failed to get groups: {}", e)))?;

    let group_info = groups
        .into_iter()
        .find(|g| g.full_path == *final_group)
        .ok_or_else(|| {
            CommandError::ExecutionFailed(format!("Group not found: {}", final_group))
        })?;

    println!(
        "{} {} ({})",
        "组名:".cyan(),
        group_info.name.green().bold(),
        group_info.full_path.dimmed()
    );

    let projects = client
        .get_group_projects(group_info.id, true, args.include_archived)
        .map_err(|e| CommandError::ExecutionFailed(format!("Failed to get projects: {}", e)))?;

    if projects.is_empty() {
        println!("{}", "未找到项目".yellow());
        return Ok(());
    }

    println!(
        "{} {} 个项目{}",
        "发现:".cyan(),
        projects.len().to_string().white().bold(),
        if args.include_archived {
            " (含已归档)"
        } else {
            ""
        }
    );
    println!();

    let output_path = Path::new(&args.output);
    if !output_path.exists() {
        if args.dry_run {
            println!(
                "  {} 创建目录: {}",
                "[DRY-RUN]".yellow(),
                args.output.cyan()
            );
        } else {
            std::fs::create_dir_all(output_path).map_err(CommandError::Io)?;
        }
    }

    let existing_urls = collect_existing_remote_urls(output_path);
    if !existing_urls.is_empty() {
        println!(
            "{} {} 个已存在的仓库",
            "检测:".cyan(),
            existing_urls.len().to_string().white().bold()
        );
        println!();
    }

    let mut success_count = 0usize;
    let mut skip_count = 0usize;
    let mut fail_count = 0usize;

    // Strip the group prefix from path_with_namespace to get the project path
    // relative to the target group.
    // e.g. group="my-org", path_with_namespace="my-org/api" → "api"
    //      group="my-org", path_with_namespace="my-org/sub/api" → "sub/api" (子组，跳过)
    let group_prefix = format!("{}/", group_info.full_path);

    for (index, project) in projects.iter().enumerate() {
        let progress = format!("({}/{})", index + 1, projects.len());

        // Determine relative path within the group
        let relative_path = project
            .path_with_namespace
            .strip_prefix(&group_prefix)
            .unwrap_or(&project.path);

        // Skip projects in subgroups — only clone direct children
        if relative_path.contains('/') {
            println!(
                "{} {} {}",
                progress.white().bold(),
                "[SKIP]".dimmed(),
                format!("{} (子组项目)", project.path_with_namespace).dimmed(),
            );
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

        println!(
            "{} {} {}",
            progress.white().bold(),
            if is_archived {
                "[归档]".dimmed()
            } else {
                "".dimmed()
            },
            project.path_with_namespace.cyan(),
        );

        if project_dir.exists() {
            println!(
                "  {} {} 已存在，跳过",
                "[SKIP]".dimmed(),
                relative_path.yellow()
            );
            skip_count += 1;
            continue;
        }

        if existing_urls.contains(clone_url)
            || existing_urls.contains(ssh_url)
            || existing_urls.contains(http_url)
        {
            println!(
                "  {} {} URL 已存在，跳过",
                "[SKIP]".dimmed(),
                relative_path.yellow()
            );
            skip_count += 1;
            continue;
        }

        if args.dry_run {
            println!(
                "  {} git clone {} {}",
                "[DRY-RUN]".yellow(),
                clone_url.green(),
                relative_path.dimmed()
            );
            if args.recursive {
                println!("  {} (含子模块)", "[DRY-RUN]".yellow());
            }
            success_count += 1;
            continue;
        }

        let project_dir_str = project_dir.to_string_lossy().to_string();
        let mut git_args = vec!["clone", clone_url, &project_dir_str];
        if args.recursive {
            git_args.push("--recursive");
        }

        match git_command(output_path, &git_args) {
            Ok(_) => {
                println!("  {} {}", "已克隆".green(), relative_path.green());
                success_count += 1;
            }
            Err(e) => {
                println!("  {} {} - {}", "克隆失败".red(), relative_path.red(), e);
                fail_count += 1;
            }
        }
    }

    println!();
    println!(
        "{} 成功 {}, 跳过 {}, 失败 {}",
        "汇总:".cyan(),
        success_count.to_string().green(),
        skip_count.to_string().yellow(),
        fail_count.to_string().red(),
    );

    Ok(())
}

/// Prompt for user input
fn prompt_input(prompt: &str) -> Result<String, CommandError> {
    print!("{}: ", prompt.white().bold());
    io::stdout().flush().map_err(CommandError::Io)?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(CommandError::Io)?;
    Ok(input.trim().to_string())
}

/// Resolve GitLab configuration from various sources
///
/// Priority: CLI args > gitlab.toml > defaults
fn resolve_gitlab_config(
    server: Option<&str>,
    token: Option<&str>,
    protocol: Option<CloneProtocol>,
) -> Result<(String, String, CloneProtocol), CommandError> {
    let gitlab_cfg = ConfigDir::load_gitlab();

    // If server is explicitly provided, find matching entry or use defaults
    if let Some(s) = server {
        let resolved_url = resolve_base_url(s);

        // Look for a matching server entry in gitlab.toml
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

    // No server specified — use the first (or only) entry in gitlab.toml
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

    // No config at all — fall back to gitlab.com
    let default_url = "https://gitlab.com".to_string();
    let resolved_token = token.map(|t| t.to_string()).unwrap_or_default();
    let resolved_protocol = protocol.unwrap_or(CloneProtocol::Ssh);
    Ok((default_url, resolved_token, resolved_protocol))
}

/// Parse protocol string to CloneProtocol
fn parse_protocol_str(s: &str) -> Option<CloneProtocol> {
    match s {
        "https" => Some(CloneProtocol::Https),
        "ssh" => Some(CloneProtocol::Ssh),
        _ => None,
    }
}

/// Resolve token from various sources
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

/// Resolve base URL
fn resolve_base_url(base_url: &str) -> String {
    let url = base_url.trim_end_matches('/');
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}

/// Parse GitLab URL to extract server and group/project path
fn parse_gitlab_url(input: &str) -> (Option<String>, Option<String>) {
    let input = input.trim_end_matches('/');

    // Check if it's a URL
    if !input.starts_with("http://") && !input.starts_with("https://") {
        return (None, None);
    }

    // Parse URL
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

    // Get path part
    let path = parsed.path().trim_start_matches('/');

    // If path is empty, not a valid group/project URL
    if path.is_empty() {
        return (None, None);
    }

    // Separate base path and group/project path
    // GitLab may be deployed under subpath, e.g., /gitlab/
    // Group/project path usually doesn't contain .git suffix and contains at least one /

    let path_segments: Vec<&str> = path.split('/').collect();

    // Try to find the starting position of group/project path
    // If the first segment is a common subpath identifier, then base URL includes it
    let common_subpaths = ["gitlab", "gitlab-ce", "gitlab-ee"];

    let (base_path, group_path) =
        if path_segments.len() >= 2 && common_subpaths.contains(&path_segments[0]) {
            // First segment is subpath, remaining is group/project path
            let base = path_segments[0];
            let group = path_segments[1..].join("/");
            (format!("/{}", base), group)
        } else {
            // No subpath, entire path is group/project path
            (String::new(), path.to_string())
        };

    let server_url = format!("{}://{}{}{}", scheme, host, port, base_path);
    let group_path = group_path.trim_end_matches(".git").to_string();

    // Ensure group path is not empty
    if group_path.is_empty() {
        return (None, None);
    }

    (Some(server_url), Some(group_path))
}

/// Collect existing remote URLs from repositories in output path
fn collect_existing_remote_urls(output_path: &Path) -> HashSet<String> {
    let mut urls = HashSet::new();

    if !output_path.exists() {
        return urls;
    }

    if let Ok(entries) = std::fs::read_dir(output_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && is_git_repo(&path)
                && let Ok(remote_urls) = get_remote_urls(&path)
            {
                for url in remote_urls {
                    urls.insert(url.trim_end_matches('/').to_string());
                }
            }
        }
    }

    urls
}
