//! Snap command definitions

use clap::Subcommand;

/// Snapshot a project
#[derive(Subcommand)]
#[command(about = "Snapshot a project")]
pub enum SnapCommands {
    /// Create a snapshot of the current project state
    #[command(about = "Create a snapshot of the current project state")]
    Create {
        /// Path to the project to snapshot, defaults to current directory
        #[arg(
            default_value = ".",
            help = "Path to the project to snapshot, defaults to current directory"
        )]
        path: String,

        /// Dry run: show what would be changed without making any modifications
        #[arg(
            long,
            default_value = "false",
            help = "Dry run: show what would be changed without making any modifications"
        )]
        dry_run: bool,
    },

    /// List snapshot history
    #[command(visible_alias = "ls")]
    #[command(about = "List snapshot history")]
    List {
        /// Path to the project, defaults to current directory
        #[arg(
            default_value = ".",
            help = "Path to the project, defaults to current directory"
        )]
        path: String,
    },

    /// Restore project to a specific snapshot
    #[command(visible_alias = "rs")]
    #[command(about = "Restore project to a specific snapshot")]
    Restore {
        /// Snapshot reference (e.g. snap-000001, #0, or commit hash)
        #[arg(help = "Snapshot reference (e.g. snap-000001, #0, or commit hash)")]
        snapshot: String,

        /// Path to the project, defaults to current directory
        #[arg(
            default_value = ".",
            help = "Path to the project, defaults to current directory"
        )]
        path: String,

        /// Dry run: show what would be changed without making any modifications
        #[arg(
            long,
            default_value = "false",
            help = "Dry run: show what would be changed without making any modifications"
        )]
        dry_run: bool,
    },
}
