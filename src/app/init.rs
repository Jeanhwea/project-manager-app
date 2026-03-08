use super::git;
use super::runner::CommandRunner;
use anyhow::{Context, Result};
use std::path::Path;

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

    // git clone <repo_url> <project_name>
    git::clone(repo_url, &project_name)
        .with_context(|| format!("无法克隆仓库 {} 到 {}", repo_url, project_dir.display()))?;

    // 读取 submodule 配置

    do_reinit_repo(project_dir)
}

fn do_reinit_repo(project_dir: &Path) -> Result<()> {
    // remove the .git directory
    std::fs::remove_dir_all(project_dir.join(".git"))?;

    // git init .
    CommandRunner::run_with_success_in_dir("git", &["init"], project_dir)
        .with_context(|| format!("无法初始化 Git 仓库到 {}", project_dir.display()))?;

    // git add .
    CommandRunner::run_with_success_in_dir("git", &["add", "."], project_dir)
        .with_context(|| format!("无法添加所有文件到 Git 仓库 {}", project_dir.display()))?;

    // git commit -m "init"
    CommandRunner::run_with_success_in_dir("git", &["commit", "-m", "init"], project_dir)
        .with_context(|| format!("无法提交初始化提交到 Git 仓库 {}", project_dir.display()))?;

    Ok(())
}
