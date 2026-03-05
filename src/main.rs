mod app;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "project-manager-app")]
#[command(about = "项目管理工具")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 发布新的版本
    #[command(alias = "re")]
    Release {
        /// 升级类型: major, minor, patch
        #[arg(default_value = "patch")]
        bump_type: String,
    },
    /// 同步所有git仓库
    #[command(alias = "sync")]
    Synchronize {
        /// 要搜索的目录路径，默认为当前目录
        #[arg(long, short, default_value = ".")]
        path: String,
    },
}

impl Commands {
    fn bump_type(&self) -> &str {
        match self {
            Commands::Release { bump_type } => bump_type.as_str(),
            Commands::Synchronize { .. } => "",
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Release { .. } => {
            app::release::execute(cli.command.bump_type());
        }
        Commands::Synchronize { path } => {
            app::sync::execute(&path);
        }
    }
}
