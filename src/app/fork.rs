use super::git;
use super::runner::CommandRunner;

use anyhow::{Context, Result};
use heck::{ToKebabCase, ToPascalCase};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    project_name: String,
    actions: Vec<Action>,
}

#[derive(Debug, Clone)]
struct Submodule {
    path: String,
    url: String,
}

pub fn execute(path: &str, name: &str) -> Result<()> {
    let root_dir = Path::new(path);

    if !root_dir.exists() {
        anyhow::bail!("目录不存在: {}", path);
    }

    let repo_dir = root_dir.join(".git");
    if !repo_dir.exists() {
        anyhow::bail!("项目仓库目录不存在: {}", repo_dir.to_string_lossy());
    }

    let curr_dir = std::env::current_dir()?;
    let project_dir = curr_dir.join(name);

    if project_dir.exists() {
        anyhow::bail!("项目目录已存在: {}", project_dir.display());
    }

    let repo_url = repo_dir.to_string_lossy();
    do_init_project(repo_url.as_ref(), &project_dir)
}

fn do_init_project(repo_url: &str, project_dir: &Path) -> Result<()> {
    let project_name = project_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    git::clone(repo_url, &project_name)
        .with_context(|| format!("无法克隆仓库 {} 到 {}", repo_url, project_dir.display()))?;

    let submodules = get_submodules(project_dir)?;

    do_reinit_repo(project_dir, &project_name, &submodules)
}

fn get_submodules(project_dir: &Path) -> Result<Vec<Submodule>> {
    let gitmodules_path = project_dir.join(".gitmodules");

    if !gitmodules_path.exists() {
        return Ok(Vec::new());
    }

    let output = CommandRunner::run_quiet_in_dir(
        "git",
        &["config", "--file", ".gitmodules", "--get-regexp", "path"],
        project_dir,
    )
    .with_context(|| "无法读取 .gitmodules 配置")?;

    let content =
        String::from_utf8(output.stdout).with_context(|| "无法解析 .gitmodules 输出")?;

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

        let url_output = CommandRunner::run_quiet_in_dir(
            "git",
            &[
                "config",
                "--file",
                ".gitmodules",
                "--get",
                &format!("submodule.{}.url", submodule_path),
            ],
            project_dir,
        )
        .with_context(|| format!("无法获取子模块 {} 的 URL", submodule_path))?;

        let url = String::from_utf8(url_output.stdout).with_context(|| "无法解析子模块 URL")?;
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

fn do_perform_actions(project_dir: &Path, project_name: &str) -> Result<()> {
    let pma_config = project_dir.join(".pma.json");
    if !pma_config.exists() {
        return Ok(());
    }

    let pma_content = std::fs::read_to_string(&pma_config)
        .with_context(|| format!("无法读取 .pma.json 文件: {}", pma_config.display()))?;

    let config: PmaConfig =
        serde_json::from_str(&pma_content).with_context(|| "无法解析 .pma.json 文件内容")?;

    for action in config.actions {
        match action {
            Action::Replace {
                str_old,
                str_new,
                files,
            } => {
                do_replace_action(project_dir, &str_old, &str_new, &files, project_name)?;
            }
            Action::AddGitRemote {
                remote_name,
                remote_url,
            } => {
                do_add_git_remote_action(project_dir, &remote_name, &remote_url, project_name)?;
            }
        }
    }

    // delete .pma.json file
    std::fs::remove_file(pma_config)?;

    Ok(())
}

fn do_replace_action(
    project_dir: &Path,
    str_old: &str,
    str_new: &str,
    files: &[String],
    project_name: &str,
) -> Result<()> {
    let str_new = resolve_placeholders(str_new, project_name);

    for file_path in files {
        let full_path = project_dir.join(file_path);
        if !full_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&full_path)
            .with_context(|| format!("无法读取文件: {}", full_path.display()))?;

        let new_content = content.replace(str_old, &str_new);

        std::fs::write(&full_path, new_content)
            .with_context(|| format!("无法写入文件: {}", full_path.display()))?;
    }

    Ok(())
}

fn do_add_git_remote_action(
    project_dir: &Path,
    remote_name: &str,
    remote_url: &str,
    project_name: &str,
) -> Result<()> {
    let remote_url = resolve_placeholders(remote_url, project_name);

    CommandRunner::run_with_success_in_dir(
        "git",
        &["remote", "add", remote_name, &remote_url],
        project_dir,
    )
    .with_context(|| {
        format!(
            "无法添加 Git 远程仓库 {} 到 {}",
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

fn do_reinit_repo(
    project_dir: &Path,
    project_name: &str,
    submodules: &[Submodule],
) -> Result<()> {
    // delete .git directory
    std::fs::remove_dir_all(project_dir.join(".git"))?;

    // delete .gitmodules file
    if project_dir.join(".gitmodules").exists() {
        std::fs::remove_file(project_dir.join(".gitmodules"))?;
    }

    // delete git submodule directory
    for submodule in submodules {
        std::fs::remove_dir_all(project_dir.join(&submodule.path))?;
    }

    CommandRunner::run_with_success_in_dir("git", &["init"], project_dir)
        .with_context(|| format!("无法初始化 Git 仓库到 {}", project_dir.display()))?;

    for submodule in submodules {
        CommandRunner::run_with_success_in_dir(
            "git",
            &["submodule", "add", &submodule.url, &submodule.path],
            project_dir,
        )
        .with_context(|| {
            format!(
                "无法添加子模块 {} 到 {}",
                submodule.path,
                project_dir.display()
            )
        })?;
    }

    do_perform_actions(project_dir, project_name)?;

    CommandRunner::run_with_success_in_dir("git", &["add", "."], project_dir)
        .with_context(|| format!("无法添加所有文件到 Git 仓库 {}", project_dir.display()))?;

    CommandRunner::run_with_success_in_dir("git", &["commit", "-m", "v0.0.0"], project_dir)
        .with_context(|| format!("无法提交初始化提交到 Git 仓库 {}", project_dir.display()))?;

    Ok(())
}
