use super::{Command, CommandResult};
use crate::domain::context::AppContext;
use crate::domain::git::GitProtocol;
use crate::domain::runner::DryRunContext;
use anyhow::{Context, Result};
use heck::{ToKebabCase, ToPascalCase};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, clap::Args)]
pub struct ForkArgs {
    /// Path to fork the project from
    #[arg(help = "Path to fork the project from")]
    pub path: String,
    /// Name of the project
    #[arg(help = "Name of the project")]
    pub name: String,
    /// Dry run: show what would be changed without making any modifications
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be changed without making any modifications"
    )]
    pub dry_run: bool,
}

pub struct ForkCommand;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
enum Action {
    Replace {
        str_old: String,
        str_new: String,
        files: Vec<String>,
    },
    AddGitRemote {
        remote_name: String,
        remote_url: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PmaConfig {
    actions: Vec<Action>,
}

#[derive(Debug, Clone)]
struct Submodule {
    path: String,
    url: String,
}

impl Command for ForkCommand {
    type Args = ForkArgs;

    fn execute(args: Self::Args) -> CommandResult {
        let root_dir = Path::new(&args.path);

        if !root_dir.exists() {
            return Err(super::CommandError::Validation(format!(
                "目录不存在: {}",
                args.path
            )));
        }

        let repo_dir = root_dir.join(".git");
        if !repo_dir.exists() {
            return Err(super::CommandError::Validation(format!(
                "Git 仓库目录不存在: {}",
                repo_dir.display()
            )));
        }

        let curr_dir = std::env::current_dir().map_err(super::CommandError::Io)?;
        let project_dir = curr_dir.join(&args.name);

        if project_dir.exists() {
            return Err(super::CommandError::Validation(format!(
                "项目目录已存在: {}",
                project_dir.display()
            )));
        }

        let ctx = DryRunContext::new(args.dry_run);

        if ctx.is_dry_run() {
            ctx.print_header("[DRY-RUN] Operations to be performed:");
            ctx.print_message(&format!("clone {} {}", root_dir.display(), args.name));
            ctx.print_message("Delete .git directory");
        }

        let submodules = get_submodules(root_dir)
            .map_err(|e| super::CommandError::ExecutionFailed(e.to_string()))?;

        if ctx.is_dry_run() {
            for submodule in &submodules {
                ctx.print_message(&format!(
                    "git submodule add {} {}",
                    submodule.url, submodule.path
                ));
            }

            let pma_config = root_dir.join(".pma.json");
            if pma_config.exists() {
                ctx.print_message("Execute actions from .pma.json");
            }

            let remotes = get_remote_info(root_dir)
                .map_err(|e| super::CommandError::ExecutionFailed(e.to_string()))?;
            for (remote_name, remote_url) in remotes {
                if let Some(new_url) = generate_new_remote_url(&remote_url, &args.name) {
                    ctx.print_message(&format!("git remote add {} {}", remote_name, new_url));
                }
            }

            ctx.print_message("git add .");
            ctx.print_message("git commit -m v0.0.0");
            return Ok(());
        }

        let repo_url = repo_dir.to_string_lossy();
        do_init_project(&ctx, repo_url.as_ref(), &project_dir)
            .map_err(|e| super::CommandError::ExecutionFailed(e.to_string()))
    }
}

fn get_submodules(project_dir: &Path) -> Result<Vec<Submodule>, anyhow::Error> {
    let gitmodules_path = project_dir.join(".gitmodules");

    if !gitmodules_path.exists() {
        return Ok(Vec::new());
    }

    let runner = AppContext::global().git_runner();
    let output = runner
        .execute_quiet_in_dir(
            &["config", "--file", ".gitmodules", "--get-regexp", "path"],
            project_dir,
        )
        .with_context(|| "读取 .gitmodules 配置失败")?;

    let content =
        String::from_utf8(output.stdout).with_context(|| "解析 .gitmodules 输出失败")?;

    let mut submodules = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() != 2 {
            continue;
        }

        let submodule_path = parts[1].trim();
        if submodule_path.is_empty() {
            continue;
        }

        let url_output = runner
            .execute_quiet_in_dir(
                &[
                    "config",
                    "--file",
                    ".gitmodules",
                    "--get",
                    &format!("submodule.{}.url", submodule_path),
                ],
                project_dir,
            )
            .with_context(|| format!("获取子模块 {} 的 URL 失败", submodule_path))?;

        let url = String::from_utf8(url_output.stdout).with_context(|| "解析子模块 URL 失败")?;
        let url = url.trim();

        if !url.is_empty() {
            submodules.push(Submodule {
                path: submodule_path.to_string(),
                url: url.to_string(),
            });
        }
    }

    Ok(submodules)
}

fn get_remote_info(project_dir: &Path) -> Result<Vec<(String, String)>, anyhow::Error> {
    let runner = AppContext::global().git_runner();
    let remote_names_output = runner.execute_quiet_in_dir(&["remote"], project_dir);

    let remote_names: Vec<String> = match remote_names_output {
        Ok(output) => {
            let stdout =
                String::from_utf8(output.stdout).with_context(|| "解析远程仓库名称输出失败")?;
            stdout
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
        Err(_) => {
            return Ok(Vec::new());
        }
    };

    let mut remotes = Vec::new();
    for name in remote_names {
        let url_output = runner.execute_quiet_in_dir(&["remote", "get-url", &name], project_dir);
        if let Ok(output) = url_output {
            let url = String::from_utf8(output.stdout)
                .with_context(|| format!("解析远程仓库 {} 的 URL 失败", name))?;
            let url = url.trim().to_string();
            if !url.is_empty() {
                remotes.push((name, url));
            }
        }
    }

    Ok(remotes)
}

fn do_init_project(
    ctx: &DryRunContext,
    repo_url: &str,
    project_dir: &Path,
) -> Result<(), anyhow::Error> {
    let project_name = project_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    let runner = AppContext::global().git_runner();
    runner
        .execute_with_success(&["clone", repo_url, project_dir.to_str().unwrap_or("")])
        .with_context(|| format!("克隆仓库 {} 到 {} 失败", repo_url, project_dir.display()))?;

    let submodules = get_submodules(project_dir)?;

    do_reinit_repo(ctx, project_dir, &project_name, &submodules, repo_url)
}

fn do_perform_actions(
    ctx: &DryRunContext,
    project_dir: &Path,
    project_name: &str,
) -> Result<(), anyhow::Error> {
    let pma_config = project_dir.join(".pma.json");
    if !pma_config.exists() {
        return Ok(());
    }

    let pma_content = std::fs::read_to_string(&pma_config)
        .with_context(|| format!("读取 .pma.json 文件失败: {}", pma_config.display()))?;

    let config: PmaConfig =
        serde_json::from_str(&pma_content).with_context(|| "解析 .pma.json 文件内容失败")?;

    for action in config.actions {
        match action {
            Action::Replace {
                str_old,
                str_new,
                files,
            } => {
                do_replace_action(ctx, project_dir, &str_old, &str_new, &files, project_name)?;
            }
            Action::AddGitRemote {
                remote_name,
                remote_url,
            } => {
                do_add_git_remote_action(
                    ctx,
                    project_dir,
                    &remote_name,
                    &remote_url,
                    project_name,
                )?;
            }
        }
    }

    if !ctx.is_dry_run() {
        std::fs::remove_file(pma_config)?;
    }

    Ok(())
}

fn do_replace_action(
    ctx: &DryRunContext,
    project_dir: &Path,
    str_old: &str,
    str_new: &str,
    files: &[String],
    project_name: &str,
) -> Result<(), anyhow::Error> {
    let str_new = resolve_placeholders(str_new, project_name);

    for file_path in files {
        let full_path = project_dir.join(file_path);
        if !full_path.exists() {
            continue;
        }

        if ctx.is_dry_run() {
            ctx.print_message(&format!("Replace content in file {}", file_path));
            continue;
        }

        let content = std::fs::read_to_string(&full_path)
            .with_context(|| format!("读取文件失败: {}", full_path.display()))?;

        let new_content = content.replace(str_old, &str_new);

        std::fs::write(&full_path, new_content)
            .with_context(|| format!("写入文件失败: {}", full_path.display()))?;
    }

    Ok(())
}

fn do_add_git_remote_action(
    ctx: &DryRunContext,
    project_dir: &Path,
    remote_name: &str,
    remote_url: &str,
    project_name: &str,
) -> Result<(), anyhow::Error> {
    let remote_url = resolve_placeholders(remote_url, project_name);

    ctx.run_in_dir(
        "git",
        &["remote", "add", remote_name, &remote_url],
        Some(project_dir),
    )
    .with_context(|| {
        format!(
            "添加 Git 远程仓库 {} 到 {} 失败",
            remote_name,
            project_dir.display()
        )
    })?;

    Ok(())
}

fn resolve_placeholders(template: &str, project_name: &str) -> String {
    template
        .replace("${PMA_PROJECT_NAME}", project_name)
        .replace("${PMA_PROJECT_NAME_KEBAB}", &project_name.to_kebab_case())
        .replace("${PMA_PROJECT_NAME_PASCAL}", &project_name.to_pascal_case())
}

fn generate_new_remote_url(original_url: &str, project_name: &str) -> Option<String> {
    if let Some((protocol, host, path)) = parse_git_remote_url(original_url) {
        if let Some(last_slash_idx) = path.rfind('/') {
            let prefix = &path[..last_slash_idx];
            let new_path = format!("{}/{}.git", prefix, project_name);
            match protocol {
                GitProtocol::Ssh => {
                    if original_url.starts_with("ssh://") {
                        Some(format!("ssh://{}/{}", host, new_path))
                    } else {
                        Some(format!(
                            "git@{host}:{new_path}",
                            host = host,
                            new_path = new_path
                        ))
                    }
                }
                GitProtocol::Https => Some(format!("https://{}/{}", host, new_path)),
                GitProtocol::Http => Some(format!("http://{}/{}", host, new_path)),
                GitProtocol::Git => Some(format!("git://{}/{}", host, new_path)),
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn parse_git_remote_url(url: &str) -> Option<(GitProtocol, String, String)> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    let protocol = if url.starts_with("git@") || url.starts_with("ssh://") {
        GitProtocol::Ssh
    } else if url.starts_with("https://") {
        GitProtocol::Https
    } else if url.starts_with("http://") {
        GitProtocol::Http
    } else if url.starts_with("git://") {
        GitProtocol::Git
    } else {
        return None;
    };

    let (url, separator) = if url.starts_with("git@") {
        (url.replace("git@", ""), ':')
    } else if url.starts_with("ssh://") {
        let stripped = url.replace("ssh://", "");
        let stripped = if stripped.starts_with("git@") {
            stripped.replacen("git@", "", 1)
        } else {
            stripped
        };
        (stripped, '/')
    } else if url.starts_with("https://") {
        (url.replace("https://", ""), '/')
    } else if url.starts_with("http://") {
        (url.replace("http://", ""), '/')
    } else if url.starts_with("git://") {
        (url.replace("git://", ""), '/')
    } else {
        (url.to_string(), ':')
    };

    let parts: Vec<&str> = url.splitn(2, separator).collect();
    if parts.len() != 2 {
        return None;
    }

    let (host, path) = (parts[0].to_string(), parts[1].to_string());
    Some((protocol, host, path))
}

fn do_reinit_repo(
    ctx: &DryRunContext,
    project_dir: &Path,
    project_name: &str,
    submodules: &[Submodule],
    original_repo_path: &str,
) -> Result<(), anyhow::Error> {
    if !ctx.is_dry_run() {
        std::fs::remove_dir_all(project_dir.join(".git"))?;

        if project_dir.join(".gitmodules").exists() {
            std::fs::remove_file(project_dir.join(".gitmodules"))?;
        }

        for submodule in submodules {
            std::fs::remove_dir_all(project_dir.join(&submodule.path))?;
        }
    }

    ctx.run_in_dir("git", &["init"], Some(project_dir))
        .with_context(|| format!("初始化 Git 仓库失败: {}", project_dir.display()))?;

    for submodule in submodules {
        ctx.run_in_dir(
            "git",
            &["submodule", "add", &submodule.url, &submodule.path],
            Some(project_dir),
        )
        .with_context(|| {
            format!(
                "添加子模块 {} 到 {} 失败",
                submodule.path,
                project_dir.display()
            )
        })?;
    }

    do_perform_actions(ctx, project_dir, project_name)?;

    let original_repo_path = Path::new(original_repo_path);
    let remotes = get_remote_info(original_repo_path)?;
    for (remote_name, remote_url) in remotes {
        if let Some(new_url) = generate_new_remote_url(&remote_url, project_name) {
            ctx.run_in_dir(
                "git",
                &["remote", "add", &remote_name, &new_url],
                Some(project_dir),
            )
            .with_context(|| {
                format!(
                    "添加 Git 远程仓库 {} 到 {} 失败",
                    remote_name,
                    project_dir.display()
                )
            })?;
        }
    }

    ctx.run_in_dir("git", &["add", "."], Some(project_dir))
        .with_context(|| format!("添加所有文件到 Git 仓库失败: {}", project_dir.display()))?;

    ctx.run_in_dir("git", &["commit", "-m", "v0.0.0"], Some(project_dir))
        .with_context(|| format!("提交初始化到 Git 仓库失败: {}", project_dir.display()))?;

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fork_args_structure() {
        let args = ForkArgs {
            path: "/test/path".to_string(),
            name: "test-project".to_string(),
            dry_run: true,
        };

        assert_eq!(args.path, "/test/path");
        assert_eq!(args.name, "test-project");
        assert!(args.dry_run);
    }

    #[test]
    fn test_dry_run_context() {
        let ctx = DryRunContext::new(true);
        assert!(ctx.is_dry_run());

        let ctx = DryRunContext::new(false);
        assert!(!ctx.is_dry_run());
    }

    #[test]
    fn test_resolve_placeholders() {
        let project_name = "MyProject";
        let template = "Project: ${PMA_PROJECT_NAME}, Kebab: ${PMA_PROJECT_NAME_KEBAB}, Pascal: ${PMA_PROJECT_NAME_PASCAL}";
        let resolved = resolve_placeholders(template, project_name);

        assert!(resolved.contains("Project: MyProject"));
        assert!(resolved.contains("Kebab: my-project"));
        assert!(resolved.contains("Pascal: MyProject"));
    }

    #[test]
    fn test_parse_git_remote_url_valid() {
        let result = parse_git_remote_url("git@github.com:user/repo.git");
        assert!(result.is_some());
        let (protocol, host, path) = result.unwrap();
        assert_eq!(protocol, GitProtocol::Ssh);
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");

        let result = parse_git_remote_url("https://github.com/user/repo.git");
        assert!(result.is_some());
        let (protocol, host, path) = result.unwrap();
        assert_eq!(protocol, GitProtocol::Https);
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");

        let result = parse_git_remote_url("http://github.com/user/repo.git");
        assert!(result.is_some());
        let (protocol, host, path) = result.unwrap();
        assert_eq!(protocol, GitProtocol::Http);
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");
    }

    #[test]
    fn test_parse_git_remote_url_invalid() {
        assert!(parse_git_remote_url("").is_none());
        assert!(parse_git_remote_url("invalid-url").is_none());
        assert!(parse_git_remote_url("github.com/user/repo.git").is_none());
    }

    #[test]
    fn test_generate_new_remote_url() {
        let original = "git@github.com:user/original.git";
        let new_url = generate_new_remote_url(original, "new-project");
        assert_eq!(
            new_url,
            Some("git@github.com:user/new-project.git".to_string())
        );

        let original = "https://github.com/user/original.git";
        let new_url = generate_new_remote_url(original, "new-project");
        assert_eq!(
            new_url,
            Some("https://github.com/user/new-project.git".to_string())
        );

        let original = "git@github.com:original.git";
        let new_url = generate_new_remote_url(original, "new-project");
        assert!(new_url.is_none());
    }
}
