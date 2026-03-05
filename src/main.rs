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
    /// 版本升级并创建新 tag
    Release {
        /// 升级类型: major, minor, patch
        #[arg(default_value = "patch")]
        bump_type: String,
    },
}

impl Commands {
    fn bump_type(&self) -> &str {
        match self {
            Commands::Release { bump_type } => bump_type.as_str(),
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Release { .. } => {
            app::release::execute(cli.command.bump_type());
        }
    }
}
