use crate::commands::Command;
use crate::domain::git::{self, GitCommandRunner, GitOperation};
use crate::engine::plan;
use crate::error::{AppError, Result};
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase};
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
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<SnapCreateContext> {
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

        let num_commit = git::snapshot::head_commit_count(&project_path)?;

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
            plan.add_message(DisplayMessage::Skip {
                msg: "无变更，跳过快照".to_string(),
            });
            return Ok(plan);
        }

        let mut snap_phase = Phase::new("创建快照");
        if ctx.needs_init {
            snap_phase.add(GitOperation::Init {
                working_dir: ctx.project_path.clone(),
            });
            snap_phase.add(GitOperation::Add {
                path: ".".to_string(),
                working_dir: ctx.project_path.clone(),
            });
            snap_phase.add(GitOperation::Commit {
                message: "snap-000000".to_string(),
                working_dir: ctx.project_path.clone(),
            });
        } else {
            snap_phase.add(GitOperation::Add {
                path: ".".to_string(),
                working_dir: ctx.project_path.clone(),
            });
            snap_phase.add(GitOperation::Commit {
                message: format!("snap-{:06}", ctx.num_commit),
                working_dir: ctx.project_path.clone(),
            });
        }
        plan.add_phase(snap_phase);

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

impl Command for ListArgs {
    type Context = SnapListContext;
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<SnapListContext> {
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

        let snap_commits = git::snapshot::list_snapshot_oneline(project_path)?;

        Ok(SnapListContext { snap_commits })
    }

    fn plan(&self, ctx: &SnapListContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();

        if ctx.snap_commits.is_empty() {
            plan.add_message(DisplayMessage::Warning {
                msg: "无快照记录".to_string(),
            });
            return Ok(plan);
        }

        plan.add_message(DisplayMessage::Section {
            title: "快照历史:".to_string(),
        });

        for (index, commit) in ctx.snap_commits.iter().enumerate() {
            let parts: Vec<&str> = commit.splitn(2, ' ').collect();
            if parts.len() == 2 {
                plan.add_message(DisplayMessage::Skip {
                    msg: format!("#{} {} {}", index, parts[0], parts[1]),
                });
            } else {
                plan.add_message(DisplayMessage::Skip {
                    msg: format!("#{} {}", index, commit),
                });
            }
        }

        plan.add_message(DisplayMessage::Blank);
        plan.add_message(DisplayMessage::Item {
            label: "汇总".to_string(),
            value: format!("共 {} 个快照", ctx.snap_commits.len()),
        });

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

impl Command for RestoreArgs {
    type Context = SnapRestoreContext;
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<SnapRestoreContext> {
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

        let commit_ref = resolve_snapshot_ref(project_path, &self.snapshot)?;

        Ok(SnapRestoreContext { commit_ref })
    }

    fn plan(&self, ctx: &SnapRestoreContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);

        let mut restore_phase = Phase::new("恢复快照");
        restore_phase.add(GitOperation::Checkout {
            ref_name: ctx.commit_ref.clone(),
            working_dir: PathBuf::from(&self.path),
        });
        plan.add_phase(restore_phase);

        plan.add_message(DisplayMessage::Success {
            msg: format!("已恢复到快照 {}", ctx.commit_ref),
        });
        plan.add_message(DisplayMessage::Warning {
            msg: "若要回到最新状态，请执行: git checkout -".to_string(),
        });
        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

pub fn run(args: SnapArgs) -> Result<()> {
    match args {
        SnapArgs::Create(args) => Command::run(&args),
        SnapArgs::List(args) => Command::run(&args),
        SnapArgs::Restore(args) => Command::run(&args),
    }
}

fn resolve_snapshot_ref(project_path: &Path, snapshot: &str) -> Result<String> {
    if snapshot.starts_with("snap-") {
        let lines = git::snapshot::search_oneline(project_path, snapshot)?;
        if let Some(first_line) = lines.first() {
            let hash = first_line.split_whitespace().next().unwrap_or(snapshot);
            return Ok(hash.to_string());
        }
        return Ok(snapshot.to_string());
    }

    if let Some(index_str) = snapshot.strip_prefix('#')
        && let Ok(index) = index_str.parse::<usize>()
    {
        let snap_commits = git::snapshot::list_snapshot_oneline(project_path)?;

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

    let hash = git::snapshot::rev_parse_verify(project_path, snapshot)?;
    let hash = hash.trim().to_string();
    if hash.is_empty() {
        return Err(AppError::snapshot(format!(
            "无法解析快照引用: {}",
            snapshot
        )));
    }

    Ok(hash)
}
