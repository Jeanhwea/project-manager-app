use clap::Parser;

use crate::commands::{
    branch::BranchArgs, config::ConfigArgs, doctor::DoctorArgs, fork::ForkArgs,
    gitlab::GitLabArgs, release::ReleaseArgs, selfman::SelfManageArgs, snap::SnapArgs,
    status::StatusArgs, sync::SyncArgs,
};
use crate::error::Result;

#[derive(Parser)]
#[command(name = "pma")]
#[command(about = "Project Manager Application (项目管理工具)")]
#[command(version)]
#[command(styles = crate::cli::get_styles())]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

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
        command: GitLabArgs,
    },

    /// Snapshot a project
    Snap {
        #[command(subcommand)]
        command: SnapArgs,
    },

    /// Show status of all code repositories
    #[command(visible_alias = "st")]
    Status(StatusArgs),

    /// Manage branches across repositories
    #[command(visible_alias = "br")]
    Branch {
        #[command(subcommand)]
        command: BranchArgs,
    },

    /// Self management commands
    #[command(name = "self")]
    Self_ {
        #[command(subcommand)]
        command: SelfManageArgs,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigArgs,
    },
}

pub fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Release(args) => crate::commands::release::run(args),
        Commands::Sync(args) => crate::commands::sync::run(args),
        Commands::Doctor(args) => crate::commands::doctor::run(args),
        Commands::Fork(args) => crate::commands::fork::run(args),
        Commands::Gitlab { command } => crate::commands::gitlab::run(command),
        Commands::Snap { command } => crate::commands::snap::run(command),
        Commands::Status(args) => crate::commands::status::run(args),
        Commands::Branch { command } => crate::commands::branch::run(command),
        Commands::Self_ { command } => crate::commands::selfman::run(command),
        Commands::Config { command } => crate::commands::config::run(command),
    }
}
