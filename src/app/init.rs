use anyhow::Result;
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
    println!("初始化项目: {} -> {}", repo_url, project_dir.display());
    super::git::clone(repo_url, project_dir)?;
    Ok(())
}
