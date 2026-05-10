use crate::control::pipeline::Pipeline;
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

struct ForkContext {
    source: PathBuf,
    target: PathBuf,
}

pub fn run(args: ForkArgs) -> Result<()> {
    Pipeline::run(args, get_context, make_plan)
}

fn get_context(args: &ForkArgs) -> Result<ForkContext> {
    let source = Path::new(&args.source);
    let target = Path::new(&args.target);

    if !source.exists() {
        return Err(AppError::not_found(format!("源路径不存在: {}", args.source)).into());
    }

    if target.exists() {
        return Err(AppError::already_exists(format!("目标路径已存在: {}", args.target)).into());
    }

    Ok(ForkContext {
        source: source.to_path_buf(),
        target: target.to_path_buf(),
    })
}

fn make_plan(args: &ForkArgs, ctx: &ForkContext) -> Result<ExecutionPlan> {
    let mut plan = ExecutionPlan::new().with_dry_run(args.dry_run);

    plan.add(MessageOperation::Header {
        title: "项目分叉".to_string(),
    });
    plan.add(MessageOperation::Item {
        label: "源".to_string(),
        value: args.source.clone(),
    });
    plan.add(MessageOperation::Item {
        label: "目标".to_string(),
        value: args.target.clone(),
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
        description: format!("xcopy {} {} /E /I", args.source, args.target),
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
        description: format!("cp -r {} {}", args.source, args.target),
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
