//! Sync command definitions

use clap::Args;

/// Synchronize all code repositories
#[derive(Args)]
#[command(visible_alias = "s")]
#[command(about = "Synchronize all code repositories")]
pub struct SyncCmd {
    /// Maximum depth to search for repositories
    #[arg(
        long,
        short,
        default_value = "3",
        help = "Maximum depth to search for repositories"
    )]
    pub max_depth: Option<usize>,

    /// Remotes to skip
    #[arg(long, short, help = "Remotes to skip")]
    pub skip_remotes: Vec<String>,

    /// Whether to pull all local branches
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Whether to pull all local branches"
    )]
    pub all_branch: bool,

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

    /// Only fetch from remotes, do not pull or push
    #[arg(
        long,
        short = 'f',
        default_value = "false",
        help = "Only fetch from remotes, do not pull or push"
    )]
    pub fetch_only: bool,

    /// Use rebase instead of merge when pulling
    #[arg(
        long,
        default_value = "false",
        help = "Use rebase instead of merge when pulling"
    )]
    pub rebase: bool,
}
