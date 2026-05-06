use clap::Parser;

use crate::commands::{
    branch::BranchArgs, config::ConfigArgs, doctor::DoctorArgs, fork::ForkArgs,
    gitlab::GitLabArgs, release::ReleaseArgs, selfman::SelfManArgs, snap::SnapArgs,
    status::StatusArgs, sync::SyncArgs,
};

#[derive(Parser)]
#[command(name = "pma")]
#[command(about = "Project Manager Application (项目管理工具)")]
#[command(version)]
#[command(styles = crate::cli::get_styles())]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    #[command(visible_alias = "re")]
    Release(ReleaseArgs),

    #[command(visible_alias = "s")]
    Sync(SyncArgs),

    Doctor(DoctorArgs),

    Fork(ForkArgs),

    #[command(visible_alias = "gl")]
    Gitlab {
        #[command(subcommand)]
        command: GitLabArgs,
    },

    Snap {
        #[command(subcommand)]
        command: SnapArgs,
    },

    #[command(visible_alias = "st")]
    Status(StatusArgs),

    #[command(visible_alias = "br")]
    Branch {
        #[command(subcommand)]
        command: BranchArgs,
    },

    #[command(name = "self")]
    Self_ {
        #[command(subcommand)]
        command: SelfManArgs,
    },

    Config {
        #[command(subcommand)]
        command: ConfigArgs,
    },
}

#[derive(Debug)]
pub enum CommandArgs {
    Release(ReleaseArgs),
    Sync(SyncArgs),
    Doctor(DoctorArgs),
    Fork(ForkArgs),
    GitLab(GitLabArgs),
    Snap(SnapArgs),
    Status(StatusArgs),
    Branch(BranchArgs),
    SelfMan(SelfManArgs),
    Config(ConfigArgs),
}

impl CommandArgs {
    pub fn command_name(&self) -> &'static str {
        match self {
            CommandArgs::Release(_) => "release",
            CommandArgs::Sync(_) => "sync",
            CommandArgs::Doctor(_) => "doctor",
            CommandArgs::Fork(_) => "fork",
            CommandArgs::GitLab(_) => "gitlab",
            CommandArgs::Snap(_) => "snap",
            CommandArgs::Status(_) => "status",
            CommandArgs::Branch(_) => "branch",
            CommandArgs::SelfMan(_) => "self",
            CommandArgs::Config(_) => "config",
        }
    }
}
