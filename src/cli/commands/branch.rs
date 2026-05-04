//! Branch command definitions

use clap::Subcommand;

/// Manage branches across repositories
#[derive(Subcommand)]
#[command(visible_alias = "br")]
#[command(about = "Manage branches across repositories")]
pub enum BranchCommands {
    /// List branches across all repositories
    #[command(visible_alias = "ls")]
    #[command(about = "List branches across all repositories")]
    List {
        /// Maximum depth to search for repositories
        #[arg(
            long,
            short,
            default_value = "3",
            help = "Maximum depth to search for repositories"
        )]
        max_depth: Option<usize>,

        /// Path to the directory to search for repositories
        #[arg(
            help = "Path to the directory to search for repositories (default: search upwards from current directory)"
        )]
        path: Option<String>,
    },

    /// Clean merged branches across all repositories
    #[command(about = "Clean merged branches across all repositories")]
    Clean {
        /// Maximum depth to search for repositories
        #[arg(
            long,
            short,
            default_value = "3",
            help = "Maximum depth to search for repositories"
        )]
        max_depth: Option<usize>,

        /// Also delete remote merged branches
        #[arg(
            long,
            short,
            default_value = "false",
            help = "Also delete remote merged branches"
        )]
        remote: bool,

        /// Path to the directory to search for repositories
        #[arg(
            help = "Path to the directory to search for repositories (default: search upwards from current directory)"
        )]
        path: Option<String>,

        /// Dry run: show what would be changed without making any modifications
        #[arg(
            long,
            default_value = "false",
            help = "Dry run: show what would be changed without making any modifications"
        )]
        dry_run: bool,
    },

    /// Switch to a branch across all repositories
    #[command(visible_alias = "sw")]
    #[command(about = "Switch to a branch across all repositories")]
    Switch {
        /// Branch name to switch to
        #[arg(help = "Branch name to switch to")]
        branch: String,

        /// Create the branch if it does not exist
        #[arg(
            long,
            short = 'c',
            default_value = "false",
            help = "Create the branch if it does not exist"
        )]
        create: bool,

        /// Maximum depth to search for repositories
        #[arg(
            long,
            short,
            default_value = "3",
            help = "Maximum depth to search for repositories"
        )]
        max_depth: Option<usize>,

        /// Path to the directory to search for repositories
        #[arg(
            help = "Path to the directory to search for repositories (default: search upwards from current directory)"
        )]
        path: Option<String>,

        /// Dry run: show what would be changed without making any modifications
        #[arg(
            long,
            default_value = "false",
            help = "Dry run: show what would be changed without making any modifications"
        )]
        dry_run: bool,
    },

    /// Rename a branch across all repositories
    #[command(visible_alias = "mv")]
    #[command(about = "Rename a branch across all repositories")]
    Rename {
        /// Old branch name
        #[arg(help = "Old branch name")]
        old_name: String,

        /// New branch name
        #[arg(help = "New branch name")]
        new_name: String,

        /// Maximum depth to search for repositories
        #[arg(
            long,
            short,
            default_value = "3",
            help = "Maximum depth to search for repositories"
        )]
        max_depth: Option<usize>,

        /// Path to the directory to search for repositories
        #[arg(
            help = "Path to the directory to search for repositories (default: search upwards from current directory)"
        )]
        path: Option<String>,

        /// Dry run: show what would be changed without making any modifications
        #[arg(
            long,
            default_value = "false",
            help = "Dry run: show what would be changed without making any modifications"
        )]
        dry_run: bool,
    },
}
