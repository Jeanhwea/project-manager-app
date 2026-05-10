use crate::control::command::Command;
use crate::error::{AppError, Result};
use crate::model::plan::{ExecutionPlan, GitOperation, MessageOperation, ShellOperation};
use std::path::{Path, PathBuf};

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

pub(crate) struct ForkContext {
    source: PathBuf,
    target: PathBuf,
}

impl Command for ForkArgs {
    type Context = ForkContext;

    fn context(&self) -> Result<ForkContext> {
        let source = Path::new(&self.source);
        let target = Path::new(&self.target);

        if !source.exists() {
            return Err(AppError::not_found(format!(
                "源路径不存在: {}",
                self.source
            )));
        }

        if target.exists() {
            return Err(AppError::already_exists(format!(
                "目标路径已存在: {}",
                self.target
            )));
        }

        Ok(ForkContext {
            source: source.to_path_buf(),
            target: target.to_path_buf(),
        })
    }

    fn plan(&self, ctx: &ForkContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);

        plan.add(MessageOperation::Header {
            title: "项目分叉".to_string(),
        });
        plan.add(MessageOperation::Item {
            label: "源".to_string(),
            value: self.source.clone(),
        });
        plan.add(MessageOperation::Item {
            label: "目标".to_string(),
            value: self.target.clone(),
        });

        #[cfg(target_os = "windows")]
        plan.add(ShellOperation::Run {
            program: "xcopy".to_string(),
            args: vec![
                ctx.source.to_string_lossy().to_string(),
                ctx.target.to_string_lossy().to_string(),
                "/E".to_string(),
                "/I".to_string(),
            ],
            dir: None,
            description: format!("xcopy {} {} /E /I", self.source, self.target),
        });
        #[cfg(not(target_os = "windows"))]
        plan.add(ShellOperation::Run {
            program: "cp".to_string(),
            args: vec![
                "-r".to_string(),
                ctx.source.to_string_lossy().to_string(),
                ctx.target.to_string_lossy().to_string(),
            ],
            dir: None,
            description: format!("cp -r {} {}", self.source, self.target),
        });

        plan.add(GitOperation::Init {
            dir: ctx.target.clone(),
        });
        plan.add(GitOperation::Add {
            path: ".".to_string(),
        });
        plan.add(GitOperation::Commit {
            message: "fork: initial commit".to_string(),
        });

        Ok(plan)
    }
}

pub fn run(args: ForkArgs) -> Result<()> {
    Command::run(&args)
}
