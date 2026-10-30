//! pma 的顶层命令行定义：一个 `Cli` 根命令加若干子命令（`Commands`）。
//! 每个子命令只持有自己的参数结构，真正的业务逻辑在 `crate::commands` 下同名模块中。

use clap::Parser;

use crate::commands::{
    branch::{BranchArgs, BranchCleanArgs},
    config::ConfigArgs,
    doctor::DoctorArgs,
    fork::ForkArgs,
    gitlab::GitlabArgs,
    release::ReleaseArgs,
    self_update::SelfManageArgs,
    snap::SnapArgs,
    status::StatusArgs,
    sync::SyncArgs,
};
use crate::error::Result;

/// 根命令，除子命令外不接受其他参数
#[derive(Parser)]
#[command(name = "pma")]
#[command(about = "Project Manager Application (项目管理工具)")]
#[command(version)]
#[command(styles = crate::cli::styles())]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// 所有可用子命令，变体上的文档注释即 clap 的 help 文案
#[derive(clap::Subcommand)]
pub enum Commands {
    /// Release a new version
    #[command(visible_alias = "re")]
    Release(ReleaseArgs),

    /// Synchronize all code repositories
    #[command(visible_alias = "s")]
    Sync(SyncArgs),

    /// Diagnostic project health
    Doctor(DoctorArgs),

    /// Fork a new project from a template
    Fork(ForkArgs),

    /// GitLab integration commands
    #[command(visible_alias = "gl")]
    Gitlab {
        #[command(subcommand)]
        command: GitlabArgs,
    },

    /// Snapshot a project
    Snap {
        #[command(subcommand)]
        command: SnapArgs,
    },

    /// Show status of all code repositories
    #[command(visible_alias = "st")]
    Status(StatusArgs),

    /// Clean branches (alias for branch clean)
    #[command(visible_alias = "c")]
    Clean(BranchCleanArgs),

    /// Manage branches across repositories
    #[command(visible_alias = "br")]
    Branch {
        #[command(subcommand)]
        command: BranchArgs,
    },

    /// Self management commands
    #[command(name = "self")]
    SelfManage {
        #[command(subcommand)]
        command: SelfManageArgs,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigArgs,
    },
}

/// 把解析后的子命令路由到对应实现，命令内部的失败统一以 `AppError` 上抛
pub fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Release(args) => crate::commands::release::run(args),
        Commands::Sync(args) => crate::commands::sync::run(args),
        Commands::Doctor(args) => crate::commands::doctor::run(args),
        Commands::Fork(args) => crate::commands::fork::run(args),
        Commands::Gitlab { command } => crate::commands::gitlab::run(command),
        Commands::Snap { command } => crate::commands::snap::run(command),
        // Clean 只是 branch clean 的顶层别名，走通用多仓库路径而非 branch::run
        Commands::Clean(args) => crate::commands::run_multi_repo_cmd(&args, &args.repo_path),
        Commands::Status(args) => crate::commands::status::run(args),
        Commands::Branch { command } => crate::commands::branch::run(command),
        Commands::SelfManage { command } => crate::commands::self_update::run(command),
        Commands::Config { command } => crate::commands::config::run(command),
    }
}
