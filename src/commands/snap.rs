use crate::control::command::Command;
use crate::domain::git::GitCommandRunner;
use crate::error::{AppError, Result};
use crate::model::plan::{ExecutionPlan, GitOperation, MessageOperation};
use std::path::{Path, PathBuf};

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

#[derive(Debug)]
pub(crate) struct SnapCreateContext {
    project_path: PathBuf,
    needs_init: bool,
    has_changes: bool,
    num_commit: usize,
}

#[derive(Debug)]
pub(crate) struct SnapListContext {
    snap_commits: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct SnapRestoreContext {
    commit_ref: String,
}

impl Command for CreateArgs {
    type Context = SnapCreateContext;

    fn context(&self) -> Result<SnapCreateContext> {
        let project_path = Path::new(&self.path).to_path_buf();

        if !project_path.exists() {
            return Err(AppError::not_found(format!(
                "项目路径不存在: {}",
                self.path
            )));
        }

        if !project_path.join(".git").exists() {
            return Ok(SnapCreateContext {
                project_path,
                needs_init: true,
                has_changes: true,
                num_commit: 0,
            });
        }

        let runner = GitCommandRunner::new();
        let has_changes = runner
            .has_uncommitted_changes(&project_path)
            .unwrap_or(true);
        if !has_changes {
            return Ok(SnapCreateContext {
                project_path,
                needs_init: false,
                has_changes: false,
                num_commit: 0,
            });
        }

        let output = runner.execute_raw(&["rev-list", "--count", "HEAD"], &project_path)?;
        let num_commit = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<usize>()?;

        Ok(SnapCreateContext {
            project_path,
            needs_init: false,
            has_changes: true,
            num_commit,
        })
    }

    fn plan(&self, ctx: &SnapCreateContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);

        if !ctx.has_changes {
            plan.add(MessageOperation::Skip {
                msg: "无变更，跳过快照".to_string(),
            });
            return Ok(plan);
        }

        if ctx.needs_init {
            plan.add(GitOperation::Init {
                working_dir: ctx.project_path.clone(),
            });
            plan.add(GitOperation::Add {
                path: ".".to_string(),
                working_dir: ctx.project_path.clone(),
            });
            plan.add(GitOperation::Commit {
                message: "snap-000000".to_string(),
                working_dir: ctx.project_path.clone(),
            });
        } else {
            plan.add(GitOperation::Add {
                path: ".".to_string(),
                working_dir: ctx.project_path.clone(),
            });
            plan.add(GitOperation::Commit {
                message: format!("snap-{:06}", ctx.num_commit),
                working_dir: ctx.project_path.clone(),
            });
        }

        Ok(plan)
    }
}

impl Command for ListArgs {
    type Context = SnapListContext;

    fn context(&self) -> Result<SnapListContext> {
        let project_path = Path::new(&self.path);

        if !project_path.exists() {
            return Err(AppError::not_found(format!(
                "项目路径不存在: {}",
                self.path
            )));
        }

        if !project_path.join(".git").exists() {
            return Ok(SnapListContext {
                snap_commits: Vec::new(),
            });
        }

        let runner = GitCommandRunner::new();
        let output = runner.execute_raw(&["log", "--oneline"], project_path)?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let snap_commits: Vec<String> = stdout
            .lines()
            .filter(|line| line.contains("snap-"))
            .map(|s| s.to_string())
            .collect();

        Ok(SnapListContext { snap_commits })
    }

    fn plan(&self, ctx: &SnapListContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();

        if ctx.snap_commits.is_empty() {
            plan.add(MessageOperation::Warning {
                msg: "无快照记录".to_string(),
            });
            return Ok(plan);
        }

        plan.add(MessageOperation::Section {
            title: "快照历史:".to_string(),
        });

        for (index, commit) in ctx.snap_commits.iter().enumerate() {
            let parts: Vec<&str> = commit.splitn(2, ' ').collect();
            if parts.len() == 2 {
                plan.add(MessageOperation::Skip {
                    msg: format!("#{} {} {}", index, parts[0], parts[1]),
                });
            } else {
                plan.add(MessageOperation::Skip {
                    msg: format!("#{} {}", index, commit),
                });
            }
        }

        plan.add(MessageOperation::Blank);
        plan.add(MessageOperation::Item {
            label: "汇总".to_string(),
            value: format!("共 {} 个快照", ctx.snap_commits.len()),
        });

        Ok(plan)
    }
}

impl Command for RestoreArgs {
    type Context = SnapRestoreContext;

    fn context(&self) -> Result<SnapRestoreContext> {
        let project_path = Path::new(&self.path);

        if !project_path.exists() {
            return Err(AppError::not_found(format!(
                "项目路径不存在: {}",
                self.path
            )));
        }

        if !project_path.join(".git").exists() {
            return Err(AppError::not_found(
                "项目尚未初始化快照，无法恢复".to_string(),
            ));
        }

        let runner = GitCommandRunner::new();
        let commit_ref = resolve_snapshot_ref(&runner, project_path, &self.snapshot)?;

        Ok(SnapRestoreContext { commit_ref })
    }

    fn plan(&self, ctx: &SnapRestoreContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);
        plan.add(GitOperation::Checkout {
            ref_name: ctx.commit_ref.clone(),
            working_dir: PathBuf::from(&self.path),
        });
        plan.add(MessageOperation::Success {
            msg: format!("已恢复到快照 {}", ctx.commit_ref),
        });
        plan.add(MessageOperation::Warning {
            msg: "若要回到最新状态，请执行: git checkout -".to_string(),
        });
        Ok(plan)
    }
}

pub fn run(args: SnapArgs) -> Result<()> {
    match args {
        SnapArgs::Create(args) => Command::run(&args),
        SnapArgs::List(args) => Command::run(&args),
        SnapArgs::Restore(args) => Command::run(&args),
    }
}

fn resolve_snapshot_ref(
    runner: &GitCommandRunner,
    project_path: &Path,
    snapshot: &str,
) -> Result<String> {
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
            )));
        }
    }

    let output = runner.execute_raw(&["rev-parse", "--verify", snapshot], project_path)?;

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if hash.is_empty() {
        return Err(AppError::snapshot(format!(
            "无法解析快照引用: {}",
            snapshot
        )));
    }

    Ok(hash)
}
