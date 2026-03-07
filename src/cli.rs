use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "pma")]
#[command(about = "项目管理工具 (Project Manager Application)")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum BumpType {
    Major,
    Minor,
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
    Release {
        /// 升级类型
        #[arg(value_enum, default_value = "patch")]
        bump_type: BumpType,
    },
    /// 同步所有代码仓库
    #[command(visible_alias = "sync")]
    Synchronize {
        /// 搜索的最大深度
        #[arg(long, short, default_value = "3")]
        max_depth: Option<usize>,
        /// 要搜索的目录路径，默认为当前目录
        #[arg(default_value = ".")]
        path: String,
    },
    /// 清理项目信息
    #[command(visible_alias = "hk")]
    Housekeeping {
        /// 搜索的最大深度
        #[arg(long, short, default_value = "3")]
        max_depth: Option<usize>,
        /// 要执行垃圾回收
        #[arg(long, short, default_value = "false")]
        gc: bool,
        /// 要搜索的目录路径，默认为当前目录
        #[arg(default_value = ".")]
        path: String,
    },
}
