//! Status command definitions

use clap::{Args, ValueEnum};

/// Status filter enumeration
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum StatusFilter {
    Dirty,
    Clean,
    Ahead,
    Behind,
}

/// Show status of all code repositories
#[derive(Args)]
#[command(visible_alias = "st")]
#[command(about = "Show status of all code repositories")]
pub struct StatusCmd {
    /// Maximum depth to search for repositories
    #[arg(
        long,
        short,
        default_value = "3",
        help = "Maximum depth to search for repositories"
    )]
    pub max_depth: Option<usize>,

    /// Show short status (branch + clean/dirty only)
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Show short status (branch + clean/dirty only)"
    )]
    pub short: bool,

    /// Filter repositories by status
    #[arg(
        long,
        short,
        value_enum,
        help = "Filter repositories by status: dirty, clean, ahead, behind"
    )]
    pub filter: Option<StatusFilter>,

    /// Path to the directory to search for repositories
    #[arg(
        help = "Path to the directory to search for repositories (default: search upwards from current directory)"
    )]
    pub path: Option<String>,
}
