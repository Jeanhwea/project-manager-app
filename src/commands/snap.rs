//! `pma snap`：把项目当前改动增量提交成快照。快照就是普通 git 提交，
//! 靠 commit message 里的 `snap-NNNNNN` 编号识别，编号取创建时的提交数，故天然递增。
//! create 先判断有无改动再 add + commit；list 列出历史快照；restore 解析出 commit 后 checkout。

use crate::commands::Command;
use crate::domain::git::{self, GitCommandRunner, GitOperation};
use crate::engine::plan;
use crate::error::{AppError, Result};
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase};
use std::path::{Path, PathBuf};

/// 快照引用解析失败的分类
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("快照索引 #{index} 超出范围 (共 {total} 个快照)")]
    IndexOutOfRange { index: usize, total: usize },

    #[error("无法解析快照引用: {snapshot}")]
    UnresolvedRef { snapshot: String },
}

/// `pma snap` 的子命令：创建（create）、列出（list/ls）、恢复（restore/rs）
#[derive(Debug, clap::Subcommand)]
pub enum SnapArgs {
    Create(CreateArgs),
    #[command(visible_alias = "ls")]
    List(ListArgs),
    #[command(visible_alias = "rs")]
    Restore(RestoreArgs),
}

/// `pma snap create`：快照一个项目的当前改动，必要时先 git init
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

/// `pma snap list`：列出项目的历史快照
#[derive(Debug, clap::Args)]
pub struct ListArgs {
    #[arg(
        default_value = ".",
        help = "Path to the project, defaults to current directory"
    )]
    pub path: String,
}

/// `pma snap restore`：把工作区切到某个快照；恢复前未提交的改动由 git checkout 自行处理
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

/// create 的采集结果：项目路径、是否需要 init、有无待提交改动、现有提交数
#[derive(Debug)]
pub(crate) struct SnapCreateContext {
    project_path: PathBuf,
    needs_init: bool,
    has_changes: bool,
    commit_count: usize,
}

/// list 的采集结果：git log --oneline 里含 `snap-` 的行，顺序为 git log 默认的倒序
#[derive(Debug)]
pub(crate) struct SnapListContext {
    snap_commits: Vec<String>,
}

/// restore 的采集结果：快照引用解析出来的 commit hash
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
            return Err(AppError::NotFound {
                resource: "项目路径".into(),
                name: self.path.clone(),
            });
        }

        let (needs_init, has_changes, commit_count) = inspect_project(&project_path)?;

        Ok(SnapCreateContext {
            project_path,
            needs_init,
            has_changes,
            commit_count,
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
        // 还没有 .git 的目录先 init，否则下面的 add/commit 无处可落
        if ctx.needs_init {
            snap_phase.add(GitOperation::Init {
                working_dir: ctx.project_path.clone(),
            });
        }
        snap_phase.add(GitOperation::Add {
            path: ".".to_string(),
            working_dir: ctx.project_path.clone(),
        });
        snap_phase.add(GitOperation::Commit {
            message: snap_commit_message(ctx),
            working_dir: ctx.project_path.clone(),
        });
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
            return Err(AppError::NotFound {
                resource: "项目路径".into(),
                name: self.path.clone(),
            });
        }

        // 目录不是 git 仓库时按「还没快照过」处理，list 不对非 git 目录报错
        if !project_path.join(".git").exists() {
            return Ok(SnapListContext {
                snap_commits: Vec::new(),
            });
        }

        let snap_commits =
            git::snapshot::list_snapshot_oneline(&GitCommandRunner::new(), project_path)?;

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

        // git log 是时间倒序，故 #0 为最新快照；该编号与 restore 的 `#N` 共用同一套顺序
        for (index, commit) in ctx.snap_commits.iter().enumerate() {
            // oneline 形如 "<短 hash> <message>"，拆不成两段时退化为整行原样输出
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
            return Err(AppError::NotFound {
                resource: "项目路径".into(),
                name: self.path.clone(),
            });
        }

        // 与 list 不同：restore 必须落在已有快照的仓库里，不能容忍非 git 目录
        if !project_path.join(".git").exists() {
            return Err(AppError::NotFound {
                resource: "快照仓库".into(),
                name: self.path.clone(),
            });
        }

        let commit_ref = resolve_snapshot_ref(project_path, &self.snapshot)?;

        Ok(SnapRestoreContext { commit_ref })
    }

    fn plan(&self, ctx: &SnapRestoreContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);

        let mut restore_phase = Phase::new("恢复快照");
        // checkout 到一个具体的 commit 会让 HEAD 处于 detached 状态，故下面提示回到原分支
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

/// 分发 `pma snap` 子命令
pub fn run(args: SnapArgs) -> Result<()> {
    match args {
        SnapArgs::Create(args) => Command::run(&args),
        SnapArgs::List(args) => Command::run(&args),
        SnapArgs::Restore(args) => Command::run(&args),
    }
}

/// 采集项目状态，返回 (是否需要 git init, 是否有未提交改动, HEAD 可达提交数)
fn inspect_project(project_path: &Path) -> Result<(bool, bool, usize)> {
    // 非 git 目录视为待初始化：一定有「改动」要提交，且还没有 HEAD，提交数记为 0
    if !project_path.join(".git").exists() {
        return Ok((true, true, 0));
    }

    let runner = GitCommandRunner::new();
    // git status 失败时不在这里中断：假定有改动，让后续 add/commit 去暴露真实错误
    let has_changes = runner.has_uncommitted_changes(project_path).unwrap_or(true);
    if !has_changes {
        // 工作区干净，plan 阶段会跳过，无需多查一次提交数
        return Ok((false, false, 0));
    }

    let commit_count = git::snapshot::head_commit_count(&runner, project_path)?;
    Ok((false, true, commit_count))
}

/// 生成快照 message：编号取当前提交数，即新提交在历史中的 0-based 序号
fn snap_commit_message(ctx: &SnapCreateContext) -> String {
    if ctx.needs_init {
        "snap-000000".to_string()
    } else {
        // 计数包含非快照的普通提交，混合提交的场景下相邻快照编号可能跳号
        format!("snap-{:06}", ctx.commit_count)
    }
}

/// 把用户给的快照引用解析为 commit hash，支持 `snap-xxx` 关键字、`#序号` 和任意 git ref
fn resolve_snapshot_ref(project_path: &Path, snapshot: &str) -> Result<String> {
    let runner = GitCommandRunner::new();

    if snapshot.starts_with("snap-") {
        let lines = git::snapshot::search_oneline(&runner, project_path, snapshot)?;
        // --grep 命中的第一条就是最新的一条
        if let Some(first_line) = lines.first() {
            let hash = first_line.split_whitespace().next().unwrap_or(snapshot);
            return Ok(hash.to_string());
        }
        // 查不到时原样返回，把错误交给后面的 checkout，省一次额外查询
        return Ok(snapshot.to_string());
    }

    // `#N` 直接按 list 的展示序号取；越界才报错
    if let Some(index_str) = snapshot.strip_prefix('#')
        && let Ok(index) = index_str.parse::<usize>()
    {
        let snap_commits = git::snapshot::list_snapshot_oneline(&runner, project_path)?;

        if index < snap_commits.len() {
            let hash = snap_commits[index]
                .split_whitespace()
                .next()
                .unwrap_or(snapshot);
            return Ok(hash.to_string());
        } else {
            return Err(SnapshotError::IndexOutOfRange {
                index,
                total: snap_commits.len(),
            }
            .into());
        }
    }

    // 兜底当成普通 git ref（分支名、tag、hash 前缀等），由 rev-parse 判定是否存在
    let hash = git::snapshot::rev_parse_verify(&runner, project_path, snapshot)?;
    let hash = hash.trim().to_string();
    if hash.is_empty() {
        return Err(SnapshotError::UnresolvedRef {
            snapshot: snapshot.to_string(),
        }
        .into());
    }

    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn debug_msg(m: &DisplayMessage) -> String {
        format!("{:?}", m)
    }

    // 锁定 list 输出的消息类型与顺序，改动渲染格式时必须同步更新这份 baseline
    #[test]
    fn snap_list_messages_byte_equal_baseline() {
        let snap_commits = vec![
            "abc1234 snap-000000".to_string(),
            "def5678 snap-000001".to_string(),
        ];
        let ctx = SnapListContext {
            snap_commits: snap_commits.clone(),
        };
        let args = ListArgs {
            path: ".".to_string(),
        };
        let plan = args.plan(&ctx).expect("plan");

        let actual: Vec<String> = plan.messages().iter().map(debug_msg).collect();

        let expected = vec![
            debug_msg(&DisplayMessage::Section {
                title: "快照历史:".to_string(),
            }),
            debug_msg(&DisplayMessage::Skip {
                msg: "#0 abc1234 snap-000000".to_string(),
            }),
            debug_msg(&DisplayMessage::Skip {
                msg: "#1 def5678 snap-000001".to_string(),
            }),
            debug_msg(&DisplayMessage::Blank),
            debug_msg(&DisplayMessage::Item {
                label: "汇总".to_string(),
                value: "共 2 个快照".to_string(),
            }),
        ];

        assert_eq!(actual, expected);
    }
}
