use crate::domain::git::command::GitCommandRunner;
use crate::domain::git::repository::find_git_repository_upwards;
use crate::utils::output::Output;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, clap::Args)]
pub struct ForkArgs {
    #[arg(help = "Template project path")]
    pub path: String,
    #[arg(help = "New project name")]
    pub name: String,
    #[arg(long, short, help = "Target directory for the new project")]
    pub target: Option<String>,
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be changed without making modifications"
    )]
    pub dry_run: bool,
}

#[derive(Deserialize)]
struct PmaConfig {
    #[serde(default)]
    actions: Vec<ForkAction>,
}

#[derive(Deserialize)]
struct ForkAction {
    action: String,
    #[serde(default)]
    target: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    find: String,
    #[serde(default)]
    replace: String,
}

pub fn run(args: ForkArgs) -> Result<()> {
    let template_path = crate::utils::path::canonicalize_path(&args.path)
        .with_context(|| format!("模板路径无效: {}", args.path))?;

    if !template_path.exists() {
        anyhow::bail!("目录不存在: {}", args.path);
    }

    let repo_dir = find_git_repository_upwards(&template_path).unwrap_or(template_path.clone());

    if !repo_dir.join(".git").exists() {
        anyhow::bail!("Git 仓库目录不存在: {}", repo_dir.display());
    }

    let target_dir = match &args.target {
        Some(t) => PathBuf::from(t),
        None => repo_dir
            .parent()
            .ok_or_else(|| anyhow::anyhow!("无法确定目标目录"))?
            .to_path_buf(),
    };

    let project_dir = target_dir.join(&args.name);

    if project_dir.exists() {
        anyhow::bail!("项目目录已存在: {}", project_dir.display());
    }

    if args.dry_run {
        Output::info("[DRY-RUN] 将要执行的操作:");
        Output::message(&format!(
            "复制 {} -> {}",
            repo_dir.display(),
            project_dir.display()
        ));
        return Ok(());
    }

    Output::info(&format!(
        "Fork 项目: {} -> {}",
        repo_dir.display(),
        project_dir.display()
    ));

    copy_dir_recursive(&repo_dir, &project_dir)?;

    let runner = GitCommandRunner::new();

    clean_git_history(&project_dir, &runner)?;

    let pma_config_path = project_dir.join(".pma.json");
    if pma_config_path.exists() {
        execute_fork_actions(&project_dir, &pma_config_path, &args.name)?;
        fs::remove_file(&pma_config_path)?;
    }

    Output::success(&format!("项目已创建: {}", project_dir.display()));
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if name == ".git" || name == "target" || name == "node_modules" {
            continue;
        }

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

fn clean_git_history(project_dir: &Path, runner: &GitCommandRunner) -> Result<()> {
    let git_dir = project_dir.join(".git");
    if git_dir.exists() {
        fs::remove_dir_all(&git_dir)?;
    }

    runner.execute_with_success(&["init"], Some(project_dir))?;
    runner.execute_with_success(&["add", "."], Some(project_dir))?;
    runner.execute_with_success(
        &["commit", "-m", "Initial commit from fork"],
        Some(project_dir),
    )?;

    Ok(())
}

fn execute_fork_actions(
    project_dir: &Path,
    config_path: &Path,
    project_name: &str,
) -> Result<()> {
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("读取配置文件失败: {}", config_path.display()))?;

    let config: PmaConfig = serde_json::from_str(&content)
        .with_context(|| format!("解析配置文件失败: {}", config_path.display()))?;

    let mut vars = HashMap::new();
    vars.insert("project_name".to_string(), project_name.to_string());

    for action in &config.actions {
        match action.action.as_str() {
            "create_file" => {
                let target = replace_vars(&action.target, &vars);
                let file_path = project_dir.join(&target);
                let content = replace_vars(&action.content, &vars);

                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&file_path, &content)?;
                Output::detail("创建文件", &target);
            }
            "replace_in_file" => {
                let target = replace_vars(&action.target, &vars);
                let file_path = project_dir.join(&target);
                let find = replace_vars(&action.find, &vars);
                let replace = replace_vars(&action.replace, &vars);

                if file_path.exists() {
                    let content = fs::read_to_string(&file_path)?;
                    let new_content = content.replace(&find, &replace);
                    fs::write(&file_path, new_content)?;
                    Output::detail("替换内容", &target);
                }
            }
            "delete_file" => {
                let target = replace_vars(&action.target, &vars);
                let file_path = project_dir.join(&target);

                if file_path.exists() {
                    fs::remove_file(&file_path)?;
                    Output::detail("删除文件", &target);
                }
            }
            _ => {
                Output::warning(&format!("未知操作: {}", action.action));
            }
        }
    }

    Ok(())
}

fn replace_vars(input: &str, vars: &HashMap<String, String>) -> String {
    let mut result = input.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}
