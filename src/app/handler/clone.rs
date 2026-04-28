use crate::app::common::runner::CommandRunner;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::Deserialize;
use std::path::Path;

#[derive(ValueEnum, Clone, Debug)]
pub enum CloneProtocol {
    Ssh,
    Https,
}

use clap::ValueEnum;

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

#[allow(clippy::too_many_arguments)]
pub fn execute(
    group: &str,
    base_url: &str,
    token: Option<&str>,
    protocol: &CloneProtocol,
    output_dir: &str,
    include_archived: bool,
    recursive: bool,
    dry_run: bool,
) -> Result<()> {
    let resolved_token = resolve_token(token)?;
    let resolved_base_url = resolve_base_url(base_url);

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

    let projects = fetch_group_projects(
        &resolved_base_url,
        group,
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
            println!(
                "  {} 创建目录: {}",
                "[DRY-RUN]".yellow(),
                output_dir.cyan()
            );
        } else {
            std::fs::create_dir_all(output_path)
                .with_context(|| format!("无法创建目录: {}", output_dir))?;
        }
    }

    let mut success_count = 0usize;
    let mut skip_count = 0usize;
    let mut fail_count = 0usize;

    for (index, project) in projects.iter().enumerate() {
        let progress = format!("({}/{})", index + 1, projects.len());
        let clone_url = match protocol {
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

        if dry_run {
            println!(
                "  {} git clone {} {}",
                "[DRY-RUN]".yellow(),
                clone_url.green(),
                project.name.dimmed()
            );
            if recursive {
                println!(
                    "  {} (含子模块)",
                    "[DRY-RUN]".yellow()
                );
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
                println!(
                    "  {} {}",
                    "已克隆".green(),
                    project.name.green()
                );
                success_count += 1;
            }
            Err(e) => {
                println!(
                    "  {} {} - {}",
                    "克隆失败".red(),
                    project.name.red(),
                    e
                );
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

fn resolve_token(token: Option<&str>) -> Result<String> {
    if let Some(t) = token {
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
         1. 命令行参数 --token <TOKEN>\n\
         2. 环境变量 GITLAB_TOKEN\n\
         3. 环境变量 GL_TOKEN\n\n\
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
        .set("User-Agent", "pma-clone")
        .call()
        .with_context(|| format!("无法获取组信息: {}", group))?;

    let group_info: GitLabGroup = resp
        .into_json()
        .with_context(|| "无法解析组信息")?;

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
            .set("User-Agent", "pma-clone")
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
