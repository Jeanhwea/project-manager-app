//! Doctor command definitions

use clap::Args;

/// Diagnostic project health
#[derive(Args)]
#[command(about = "Diagnostic project health")]
pub struct DoctorCmd {
    /// Maximum depth to search for repositories
    #[arg(
        long,
        short,
        default_value = "3",
        help = "Maximum depth to search for repositories"
    )]
    pub max_depth: Option<usize>,

    /// Whether to perform garbage collection
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Whether to perform garbage collection"
    )]
    pub gc: bool,

    /// Whether to rename remotes to their canonical names
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Whether to rename remotes to their canonical names"
    )]
    pub rename: bool,

    /// Whether to automatically fix detected issues
    #[arg(
        long,
        default_value = "false",
        help = "Whether to automatically fix detected issues"
    )]
    pub fix: bool,

    /// Path to the directory to search for repositories
    #[arg(
        help = "Path to the directory to search for repositories (default: search upwards from current directory)"
    )]
    pub path: Option<String>,

    /// Dry run: show what would be changed without making any modifications
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be changed without making any modifications"
    )]
    pub dry_run: bool,
}
