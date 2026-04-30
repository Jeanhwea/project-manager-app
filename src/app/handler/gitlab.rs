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

#[derive(Debug, Deserialize)]
struct GitLabSession {
    private_token: String,
    username: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct GitLabPersonalAccessToken {
    token: String,
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

        let username = prompt_input("用户名")?;
        let password = prompt_password("密码")?;

        if username.is_empty() || password.is_empty() {
            anyhow::bail!("用户名和密码不能为空");
        }

        println!();
        println!("{}", "正在认证...".cyan());

        let session = create_session(&resolved_url, &username, &password)?;

        println!(
            "{} {} ({})",
            "已认证:".green(),
            session.name.green().bold(),
            session.username.yellow()
        );

        let pat_token = create_personal_access_token(&resolved_url, &session.private_token)?;

        revoke_session(&resolved_url, &session.private_token).ok();

        println!(
            "{} 已创建 Personal Access Token: {}",
            "令牌:".green(),
            pat_token.name.cyan()
        );

        pat_token.token
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

fn prompt_password(prompt: &str) -> Result<String> {
    let password = rpassword::prompt_password(format!("{}: ", prompt.white().bold()))?;
    Ok(password)
}

fn create_session(base_url: &str, login: &str, password: &str) -> Result<GitLabSession> {
    let url = format!("{}/api/v4/session", base_url);

    let resp = match ureq::post(&url)
        .set("User-Agent", "pma-gitlab")
        .send_form(&[("login", login), ("password", password)])
    {
        Ok(r) => r,
        Err(ureq::Error::Status(code, response)) => {
            let error_msg = match code {
                404 => format!(
                    "Session API 不可用 (HTTP 404)。\n\
                     该 GitLab 服务器可能禁用了用户名密码登录。\n\
                     请使用 --token 参数直接提供 Personal Access Token:\n\
                     pma gitlab login --server {} --token <YOUR_TOKEN>",
                    base_url
                ),
                401 => "用户名或密码错误 (HTTP 401)".to_string(),
                403 => "禁止访问 (HTTP 403)，请检查账户权限".to_string(),
                _ => format!("认证失败 (HTTP {}): {}", code, response.status_text()),
            };
            anyhow::bail!("{}", error_msg);
        }
        Err(e) => {
            anyhow::bail!("无法连接到 {}: {}", base_url, e);
        }
    };

    let session: GitLabSession = resp.into_json().with_context(|| "无法解析会话信息")?;

    Ok(session)
}

fn create_personal_access_token(
    base_url: &str,
    session_token: &str,
) -> Result<GitLabPersonalAccessToken> {
    let url = format!("{}/api/v4/personal_access_tokens", base_url);

    let body = serde_json::json!({
        "name": "pma-cli",
        "scopes": ["api", "read_api", "read_repository", "write_repository"]
    });

    let resp = ureq::post(&url)
        .set("PRIVATE-TOKEN", session_token)
        .set("User-Agent", "pma-gitlab")
        .set("Content-Type", "application/json")
        .send_json(&body)
        .with_context(|| "无法创建 Personal Access Token")?;

    if resp.status() != 201 {
        anyhow::bail!("创建 Personal Access Token 失败 (HTTP {})", resp.status());
    }

    let pat: GitLabPersonalAccessToken = resp.into_json().with_context(|| "无法解析令牌信息")?;

    Ok(pat)
}

fn revoke_session(base_url: &str, session_token: &str) -> Result<()> {
    let url = format!("{}/api/v4/session", base_url);

    ureq::delete(&url)
        .set("PRIVATE-TOKEN", session_token)
        .set("User-Agent", "pma-gitlab")
        .call()
        .ok();

    Ok(())
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
    let (resolved_url, saved_token, saved_protocol) =
        resolve_gitlab_config(server, token, protocol)?;

    let resolved_base_url = resolve_base_url(&resolved_url);
    let resolved_token = resolve_token(Some(&saved_token))?;
    let resolved_protocol = saved_protocol;

    println!(
        "{} {} {}/",
        "克隆组:".cyan(),
        resolved_base_url.dimmed(),
        group.yellow().bold()
    );

    let group_info = fetch_group_info(&resolved_base_url, group, &resolved_token)?;

    println!(
        "{} {} ({})",
        "组名:".cyan(),
        group_info.name.green().bold(),
        group_info.full_path.dimmed()
    );

    let projects =
        fetch_group_projects(&resolved_base_url, group, &resolved_token, include_archived)?;

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
