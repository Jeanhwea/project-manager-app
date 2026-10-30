//! `pma fork`：把模板目录复制成一个新项目，并在副本里初始化独立的 git 仓库。

use crate::commands::Command;
use crate::domain::git::GitOperation;
use crate::engine::plan;
use crate::error::{AppError, Result};
use crate::model::operation::EditOperation;
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase};
use std::path::{Path, PathBuf};

/// `pma fork` 的命令行参数
#[derive(Debug, clap::Args)]
pub struct ForkArgs {
    #[arg(help = "Source path")]
    pub source: String,
    #[arg(help = "Target path")]
    pub target: String,
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be changed without making any modifications"
    )]
    pub dry_run: bool,
}

/// 复制的源目录与目标目录，沿用命令行给出的原样路径（不 canonicalize）
#[derive(Debug)]
pub(crate) struct ForkContext {
    source: PathBuf,
    target: PathBuf,
}

impl Command for ForkArgs {
    type Context = ForkContext;
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<ForkContext> {
        let source = Path::new(&self.source);
        let target = Path::new(&self.target);

        // 两个前置检查放在 collect：目标已存在时不覆盖，避免把已有项目冲掉
        if !source.exists() {
            return Err(AppError::NotFound {
                resource: "源路径".into(),
                name: self.source.clone(),
            });
        }

        if target.exists() {
            return Err(AppError::AlreadyExists {
                resource: "目标路径".into(),
                name: self.target.clone(),
            });
        }

        Ok(ForkContext {
            source: source.to_path_buf(),
            target: target.to_path_buf(),
        })
    }

    fn plan(&self, ctx: &ForkContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);

        plan.add_message(DisplayMessage::Header {
            title: "项目分叉".to_string(),
        });
        plan.add_message(DisplayMessage::Item {
            label: "源".to_string(),
            value: self.source.clone(),
        });
        plan.add_message(DisplayMessage::Item {
            label: "目标".to_string(),
            value: self.target.clone(),
        });

        let mut copy_phase = Phase::new("复制项目");
        copy_phase.add(EditOperation::CopyDir {
            source: ctx.source.to_string_lossy().to_string(),
            target: ctx.target.to_string_lossy().to_string(),
            description: format!("copy {} -> {}", self.source, self.target),
        });
        plan.add_phase(copy_phase);

        // 复制完成后重新 git init：新项目要一份自己的历史，而不是带着模板的 .git
        let mut init_phase = Phase::new("初始化仓库");
        init_phase.add(GitOperation::Init {
            working_dir: ctx.target.clone(),
        });
        init_phase.add(GitOperation::Add {
            path: ".".to_string(),
            working_dir: ctx.target.clone(),
        });
        init_phase.add(GitOperation::Commit {
            message: "fork: initial commit".to_string(),
            working_dir: ctx.target.clone(),
        });
        plan.add_phase(init_phase);

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

/// `pma fork` 入口；只作用于一个目标目录，走 `Command` 而非批量骨架
pub fn run(args: ForkArgs) -> Result<()> {
    Command::run(&args)
}
