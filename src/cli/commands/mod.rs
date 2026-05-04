//! Command definitions and types
//!
//! This module contains all CLI command definitions organized by functionality.
//! Each command is defined in its own module for better maintainability.

mod branch;
mod config;
mod doctor;
mod fork;
mod gitlab;
mod release;
mod selfman;
mod snap;
mod status;
mod sync;

pub use branch::BranchCommands;
pub use config::ConfigCommands;
pub use doctor::DoctorCmd;
pub use fork::ForkCmd;
pub use gitlab::{CloneProtocol, GitlabCommands};
pub use release::ReleaseCmd;
pub use selfman::SelfCommands;
pub use snap::SnapCommands;
pub use status::{StatusCmd, StatusFilter};
pub use sync::SyncCmd;

use crate::commands::{
    branch::BranchArgs, config::ConfigArgs, doctor::DoctorArgs, fork::ForkArgs,
    gitlab::GitLabArgs, release::ReleaseArgs, selfman::SelfManArgs, snap::SnapArgs,
    status::StatusArgs, sync::SyncArgs,
};
use clap::{Parser, Subcommand};

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

/// All available commands
#[derive(Subcommand)]
pub enum Commands {
    /// Release a new version
    Release(ReleaseCmd),

    /// Synchronize all code repositories
    Sync(SyncCmd),

    /// Diagnostic project health
    Doctor(DoctorCmd),

    /// Fork a new project from a template
    Fork(ForkCmd),

    /// GitLab integration commands
    Gitlab {
        #[command(subcommand)]
        command: GitlabCommands,
    },

    /// Snapshot a project
    Snap {
        #[command(subcommand)]
        command: SnapCommands,
    },

    /// Show status of all code repositories
    Status(StatusCmd),

    /// Manage branches across repositories
    Branch {
        #[command(subcommand)]
        command: BranchCommands,
    },

    /// Self management commands
    #[command(name = "self")]
    Self_ {
        #[command(subcommand)]
        command: SelfCommands,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}
