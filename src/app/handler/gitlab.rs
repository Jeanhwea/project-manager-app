use crate::app::common::config;
use crate::app::common::git;
use crate::app::common::runner::CommandRunner;
use anyhow::{Context, Result};
use clap::ValueEnum;
use colored::Colorize;
use serde::Deserialize;
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::Path;

#[derive(ValueEnum, Clone, Debug)]
pub enum CloneProtocol {
    Ssh,
    Https,
}

#[derive(Debug, Deserialize)]
struct GitLabProject {
    path_with_namespace: String,
    ssh_url_to_repo: String,
    http_url_to_repo: String,
    name: String,
    archived: bool,
}

#[derive(Debug, Deserialize)]
struct GitLabGroup {
    full_path: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct GitLabUser {
    username: String,
    name: String,
}

pub fn execute_login(
    server: Option<&str>,
    token: Option<&str>,
    protocol: &CloneProtocol,
) -> Result<()> {
    let resolved_url = if let Some(s) = server {
        resolve_base_url(s)
    } else {
        println!("{}", "GitLab 登录".cyan().bold());
        println!();
        let server_url =
            prompt_input("服务器地址 (例如 https://gitlab.com 或 http://192.168.0.110/gitlab/)")?;
        if server_url.is_empty() {
            anyhow::bail!("服务器地址不能为空");
        }
        resolve_base_url(&server_url)
    };

    let final_token = if let Some(t) = token {
        t.to_string()
    } else {
        println!("{} {}", "登录到:".cyan(), resolved_url.dimmed());
        println!();

        let input_token = prompt_input("Personal Access Token")?;

        if input_token.is_empty() {
            anyhow::bail!(
                "Personal Access Token 不能为空\n\
                 在 GitLab 中创建令牌: Settings -> Access Tokens (需要 api 权限)"
            );
        }

        input_token
    };

    println!("{} {}", "验证连接:".cyan(), resolved_url.dimmed());

    let user = verify_token(&resolved_url, &final_token)?;

    println!(
        "{} {} ({})",
        "已认证:".green(),
        user.name.green().bold(),
        user.username.yellow()
    );

    let mut gitlab_cfg = config::load_gitlab();

    let protocol_str = match protocol {
        CloneProtocol::Ssh => "ssh",
        CloneProtocol::Https => "https",
    };

    if let Some(existing) = gitlab_cfg
        .servers
        .iter_mut()
        .find(|s| s.url == resolved_url)
    {
        existing.token = final_token.clone();
        existing.protocol = protocol_str.to_string();
        println!("{} {} 的凭据已更新", "更新:".yellow(), resolved_url.cyan());
    } else {
        gitlab_cfg.servers.push(config::GitLabServer {
            url: resolved_url.clone(),
            token: final_token.clone(),
            protocol: protocol_str.to_string(),
        });
        println!("{} {} 凭据已保存", "保存:".green(), resolved_url.cyan());
    }

    config::save_gitlab(&gitlab_cfg)?;

    println!(
        "{} 配置文件: {}",
        "位置:".dimmed(),
        config::gitlab_config_path().display()
    );

    Ok(())
}

fn prompt_input(prompt: &str) -> Result<String> {
    print!("{}: ", prompt.white().bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn verify_token(base_url: &str, token: &str) -> Result<GitLabUser> {
    let url = format!("{}/api/v4/user", base_url);

    let resp = ureq::get(&url)
        .set("PRIVATE-TOKEN", token)
        .set("User-Agent", "pma-gitlab")
        .call()
        .with_context(|| format!("无法连接到 {}", base_url))?;

    let user: GitLabUser = resp.into_json().with_context(|| "无法解析用户信息")?;

    Ok(user)
}

#[allow(clippy::too_many_arguments)]
pub fn execute_clone(
    group: &str,
    server: Option<&str>,
    token: Option<&str>,
    protocol: Option<&CloneProtocol>,
    output_dir: &str,
    include_archived: bool,
    recursive: bool,
    dry_run: bool,
) -> Result<()> {
    // 尝试从 URL 中提取服务器和组路径
    let (extracted_server, extracted_group) = parse_gitlab_url(group);

    let final_server = extracted_server.as_deref().or(server);
    let final_group = extracted_group.as_deref().unwrap_or(group);

    let (resolved_url, saved_token, saved_protocol) =
        resolve_gitlab_config(final_server, token, protocol)?;

    let resolved_base_url = resolve_base_url(&resolved_url);
    let resolved_token = resolve_token(Some(&saved_token))?;
    let resolved_protocol = saved_protocol;

    println!(
        "{} {} {}/",
        "克隆组:".cyan(),
        resolved_base_url.dimmed(),
        final_group.yellow().bold()
    );

    let group_info = fetch_group_info(&resolved_base_url, final_group, &resolved_token)?;

    println!(
        "{} {} ({})",
        "组名:".cyan(),
        group_info.name.green().bold(),
        group_info.full_path.dimmed()
    );

    let projects = fetch_group_projects(
        &resolved_base_url,
        final_group,
        &resolved_token,
        include_archived,
    )?;

    if projects.is_empty() {
        println!("{}", "未找到项目".yellow());
        return Ok(());
    }

    println!(
        "{} {} 个项目{}",
        "发现:".cyan(),
        projects.len().to_string().white().bold(),
        if include_archived {
            " (含已归档)"
        } else {
            ""
        }
    );
    println!();

    let output_path = Path::new(output_dir);
    if !output_path.exists() {
        if dry_run {
            println!("  {} 创建目录: {}", "[DRY-RUN]".yellow(), output_dir.cyan());
        } else {
            std::fs::create_dir_all(output_path)
                .with_context(|| format!("无法创建目录: {}", output_dir))?;
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

    for (index, project) in projects.iter().enumerate() {
        let progress = format!("({}/{})", index + 1, projects.len());
        let clone_url = match resolved_protocol {
            CloneProtocol::Ssh => &project.ssh_url_to_repo,
            CloneProtocol::Https => &project.http_url_to_repo,
        };

        let project_dir = output_path.join(&project.name);

        println!(
            "{} {} {}",
            progress.white().bold(),
            if project.archived {
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
                project.name.yellow()
            );
            skip_count += 1;
            continue;
        }

        if existing_urls.contains(clone_url.as_str())
            || existing_urls.contains(project.ssh_url_to_repo.as_str())
            || existing_urls.contains(project.http_url_to_repo.as_str())
        {
            println!(
                "  {} {} URL 已存在，跳过",
                "[SKIP]".dimmed(),
                project.name.yellow()
            );
            skip_count += 1;
            continue;
        }

        if dry_run {
            println!(
                "  {} git clone {} {}",
                "[DRY-RUN]".yellow(),
                clone_url.green(),
                project.name.dimmed()
            );
            if recursive {
                println!("  {} (含子模块)", "[DRY-RUN]".yellow());
            }
            success_count += 1;
            continue;
        }

        let mut args = vec!["clone", clone_url, &project.name];
        if recursive {
            args.push("--recursive");
        }

        match CommandRunner::run_with_success_in_dir("git", &args, output_path) {
            Ok(_) => {
                println!("  {} {}", "已克隆".green(), project.name.green());
                success_count += 1;
            }
            Err(e) => {
                println!("  {} {} - {}", "克隆失败".red(), project.name.red(), e);
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

fn resolve_gitlab_config(
    server: Option<&str>,
    token: Option<&str>,
    protocol: Option<&CloneProtocol>,
) -> Result<(String, String, CloneProtocol)> {
    let gitlab_cfg = config::load_gitlab();

    if let Some(s) = server {
        let resolved_url = resolve_base_url(s);

        let saved = gitlab_cfg
            .servers
            .iter()
            .find(|srv| srv.url == resolved_url);

        let resolved_token = token
            .map(|t| t.to_string())
            .or_else(|| saved.map(|s| s.token.clone()))
            .unwrap_or_default();

        let resolved_protocol = protocol
            .cloned()
            .or_else(|| {
                saved.map(|s| match s.protocol.as_str() {
                    "https" => CloneProtocol::Https,
                    _ => CloneProtocol::Ssh,
                })
            })
            .unwrap_or(CloneProtocol::Ssh);

        return Ok((resolved_url, resolved_token, resolved_protocol));
    }

    if gitlab_cfg.servers.is_empty() {
        let default_url = "https://gitlab.com".to_string();
        let resolved_token = token.map(|t| t.to_string()).unwrap_or_default();
        let resolved_protocol = protocol.cloned().unwrap_or(CloneProtocol::Ssh);
        return Ok((default_url, resolved_token, resolved_protocol));
    }

    if gitlab_cfg.servers.len() == 1 {
        let srv = &gitlab_cfg.servers[0];
        let resolved_token = token
            .map(|t| t.to_string())
            .unwrap_or_else(|| srv.token.clone());
        let resolved_protocol = protocol
            .cloned()
            .or({
                match srv.protocol.as_str() {
                    "https" => Some(CloneProtocol::Https),
                    _ => Some(CloneProtocol::Ssh),
                }
            })
            .unwrap_or(CloneProtocol::Ssh);
        return Ok((srv.url.clone(), resolved_token, resolved_protocol));
    }

    println!("{}", "已配置的 GitLab 服务器:".cyan());
    for (i, srv) in gitlab_cfg.servers.iter().enumerate() {
        println!(
            "  {} {} ({})",
            format!("[{}]", i + 1).yellow(),
            srv.url.cyan(),
            srv.protocol.dimmed()
        );
    }
    anyhow::bail!("存在多个 GitLab 服务器配置，请使用 --server 指定")
}

fn resolve_token(token: Option<&str>) -> Result<String> {
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

    anyhow::bail!(
        "未提供 GitLab 访问令牌。\n\
         请通过以下方式之一提供:\n\
         1. pma gitlab login --server <URL> --token <TOKEN>\n\
         2. 命令行参数 --token <TOKEN>\n\
         3. 环境变量 GITLAB_TOKEN 或 GL_TOKEN\n\n\
         在 GitLab 中创建令牌: Settings -> Access Tokens (需要 read_api 权限)"
    )
}

fn resolve_base_url(base_url: &str) -> String {
    let url = base_url.trim_end_matches('/');
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}

/// 从 GitLab URL 中提取服务器地址和组/项目路径
/// 例如: "http://192.168.0.110/gitlab/ntfw/fe" -> ("http://192.168.0.110/gitlab", "ntfw/fe")
fn parse_gitlab_url(input: &str) -> (Option<String>, Option<String>) {
    let input = input.trim_end_matches('/');

    // 检查是否是 URL
    if !input.starts_with("http://") && !input.starts_with("https://") {
        return (None, None);
    }

    // 解析 URL
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

    // 获取路径部分
    let path = parsed.path().trim_start_matches('/');

    // 如果路径为空，不是有效的组/项目 URL
    if path.is_empty() {
        return (None, None);
    }

    // 分离基础路径和组/项目路径
    // GitLab 可能部署在子路径下，如 /gitlab/
    // 组/项目路径通常不包含 .git 后缀，且至少包含一个 /

    // 常见的 GitLab 子路径: gitlab, gitlab-ce, gitlab-ee 等
    let path_segments: Vec<&str> = path.split('/').collect();

    // 尝试找到组/项目路径的起始位置
    // 如果第一段是常见的子路径标识，则基础 URL 包含它
    let common_subpaths = ["gitlab", "gitlab-ce", "gitlab-ee"];

    let (base_path, group_path) =
        if path_segments.len() >= 2 && common_subpaths.contains(&path_segments[0]) {
            // 第一段是子路径，剩余的是组/项目路径
            let base = path_segments[0];
            let group = path_segments[1..].join("/");
            (format!("/{}", base), group)
        } else {
            // 没有子路径，整个路径是组/项目路径
            (String::new(), path.to_string())
        };

    let server_url = format!("{}://{}{}{}", scheme, host, port, base_path);
    let group_path = group_path.trim_end_matches(".git").to_string();

    // 确保组路径不为空
    if group_path.is_empty() {
        return (None, None);
    }

    (Some(server_url), Some(group_path))
}

fn fetch_group_info(base_url: &str, group: &str, token: &str) -> Result<GitLabGroup> {
    let encoded_group = url_encode(group);
    let url = format!("{}/api/v4/groups/{}", base_url, encoded_group);

    let resp = ureq::get(&url)
        .set("PRIVATE-TOKEN", token)
        .set("User-Agent", "pma-gitlab")
        .call()
        .with_context(|| format!("无法获取组信息: {}", group))?;

    let group_info: GitLabGroup = resp.into_json().with_context(|| "无法解析组信息")?;

    Ok(group_info)
}

fn fetch_group_projects(
    base_url: &str,
    group: &str,
    token: &str,
    include_archived: bool,
) -> Result<Vec<GitLabProject>> {
    let encoded_group = url_encode(group);
    let mut all_projects = Vec::new();
    let mut page = 1;
    let per_page = 100u32;

    loop {
        let url = format!(
            "{}/api/v4/groups/{}/projects?page={}&per_page={}&include_subgroups=true&order_by=path&sort=asc{}",
            base_url,
            encoded_group,
            page,
            per_page,
            if include_archived {
                "&archived=true"
            } else {
                "&archived=false"
            }
        );

        let resp = ureq::get(&url)
            .set("PRIVATE-TOKEN", token)
            .set("User-Agent", "pma-gitlab")
            .call()
            .with_context(|| format!("无法获取组 {} 的项目列表 (第 {} 页)", group, page))?;

        let projects: Vec<GitLabProject> = resp
            .into_json()
            .with_context(|| format!("无法解析组 {} 的项目列表", group))?;

        let count = projects.len();
        all_projects.extend(projects);

        if count < per_page as usize {
            break;
        }

        page += 1;
    }

    Ok(all_projects)
}

fn url_encode(s: &str) -> String {
    s.replace('%', "%25")
        .replace('/', "%2F")
        .replace(' ', "%20")
        .replace('#', "%23")
}

fn collect_existing_remote_urls(output_path: &Path) -> HashSet<String> {
    let mut urls = HashSet::new();

    if !output_path.exists() {
        return urls;
    }

    let repos = git::find_git_repositories(output_path, Some(1));

    for repo in &repos {
        let remote_info = git::get_remote_info(&repo.path);
        for (_, url) in remote_info {
            let url = url.trim_end_matches('/').to_string();
            urls.insert(url);
        }
    }

    urls
}
