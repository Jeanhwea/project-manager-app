//! Fork command definitions

use clap::Args;

/// Fork a new project from a template
#[derive(Args)]
#[command(about = "Fork a new project from a template")]
pub struct ForkCmd {
    /// Path to fork the project from
    #[arg(help = "Path to fork the project from")]
    pub path: String,

    /// Name of the project
    #[arg(help = "Name of the project")]
    pub name: String,

    /// Dry run: show what would be changed without making any modifications
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be changed without making any modifications"
    )]
    pub dry_run: bool,
}
