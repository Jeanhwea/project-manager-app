//! Config command definitions

use clap::Subcommand;

/// Manage configuration
#[derive(Subcommand)]
#[command(about = "Manage configuration")]
pub enum ConfigCommands {
    /// Initialize a default configuration file
    #[command(about = "Initialize a default configuration file")]
    Init,

    /// Show current configuration
    #[command(about = "Show current configuration")]
    Show,

    /// Show configuration file path
    #[command(about = "Show configuration file path")]
    Path,
}
