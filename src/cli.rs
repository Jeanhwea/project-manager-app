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
        /// Path to the directory to search for repositories, defaults to current directory
        #[arg(
            default_value = ".",
            help = "Path to the directory to search for repositories, defaults to current directory"
        )]
        path: String,
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
    },
}
