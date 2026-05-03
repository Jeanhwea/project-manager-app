use super::{Command, CommandError, CommandResult};
use crate::domain::git::command::GitCommandRunner;
use crate::domain::runner::DryRunContext;
use crate::utils::output::Output;
use anyhow::Result;
use std::path::Path;

/// Snap command arguments
#[derive(Debug)]
pub enum SnapArgs {
    /// Create a snapshot of the current project state
    Create(CreateArgs),
    /// List snapshot history
    List(ListArgs),
    /// Restore project to a specific snapshot
    Restore(RestoreArgs),
}

/// Create snapshot arguments
#[derive(Debug)]
pub struct CreateArgs {
    /// Path to the project to snapshot
    pub path: String,
    /// Dry run: show what would be changed without making any modifications
    pub dry_run: bool,
}

/// List snapshots arguments
#[derive(Debug)]
pub struct ListArgs {
    /// Path to the project
    pub path: String,
}

/// Restore snapshot arguments
#[derive(Debug)]
pub struct RestoreArgs {
    /// Snapshot reference (e.g. snap-000001, #0, or commit hash)
    pub snapshot: String,
    /// Path to the project
    pub path: String,
    /// Dry run: show what would be changed without making any modifications
    pub dry_run: bool,
}

/// Snap command
pub struct SnapCommand;

impl Command for SnapCommand {
    type Args = SnapArgs;

    fn execute(args: Self::Args) -> CommandResult {
        match execute_snap(args) {
            Ok(()) => Ok(()),
            Err(e) => Err(CommandError::ExecutionFailed(format!("{}", e))),
        }
    }
}

/// Main snap execution function
fn execute_snap(args: SnapArgs) -> Result<()> {
    match args {
        SnapArgs::Create(args) => execute_create(args),
        SnapArgs::List(args) => execute_list(args),
        SnapArgs::Restore(args) => execute_restore(args),
    }
}

/// Execute create snapshot command
fn execute_create(args: CreateArgs) -> Result<()> {
    let project_path = Path::new(&args.path);
    let ctx = DryRunContext::new(args.dry_run);
    let runner = GitCommandRunner::new();

    if !project_path.exists() {
        anyhow::bail!("项目路径不存在: {}", args.path);
    }

    if ctx.is_dry_run() {
        ctx.print_header("[DRY-RUN] 将要执行的操作:");
    }

    if !project_path.join(".git").exists() {
        do_initialize_snapshot(&ctx, &runner, project_path)?;
    } else {
        do_incremental_snapshot(&ctx, &runner, project_path)?;
    }

    Ok(())
}

/// Execute list snapshots command
fn execute_list(args: ListArgs) -> Result<()> {
    let project_path = Path::new(&args.path);
    let runner = GitCommandRunner::new();

    if !project_path.exists() {
        anyhow::bail!("项目路径不存在: {}", args.path);
    }

    if !project_path.join(".git").exists() {
        Output::warning("项目尚未初始化快照");
        return Ok(());
    }

    let output = runner
        .execute_quiet_in_dir(&["log", "--oneline"], project_path)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
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
            let hash = parts[0];
            let message = parts[1];
            Output::message(&format!("#{} {} {}", index, hash, message));
        } else {
            Output::message(&format!("#{} {}", index, commit));
        }
    }

    let total = snap_commits.len();
    Output::blank();
    Output::item("汇总", &format!("共 {} 个快照", total));

    Ok(())
}

/// Execute restore snapshot command
fn execute_restore(args: RestoreArgs) -> Result<()> {
    let project_path = Path::new(&args.path);
    let ctx = DryRunContext::new(args.dry_run);
    let runner = GitCommandRunner::new();

    if !project_path.exists() {
        anyhow::bail!("项目路径不存在: {}", args.path);
    }

    if !project_path.join(".git").exists() {
        anyhow::bail!("项目尚未初始化快照，无法恢复");
    }

    let commit_ref = resolve_snapshot_ref(&runner, project_path, &args.snapshot)?;

    if ctx.is_dry_run() {
        ctx.print_header("[DRY-RUN] 将要执行的操作:");
        ctx.print_message(&format!("git checkout {}", commit_ref));
        return Ok(());
    }

    let output = runner
        .execute_raw_in_dir(&["checkout", &commit_ref], project_path)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        print!("{}", stdout);
    }

    Output::success(&format!("已恢复到快照 {}", commit_ref));
    Output::warning("若要回到最新状态，请执行: git checkout -");

    Ok(())
}

/// Resolve snapshot reference to commit hash
fn resolve_snapshot_ref(
    runner: &GitCommandRunner,
    project_path: &Path,
    snapshot: &str,
) -> Result<String> {
    if snapshot.starts_with("snap-") {
        let output = runner
            .execute_quiet_in_dir(&["log", "--oneline", "--grep", snapshot], project_path)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Some(first_line) = stdout.lines().next() {
            let hash = first_line.split_whitespace().next().unwrap_or(snapshot);
            return Ok(hash.to_string());
        }
    }

    if let Some(index_str) = snapshot.strip_prefix('#')
        && let Ok(index) = index_str.parse::<usize>()
    {
        let output = runner
            .execute_quiet_in_dir(&["log", "--oneline"], project_path)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
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
            anyhow::bail!(
                "快照索引 #{} 超出范围 (共 {} 个快照)",
                index,
                snap_commits.len()
            );
        }
    }

    let output = runner
        .execute_quiet_in_dir(&["rev-parse", "--verify", snapshot], project_path)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if hash.is_empty() {
        anyhow::bail!("无法解析快照引用: {}", snapshot);
    }

    Ok(hash)
}

/// Initialize a new snapshot repository
fn do_initialize_snapshot(
    ctx: &DryRunContext,
    _runner: &GitCommandRunner,
    work_dir: &Path,
) -> Result<()> {
    ctx.run_in_dir("git", &["init"], Some(work_dir))?;
    ctx.run_in_dir("git", &["add", "."], Some(work_dir))?;
    ctx.run_in_dir("git", &["commit", "-m", "snap-000000"], Some(work_dir))?;

    Ok(())
}

/// Create incremental snapshot
fn do_incremental_snapshot(
    ctx: &DryRunContext,
    runner: &GitCommandRunner,
    work_dir: &Path,
) -> Result<()> {
    let has_changes = check_pending_changes(runner, work_dir);

    if !has_changes {
        Output::skip("无变更，跳过快照");
        return Ok(());
    }

    let output = runner
        .execute_raw_in_dir(&["rev-list", "--count", "HEAD"], work_dir)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let num_commit = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()?;

    ctx.run_in_dir("git", &["add", "."], Some(work_dir))?;
    ctx.run_in_dir(
        "git",
        &["commit", "-m", &format!("snap-{:06}", num_commit)],
        Some(work_dir),
    )?;

    Ok(())
}

/// Check if there are pending changes in the repository
fn check_pending_changes(runner: &GitCommandRunner, work_dir: &Path) -> bool {
    let output = match runner.execute_quiet_in_dir(&["status", "--porcelain"], work_dir) {
        Ok(o) => o,
        Err(_) => return true,
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return true,
    };

    !stdout.trim().is_empty()
}
