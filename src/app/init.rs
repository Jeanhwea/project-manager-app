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

    do_init_project(&repo_dir, &project_dir)
}

fn do_init_project(from_dir: &Path, project_dir: &Path) -> Result<()> {
    println!(
        "初始化项目: {} -> {}",
        from_dir.display(),
        project_dir.display()
    );
    Ok(())
}
