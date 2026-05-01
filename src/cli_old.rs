use clap::builder::styling::Styles;
use clap::{Parser, Subcommand, ValueEnum};

use crate::commands::gitlab::CloneProtocol;
use crate::commands::status::StatusFilter;

fn get_styles() -> Styles {
    Styles::styled()
        .header(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .literal(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .placeholder(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
        .error(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .valid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan))),
        )
        .invalid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
}

#[derive(Parser)]
#[command(name = "pma")]
#[command(about = "Project Manager Application (项目管理工具)")]
#[command(version)]
#[command(styles = get_styles())]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum BumpType {
    /// Major version bump (e.g. 1.0.0 -> 2.0.0)
    #[value(alias = "ma")]
    Major,
    /// Minor version bump (e.g. 1.0.0 -> 1.1.0)
    #[value(alias = "mi")]
    Minor,
    /// Patch version bump (e.g. 1.0.0 -> 1.0.1)
    #[value(alias = "pa")]
    Patch,
}

impl BumpType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BumpType::Major => "major",
            BumpType::Minor => "minor",
            BumpType::Patch => "patch",
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Release a new version
    #[command(visible_alias = "re")]
    #[command(about = "Release a new version")]
    Release {
        /// Bump type: major, minor, patch
        #[arg(
            value_enum,
            default_value = "patch",
            help = "Bump type: major, minor, patch"
        )]
        bump_type: BumpType,
        /// Files to update version (auto-detect if not specified)
        #[arg(help = "Files to update version (auto-detect if not specified)")]
        files: Vec<String>,
        /// Stay in current directory instead of switching to git root
        #[arg(
            long,
            short = 'n',
            default_value = "false",
            help = "Stay in current directory instead of switching to git root"
        )]
        no_root: bool,
        /// Force release even if not on master branch
        #[arg(
            long,
            short,
            default_value = "false",
            help = "Force release even if not on master branch"
        )]
        force: bool,
        /// Skip pushing tags and branches to remotes
        #[arg(
            long,
            default_value = "false",
            help = "Skip pushing tags and branches to remotes"
        )]
        skip_push: bool,
        /// Dry run: show what would be changed without making any modifications
        #[arg(
            long,
            default_value = "false",
            help = "Dry run: show what would be changed without making any modifications"
        )]
        dry_run: bool,
        /// Custom commit message (tag name will be prepended automatically)
        #[arg(
            long,
            short = 'm',
            help = "Custom commit message (tag name will be prepended automatically)"
        )]
        message: Option<String>,
        /// Pre-release suffix (e.g. "alpha", "rc.1" -> v1.0.0-alpha)
        #[arg(
            long,
            help = "Pre-release suffix (e.g. \"alpha\", \"rc.1\" -> v1.0.0-alpha)"
        )]
        pre_release: Option<String>,
    },
    /// Synchronize all code repositories
    #[command(visible_alias = "s")]
    #[command(about = "Synchronize all code repositories")]
    Sync {
        /// Maximum depth to search for repositories
        #[arg(
            long,
            short,
            default_value = "3",
            help = "Maximum depth to search for repositories"
        )]
        max_depth: Option<usize>,
        /// Remotes to skip
        #[arg(long, short, help = "Remotes to skip")]
        skip_remotes: Vec<String>,
        /// Whether to pull all local branches
        #[arg(
            long,
            short,
            default_value = "false",
            help = "Whether to pull all local branches"
        )]
        all_branch: bool,
        /// Path to the directory to search for repositories, defaults to current directory
        #[arg(
            default_value = ".",
            help = "Path to the directory to search for repositories, defaults to current directory"
        )]
        path: String,
        /// Dry run: show what would be changed without making any modifications
        #[arg(
            long,
            default_value = "false",
            help = "Dry run: show what would be changed without making any modifications"
        )]
        dry_run: bool,
        /// Only fetch from remotes, do not pull or push
        #[arg(
            long,
            short = 'f',
            default_value = "false",
            help = "Only fetch from remotes, do not pull or push"
        )]
        fetch_only: bool,
        /// Use rebase instead of merge when pulling
        #[arg(
            long,
            default_value = "false",
            help = "Use rebase instead of merge when pulling"
        )]
        rebase: bool,
    },
    /// Diagnostic project health
    #[command(about = "Diagnostic project health")]
    Doctor {
        /// Maximum depth to search for repositories
        #[arg(
            long,
            short,
            default_value = "3",
            help = "Maximum depth to search for repositories"
        )]
        max_depth: Option<usize>,
        /// Whether to perform garbage collection
        #[arg(
            long,
            short,
            default_value = "false",
            help = "Whether to perform garbage collection"
        )]
        gc: bool,
        /// Whether to rename remotes to their canonical names
        #[arg(
            long,
            short,
            default_value = "false",
            help = "Whether to rename remotes to their canonical names"
        )]
        rename: bool,
        /// Whether to automatically fix detected issues
        #[arg(
            long,
            default_value = "false",
            help = "Whether to automatically fix detected issues"
        )]
        fix: bool,
        /// Path to the directory to search for repositories, defaults to current directory
        #[arg(
            default_value = ".",
            help = "Path to the directory to search for repositories, defaults to current directory"
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
    /// Fork a new project from a template
    #[command(about = "Fork a new project from a template")]
    Fork {
        /// Path to fork the project from
        #[arg(help = "Path to fork the project from")]
        path: String,

        /// Name of the project
        #[arg(help = "Name of the project")]
        name: String,

        /// Dry run: show what would be changed without making any modifications
        #[arg(
            long,
            default_value = "false",
            help = "Dry run: show what would be changed without making any modifications"
        )]
        dry_run: bool,
    },
    /// GitLab integration commands
    #[command(visible_alias = "gl")]
    #[command(about = "GitLab integration commands")]
    Gitlab {
        #[command(subcommand)]
        command: GitlabCommands,
    },
    /// Snapshot a project
    #[command(about = "Snapshot a project")]
    Snap {
        #[command(subcommand)]
        command: SnapCommands,
    },
    /// Show status of all code repositories
    #[command(visible_alias = "st")]
    #[command(about = "Show status of all code repositories")]
    Status {
        /// Maximum depth to search for repositories
        #[arg(
            long,
            short,
            default_value = "3",
            help = "Maximum depth to search for repositories"
        )]
        max_depth: Option<usize>,
        /// Show short status (branch + clean/dirty only)
        #[arg(
            long,
            short,
            default_value = "false",
            help = "Show short status (branch + clean/dirty only)"
        )]
        short: bool,
        /// Filter repositories by status
        #[arg(
            long,
            short,
            value_enum,
            help = "Filter repositories by status: dirty, clean, ahead, behind"
        )]
        filter: Option<StatusFilter>,
        /// Path to the directory to search for repositories, defaults to current directory
        #[arg(
            default_value = ".",
            help = "Path to the directory to search for repositories, defaults to current directory"
        )]
        path: String,
    },
    /// Manage branches across repositories
    #[command(visible_alias = "br")]
    #[command(about = "Manage branches across repositories")]
    Branch {
        #[command(subcommand)]
        command: BranchCommands,
    },
    /// Self management commands
    #[command(name = "self", about = "Self management commands")]
    Self_ {
        #[command(subcommand)]
        command: SelfCommands,
    },
    /// Manage configuration
    #[command(about = "Manage configuration")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
pub enum GitlabCommands {
    /// Login to a GitLab server and save credentials
    #[command(about = "Login to a GitLab server and save credentials")]
    Login {
        /// GitLab server URL (will prompt if not provided)
        #[arg(
            long,
            short,
            help = "GitLab server URL (e.g. https://gitlab.com, http://192.168.0.110/gitlab/)"
        )]
        server: Option<String>,
        /// GitLab Personal Access Token (required, will prompt if not provided)
        #[arg(long, short = 't', help = "GitLab Personal Access Token (required)")]
        token: Option<String>,
        /// Default clone protocol
        #[arg(
            long,
            short = 'p',
            value_enum,
            default_value = "ssh",
            help = "Default clone protocol: ssh or https"
        )]
        protocol: CloneProtocol,
    },
    /// Clone all repositories from a GitLab group
    #[command(visible_alias = "cl")]
    #[command(about = "Clone all repositories from a GitLab group")]
    Clone {
        /// GitLab group path (e.g. "my-org/team" or numeric ID)
        #[arg(help = "GitLab group path (e.g. \"my-org/team\" or numeric ID)")]
        group: String,
        /// GitLab server URL (uses saved config if not specified)
        #[arg(
            long,
            short,
            help = "GitLab server URL (uses saved config if not specified)"
        )]
        server: Option<String>,
        /// GitLab private token (overrides saved config)
        #[arg(
            long,
            short = 't',
            help = "GitLab private token (overrides saved config)"
        )]
        token: Option<String>,
        /// Clone protocol (overrides saved config)
        #[arg(
            long,
            short = 'p',
            value_enum,
            help = "Clone protocol: ssh or https (uses saved config if not specified)"
        )]
        protocol: Option<CloneProtocol>,
        /// Output directory for cloned repositories
        #[arg(
            long,
            short = 'o',
            default_value = ".",
            help = "Output directory for cloned repositories"
        )]
        output: String,
        /// Include archived projects
        #[arg(long, default_value = "false", help = "Include archived projects")]
        include_archived: bool,
        /// Clone submodules recursively
        #[arg(long, default_value = "false", help = "Clone submodules recursively")]
        recursive: bool,
        /// Dry run: show what would be changed without making any modifications
        #[arg(
            long,
            default_value = "false",
            help = "Dry run: show what would be changed without making any modifications"
        )]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
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

#[derive(Subcommand)]
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
        /// Path to the directory to search for repositories, defaults to current directory
        #[arg(
            default_value = ".",
            help = "Path to the directory to search for repositories, defaults to current directory"
        )]
        path: String,
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
        /// Path to the directory to search for repositories, defaults to current directory
        #[arg(
            default_value = ".",
            help = "Path to the directory to search for repositories, defaults to current directory"
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
        /// Path to the directory to search for repositories, defaults to current directory
        #[arg(
            default_value = ".",
            help = "Path to the directory to search for repositories, defaults to current directory"
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
        /// Path to the directory to search for repositories, defaults to current directory
        #[arg(
            default_value = ".",
            help = "Path to the directory to search for repositories, defaults to current directory"
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

#[derive(Subcommand)]
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

#[derive(Subcommand)]
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
