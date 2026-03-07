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
#[command(about = "项目管理工具 (Project Manager Application)")]
#[command(version)]
#[command(styles = get_styles())]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum BumpType {
    /// 主版本号升级（如 1.0.0 -> 2.0.0）
    #[value(alias = "ma")]
    Major,
    /// 次版本号升级（如 1.0.0 -> 1.1.0）
    #[value(alias = "mi")]
    Minor,
    /// 修订版本号升级（如 1.0.0 -> 1.0.1）
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
        #[arg(value_enum, default_value = "patch", help = "升级类型")]
        bump_type: BumpType,
    },
    /// Synchronize all code repositories
    #[command(visible_aliases = ["sy", "sync"])]
    #[command(about = "Synchronize all code repositories")]
    Synchronize {
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
    /// Diagnostic project health
    #[command(visible_alias = "dc")]
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
}
