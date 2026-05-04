//! Self management command definitions

use clap::Subcommand;

/// Self management commands
#[derive(Subcommand)]
#[command(name = "self", about = "Self management commands")]
pub enum SelfCommands {
    /// Update to the latest version from GitHub releases
    #[command(visible_alias = "up")]
    #[command(about = "Update to the latest version from GitHub releases")]
    Update {
        #[arg(
            long,
            short,
            default_value = "false",
            help = "Force update even if already on the latest version"
        )]
        force: bool,
    },

    /// Display the current version
    #[command(visible_alias = "ver")]
    #[command(about = "Display the current version")]
    Version,
}
