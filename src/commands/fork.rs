use crate::domain::AppError;
use crate::domain::git::executor::{ExecutionPlan, GitOperation};
use crate::utils::output::Output;
use std::path::Path;

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

pub fn run(args: ForkArgs) -> anyhow::Result<()> {
    let source = Path::new(&args.source);
    let target = Path::new(&args.target);

    if !source.exists() {
        return Err(AppError::not_found(format!("源路径不存在: {}", args.source)).into());
    }

    if target.exists() {
        return Err(AppError::already_exists(format!("目标路径已存在: {}", args.target)).into());
    }

    Output::header("项目分叉");
    Output::item("源", &args.source);
    Output::item("目标", &args.target);

    let mut plan = ExecutionPlan::new().dry_run(args.dry_run);

    plan.add(GitOperation::Custom {
        args: vec![
            "cp".to_string(),
            "-r".to_string(),
            source.to_string_lossy().to_string(),
            target.to_string_lossy().to_string(),
        ],
        description: format!("cp -r {} {}", args.source, args.target),
    });

    plan.add(GitOperation::Init {
        dir: target.to_path_buf(),
    });
    plan.add(GitOperation::Add {
        path: ".".to_string(),
    });
    plan.add(GitOperation::Commit {
        message: "fork: initial commit".to_string(),
    });

    plan.execute()
}
