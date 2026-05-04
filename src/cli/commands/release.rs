//! Release command definitions

use super::super::BumpType;
use clap::Args;

/// Release a new version
#[derive(Args)]
#[command(visible_alias = "re")]
#[command(about = "Release a new version")]
pub struct ReleaseCmd {
    /// Bump type: major, minor, patch
    #[arg(
        value_enum,
        default_value = "patch",
        help = "Bump type: major, minor, patch"
    )]
    pub bump_type: BumpType,

    /// Files to update version (auto-detect if not specified)
    #[arg(help = "Files to update version (auto-detect if not specified)")]
    pub files: Vec<String>,

    /// Stay in current directory instead of switching to git root
    #[arg(
        long,
        short = 'n',
        default_value = "false",
        help = "Stay in current directory instead of switching to git root"
    )]
    pub no_root: bool,

    /// Force release even if not on master branch
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Force release even if not on master branch"
    )]
    pub force: bool,

    /// Skip pushing tags and branches to remotes
    #[arg(
        long,
        default_value = "false",
        help = "Skip pushing tags and branches to remotes"
    )]
    pub skip_push: bool,

    /// Dry run: show what would be changed without making any modifications
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be changed without making any modifications"
    )]
    pub dry_run: bool,

    /// Custom commit message (tag name will be prepended automatically)
    #[arg(
        long,
        short = 'm',
        help = "Custom commit message (tag name will be prepended automatically)"
    )]
    pub message: Option<String>,

    /// Pre-release suffix (e.g. "alpha", "rc.1" -> v1.0.0-alpha)
    #[arg(
        long,
        help = "Pre-release suffix (e.g. \"alpha\", \"rc.1\" -> v1.0.0-alpha)"
    )]
    pub pre_release: Option<String>,
}
