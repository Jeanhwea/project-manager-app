//! CLI command definitions
//!
//! Defines CLI structure and command routing types.

use clap::Parser;

// Import Args types from commands module for use in Commands enum
use crate::commands::{
    branch::BranchArgs, config::ConfigArgs, doctor::DoctorArgs, fork::ForkArgs, gitlab::GitLabArgs,
    release::ReleaseArgs, selfman::SelfManArgs, snap::SnapArgs, status::StatusArgs, sync::SyncArgs,
};

/// Main CLI structure
#[derive(Parser)]
#[command(name = "pma")]
#[command(about = "Project Manager Application (项目管理工具)")]
#[command(version)]
#[command(styles = crate::cli::get_styles())]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// All available commands
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
        command: SelfManArgs,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigArgs,
    },
}

/// Command name enumeration for dispatching
#[derive(Debug, Clone, PartialEq)]
pub enum CommandName {
    Release,
    Sync,
    Doctor,
    Fork,
    GitLab,
    Snap,
    Status,
    Branch,
    SelfMan,
    Config,
}

/// Parsed command with name and arguments
#[derive(Debug)]
pub struct ParsedCommand {
    pub name: CommandName,
    pub args: CommandArgs,
}

/// Command arguments enumeration
#[derive(Debug)]
pub enum CommandArgs {
    Release(ReleaseArgs),
    Sync(SyncArgs),
    Doctor(DoctorArgs),
    Fork(ForkArgs),
    GitLab(GitLabArgs),
    Snap(SnapArgs),
    Status(StatusArgs),
    Branch(BranchArgs),
    SelfMan(SelfManArgs),
    Config(ConfigArgs),
}
