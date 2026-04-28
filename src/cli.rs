use clap::builder::styling::Styles;
use clap::{Parser, Subcommand, ValueEnum};

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
    /// Snapshot a project
    #[command(about = "Snapshot a project")]
    Snap {
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
