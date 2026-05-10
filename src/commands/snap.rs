use crate::control::plan::run_plan;
use crate::domain::AppError;
use crate::domain::git::GitCommandRunner;
use crate::model::plan::{ExecutionPlan, GitOperation};
use crate::utils::output::Output;
use std::path::Path;

#[derive(Debug, clap::Subcommand)]
pub enum SnapArgs {
    Create(CreateArgs),
    #[command(visible_alias = "ls")]
    List(ListArgs),
    #[command(visible_alias = "rs")]
    Restore(RestoreArgs),
}

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    #[arg(
        default_value = ".",
        help = "Path to the project to snapshot, defaults to current directory"
    )]
    pub path: String,
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be changed without making any modifications"
    )]
    pub dry_run: bool,
}

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    #[arg(
        default_value = ".",
        help = "Path to the project, defaults to current directory"
    )]
    pub path: String,
}

#[derive(Debug, clap::Args)]
pub struct RestoreArgs {
    #[arg(help = "Snapshot reference (e.g. snap-000001, #0, or commit hash)")]
    pub snapshot: String,
    #[arg(
        default_value = ".",
        help = "Path to the project, defaults to current directory"
    )]
    pub path: String,
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be changed without making any modifications"
    )]
    pub dry_run: bool,
}

pub fn run(args: SnapArgs) -> anyhow::Result<()> {
    match args {
        SnapArgs::Create(args) => execute_create(args),
        SnapArgs::List(args) => execute_list(args),
        SnapArgs::Restore(args) => execute_restore(args),
    }
}

fn execute_create(args: CreateArgs) -> anyhow::Result<()> {
    let project_path = Path::new(&args.path);
    let runner = GitCommandRunner::new();

    if !project_path.exists() {
        return Err(AppError::not_found(format!("项目路径不存在: {}", args.path)).into());
    }

    let mut plan = ExecutionPlan::new().dry_run(args.dry_run);

    if !project_path.join(".git").exists() {
        plan.add(GitOperation::Init {
            dir: project_path.to_path_buf(),
        });
        plan.add(GitOperation::Add {
            path: ".".to_string(),
        });
        plan.add(GitOperation::Commit {
            message: "snap-000000".to_string(),
        });
    } else {
        let has_changes = runner.has_uncommitted_changes(project_path).unwrap_or(true);
        if !has_changes {
            Output::skip("无变更，跳过快照");
            return Ok(());
        }

        let output = runner.execute_raw(&["rev-list", "--count", "HEAD"], project_path)?;
        let num_commit = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<usize>()?;

        plan.add(GitOperation::Add {
            path: ".".to_string(),
        });
        plan.add(GitOperation::Commit {
            message: format!("snap-{:06}", num_commit),
        });
    }

    run_plan(&plan)
}

fn execute_list(args: ListArgs) -> anyhow::Result<()> {
    let project_path = Path::new(&args.path);
    let runner = GitCommandRunner::new();

    if !project_path.exists() {
        return Err(AppError::not_found(format!("项目路径不存在: {}", args.path)).into());
    }

    if !project_path.join(".git").exists() {
        Output::warning("项目尚未初始化快照");
        return Ok(());
    }

    let output = runner.execute_raw(&["log", "--oneline"], project_path)?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let snap_commits: Vec<&str> = stdout
        .lines()
        .filter(|line| line.contains("snap-"))
        .collect();

    if snap_commits.is_empty() {
        Output::warning("无快照记录");
        return Ok(());
    }

    Output::section("快照历史:");
    for (index, commit) in snap_commits.iter().enumerate() {
        let parts: Vec<&str> = commit.splitn(2, ' ').collect();
        if parts.len() == 2 {
            Output::message(&format!("#{} {} {}", index, parts[0], parts[1]));
        } else {
            Output::message(&format!("#{} {}", index, commit));
        }
    }

    Output::blank();
    Output::item("汇总", &format!("共 {} 个快照", snap_commits.len()));

    Ok(())
}

fn execute_restore(args: RestoreArgs) -> anyhow::Result<()> {
    let project_path = Path::new(&args.path);
    let runner = GitCommandRunner::new();

    if !project_path.exists() {
        return Err(AppError::not_found(format!("项目路径不存在: {}", args.path)).into());
    }

    if !project_path.join(".git").exists() {
        return Err(AppError::not_found("项目尚未初始化快照，无法恢复".to_string()).into());
    }

    let commit_ref = resolve_snapshot_ref(&runner, project_path, &args.snapshot)?;

    let mut plan = ExecutionPlan::new().dry_run(args.dry_run);
    plan.add(GitOperation::Checkout {
        ref_name: commit_ref.clone(),
    });
    run_plan(&plan)?;

    Output::success(&format!("已恢复到快照 {}", commit_ref));
    Output::warning("若要回到最新状态，请执行: git checkout -");

    Ok(())
}

fn resolve_snapshot_ref(
    runner: &GitCommandRunner,
    project_path: &Path,
    snapshot: &str,
) -> anyhow::Result<String> {
    if snapshot.starts_with("snap-") {
        let output =
            runner.execute_raw(&["log", "--oneline", "--grep", snapshot], project_path)?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Some(first_line) = stdout.lines().next() {
            let hash = first_line.split_whitespace().next().unwrap_or(snapshot);
            return Ok(hash.to_string());
        }
    }

    if let Some(index_str) = snapshot.strip_prefix('#')
        && let Ok(index) = index_str.parse::<usize>()
    {
        let output = runner.execute_raw(&["log", "--oneline"], project_path)?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let snap_commits: Vec<&str> = stdout
            .lines()
            .filter(|line| line.contains("snap-"))
            .collect();

        if index < snap_commits.len() {
            let hash = snap_commits[index]
                .split_whitespace()
                .next()
                .unwrap_or(snapshot);
            return Ok(hash.to_string());
        } else {
            return Err(AppError::snapshot(format!(
                "快照索引 #{} 超出范围 (共 {} 个快照)",
                index,
                snap_commits.len()
            ))
            .into());
        }
    }

    let output = runner.execute_raw(&["rev-parse", "--verify", snapshot], project_path)?;

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if hash.is_empty() {
        return Err(AppError::snapshot(format!("无法解析快照引用: {}", snapshot)).into());
    }

    Ok(hash)
}
