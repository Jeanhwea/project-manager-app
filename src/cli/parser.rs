//! CLI parser implementation

use super::{
    BranchCommands, Cli, CommandArgs, CommandName, Commands, ConfigCommands, GitlabCommands,
    ParsedCommand, SelfCommands, SnapCommands,
};
use crate::commands::{
    branch::{BranchArgs, CleanArgs, ListArgs as BranchListArgs, RenameArgs, SwitchArgs},
    config::ConfigArgs,
    doctor::DoctorArgs,
    fork::ForkArgs,
    gitlab::{CloneArgs, GitLabArgs, LoginArgs},
    release::ReleaseArgs,
    selfman::SelfManArgs,
    snap::{CreateArgs, ListArgs as SnapListArgs, RestoreArgs, SnapArgs},
    status::StatusArgs,
    sync::SyncArgs,
};
use clap::Parser;

/// Trait for parsing CLI arguments into commands
pub trait CliParser {
    fn parse() -> Result<ParsedCommand, anyhow::Error>;
}

pub struct ClapParser;

impl CliParser for ClapParser {
    fn parse() -> Result<ParsedCommand, anyhow::Error> {
        let cli = Cli::parse();

        let parsed_command = match cli.command {
            Commands::Release {
                bump_type,
                files,
                no_root,
                force,
                skip_push,
                dry_run,
                message,
                pre_release,
            } => ParsedCommand {
                name: CommandName::Release,
                args: CommandArgs::Release(ReleaseArgs {
                    bump_type,
                    files,
                    no_root,
                    force,
                    skip_push,
                    dry_run,
                    message,
                    pre_release,
                }),
            },
            Commands::Sync {
                max_depth,
                skip_remotes,
                all_branch,
                path,
                dry_run,
                fetch_only,
                rebase,
            } => ParsedCommand {
                name: CommandName::Sync,
                args: CommandArgs::Sync(SyncArgs {
                    max_depth,
                    skip_remotes,
                    all_branch,
                    path,
                    dry_run,
                    fetch_only,
                    rebase,
                }),
            },
            Commands::Doctor {
                max_depth,
                gc,
                rename,
                fix,
                path,
                dry_run,
            } => ParsedCommand {
                name: CommandName::Doctor,
                args: CommandArgs::Doctor(DoctorArgs {
                    max_depth,
                    gc,
                    rename,
                    fix,
                    path,
                    dry_run,
                }),
            },
            Commands::Fork {
                path,
                name,
                dry_run,
            } => ParsedCommand {
                name: CommandName::Fork,
                args: CommandArgs::Fork(ForkArgs {
                    path,
                    name,
                    dry_run,
                }),
            },
            Commands::Gitlab { command } => match command {
                GitlabCommands::Login {
                    server,
                    token,
                    protocol,
                } => ParsedCommand {
                    name: CommandName::GitLab,
                    args: CommandArgs::GitLab(GitLabArgs::Login(LoginArgs {
                        server,
                        token,
                        protocol,
                    })),
                },
                GitlabCommands::Clone {
                    group,
                    server,
                    token,
                    protocol,
                    output,
                    include_archived,
                    recursive,
                    dry_run,
                } => ParsedCommand {
                    name: CommandName::GitLab,
                    args: CommandArgs::GitLab(GitLabArgs::Clone(CloneArgs {
                        group,
                        server,
                        token,
                        protocol,
                        output,
                        include_archived,
                        recursive,
                        dry_run,
                    })),
                },
            },
            Commands::Snap { command } => match command {
                SnapCommands::Create { path, dry_run } => ParsedCommand {
                    name: CommandName::Snap,
                    args: CommandArgs::Snap(SnapArgs::Create(CreateArgs { path, dry_run })),
                },
                SnapCommands::List { path } => ParsedCommand {
                    name: CommandName::Snap,
                    args: CommandArgs::Snap(SnapArgs::List(SnapListArgs { path })),
                },
                SnapCommands::Restore {
                    snapshot,
                    path,
                    dry_run,
                } => ParsedCommand {
                    name: CommandName::Snap,
                    args: CommandArgs::Snap(SnapArgs::Restore(RestoreArgs {
                        snapshot,
                        path,
                        dry_run,
                    })),
                },
            },
            Commands::Status {
                max_depth,
                short,
                filter,
                path,
            } => ParsedCommand {
                name: CommandName::Status,
                args: CommandArgs::Status(StatusArgs {
                    max_depth,
                    short,
                    filter,
                    path,
                }),
            },
            Commands::Branch { command } => match command {
                BranchCommands::List { max_depth, path } => ParsedCommand {
                    name: CommandName::Branch,
                    args: CommandArgs::Branch(BranchArgs::List(BranchListArgs {
                        max_depth,
                        path,
                    })),
                },
                BranchCommands::Clean {
                    max_depth,
                    remote,
                    path,
                    dry_run,
                } => ParsedCommand {
                    name: CommandName::Branch,
                    args: CommandArgs::Branch(BranchArgs::Clean(CleanArgs {
                        max_depth,
                        remote,
                        path,
                        dry_run,
                    })),
                },
                BranchCommands::Switch {
                    branch,
                    create,
                    max_depth,
                    path,
                    dry_run,
                } => ParsedCommand {
                    name: CommandName::Branch,
                    args: CommandArgs::Branch(BranchArgs::Switch(SwitchArgs {
                        branch,
                        create,
                        max_depth,
                        path,
                        dry_run,
                    })),
                },
                BranchCommands::Rename {
                    old_name,
                    new_name,
                    max_depth,
                    path,
                    dry_run,
                } => ParsedCommand {
                    name: CommandName::Branch,
                    args: CommandArgs::Branch(BranchArgs::Rename(RenameArgs {
                        old_name,
                        new_name,
                        max_depth,
                        path,
                        dry_run,
                    })),
                },
            },
            Commands::Self_ { command } => match command {
                SelfCommands::Update { force } => ParsedCommand {
                    name: CommandName::SelfMan,
                    args: CommandArgs::SelfMan(SelfManArgs::Update(
                        crate::commands::selfman::UpdateArgs { force },
                    )),
                },
                SelfCommands::Version => ParsedCommand {
                    name: CommandName::SelfMan,
                    args: CommandArgs::SelfMan(SelfManArgs::Version),
                },
            },
            Commands::Config { command } => match command {
                ConfigCommands::Init => ParsedCommand {
                    name: CommandName::Config,
                    args: CommandArgs::Config(ConfigArgs::Init),
                },
                ConfigCommands::Show => ParsedCommand {
                    name: CommandName::Config,
                    args: CommandArgs::Config(ConfigArgs::Show),
                },
                ConfigCommands::Path => ParsedCommand {
                    name: CommandName::Config,
                    args: CommandArgs::Config(ConfigArgs::Path),
                },
            },
        };

        Ok(parsed_command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::BumpType;

    #[test]
    fn test_parsed_command_release_variant() {
        let release_args = ReleaseArgs {
            bump_type: BumpType::Patch,
            files: vec![],
            no_root: false,
            force: false,
            skip_push: false,
            dry_run: false,
            message: None,
            pre_release: None,
        };

        let command_args = CommandArgs::Release(release_args);
        assert!(matches!(command_args, CommandArgs::Release(_)));
    }
}
