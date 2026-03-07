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
    /// 发布新的版本
    #[command(visible_alias = "re")]
    #[command(about = "发布新的版本")]
    Release {
        /// 升级类型：major（主版本）、minor（次版本）、patch（修订版本）
        #[arg(value_enum, default_value = "patch", help = "升级类型")]
        bump_type: BumpType,
    },
    /// 同步所有代码仓库
    #[command(visible_alias = "sync")]
    #[command(about = "同步所有代码仓库")]
    Synchronize {
        /// 搜索的最大深度
        #[arg(long, short, default_value = "3", help = "搜索的最大深度")]
        max_depth: Option<usize>,
        /// 要搜索的目录路径，默认为当前目录
        #[arg(default_value = ".", help = "要搜索的目录路径")]
        path: String,
    },
    /// 清理项目信息
    #[command(visible_alias = "hk")]
    #[command(about = "清理项目信息")]
    Housekeeping {
        /// 搜索的最大深度
        #[arg(long, short, default_value = "3", help = "搜索的最大深度")]
        max_depth: Option<usize>,
        /// 要执行垃圾回收
        #[arg(long, short, default_value = "false", help = "执行垃圾回收")]
        gc: bool,
        /// 要搜索的目录路径，默认为当前目录
        #[arg(default_value = ".", help = "要搜索的目录路径")]
        path: String,
    },
}
