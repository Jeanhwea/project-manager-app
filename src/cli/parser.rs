use super::cli::{
    BranchCommands, BumpType, Cli, Commands, ConfigCommands, GitlabCommands, SelfCommands,
    SnapCommands,
};
use super::{CliParser, CommandArgs, CommandName, ParsedCommand};
use clap::Parser;

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
            } => {
                let bump_type = match bump_type {
                    BumpType::Major => crate::commands::release::BumpType::Major,
                    BumpType::Minor => crate::commands::release::BumpType::Minor,
                    BumpType::Patch => crate::commands::release::BumpType::Patch,
                };

                ParsedCommand {
                    name: CommandName::Release,
                    args: CommandArgs::Release(crate::commands::release::ReleaseArgs {
                        bump_type,
                        files,
                        no_root,
                        force,
                        skip_push,
                        dry_run,
                        message,
                        pre_release,
                    }),
                }
            }
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
                args: CommandArgs::Sync(crate::commands::sync::SyncArgs {
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
                args: CommandArgs::Doctor(crate::commands::doctor::DoctorArgs {
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
                args: CommandArgs::Fork(crate::commands::fork::ForkArgs {
                    path,
                    name,
                    dry_run,
                }),
            },
            Commands::Gitlab { command } => {
                match command {
                    GitlabCommands::Login {
                        server,
                        token,
                        protocol,
                    } => ParsedCommand {
                        name: CommandName::GitLab,
                        args: CommandArgs::GitLab(crate::commands::gitlab::GitLabArgs::Login(
                            crate::commands::gitlab::LoginArgs {
                                server,
                                token,
                                protocol,
                            },
                        )),
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
                        args: CommandArgs::GitLab(crate::commands::gitlab::GitLabArgs::Clone(
                            crate::commands::gitlab::CloneArgs {
                                group,
                                server,
                                token,
                                protocol,
                                output,
                                include_archived,
                                recursive,
                                dry_run,
                            },
                        )),
                    },
                }
            }
            Commands::Snap { command } => {
                match command {
                    SnapCommands::Create { path, dry_run } => ParsedCommand {
                        name: CommandName::Snap,
                        args: CommandArgs::Snap(crate::commands::snap::SnapArgs::Create(
                            crate::commands::snap::CreateArgs { path, dry_run },
                        )),
                    },
                    SnapCommands::List { path } => ParsedCommand {
                        name: CommandName::Snap,
                        args: CommandArgs::Snap(crate::commands::snap::SnapArgs::List(
                            crate::commands::snap::ListArgs { path },
                        )),
                    },
                    SnapCommands::Restore {
                        snapshot,
                        path,
                        dry_run,
                    } => ParsedCommand {
                        name: CommandName::Snap,
                        args: CommandArgs::Snap(crate::commands::snap::SnapArgs::Restore(
                            crate::commands::snap::RestoreArgs {
                                snapshot,
                                path,
                                dry_run,
                            },
                        )),
                    },
                }
            }
            Commands::Status {
                max_depth,
                short,
                filter,
                path,
            } => ParsedCommand {
                name: CommandName::Status,
                args: CommandArgs::Status(crate::commands::status::StatusArgs {
                    max_depth,
                    short,
                    filter,
                    path,
                }),
            },
            Commands::Branch { command } => {
                match command {
                    BranchCommands::List { max_depth, path } => ParsedCommand {
                        name: CommandName::Branch,
                        args: CommandArgs::Branch(crate::commands::branch::BranchArgs::List(
                            crate::commands::branch::ListArgs { max_depth, path },
                        )),
                    },
                    BranchCommands::Clean {
                        max_depth,
                        remote,
                        path,
                        dry_run,
                    } => ParsedCommand {
                        name: CommandName::Branch,
                        args: CommandArgs::Branch(crate::commands::branch::BranchArgs::Clean(
                            crate::commands::branch::CleanArgs {
                                max_depth,
                                remote,
                                path,
                                dry_run,
                            },
                        )),
                    },
                    BranchCommands::Switch {
                        branch,
                        create,
                        max_depth,
                        path,
                        dry_run,
                    } => ParsedCommand {
                        name: CommandName::Branch,
                        args: CommandArgs::Branch(crate::commands::branch::BranchArgs::Switch(
                            crate::commands::branch::SwitchArgs {
                                branch,
                                create,
                                max_depth,
                                path,
                                dry_run,
                            },
                        )),
                    },
                    BranchCommands::Rename {
                        old_name,
                        new_name,
                        max_depth,
                        path,
                        dry_run,
                    } => ParsedCommand {
                        name: CommandName::Branch,
                        args: CommandArgs::Branch(crate::commands::branch::BranchArgs::Rename(
                            crate::commands::branch::RenameArgs {
                                old_name,
                                new_name,
                                max_depth,
                                path,
                                dry_run,
                            },
                        )),
                    },
                }
            }
            Commands::Self_ { command } => {
                match command {
                    SelfCommands::Update { force } => ParsedCommand {
                        name: CommandName::SelfMan,
                        args: CommandArgs::SelfMan(
                            crate::commands::selfman::SelfManArgs::Update(
                                crate::commands::selfman::UpdateArgs { force },
                            ),
                        ),
                    },
                    SelfCommands::Version => ParsedCommand {
                        name: CommandName::SelfMan,
                        args: CommandArgs::SelfMan(
                            crate::commands::selfman::SelfManArgs::Version,
                        ),
                    },
                }
            }
            Commands::Config { command } => {
                match command {
                    ConfigCommands::Init => ParsedCommand {
                        name: CommandName::Config,
                        args: CommandArgs::Config(crate::commands::config::ConfigArgs::Init),
                    },
                    ConfigCommands::Show => ParsedCommand {
                        name: CommandName::Config,
                        args: CommandArgs::Config(crate::commands::config::ConfigArgs::Show),
                    },
                    ConfigCommands::Path => ParsedCommand {
                        name: CommandName::Config,
                        args: CommandArgs::Config(crate::commands::config::ConfigArgs::Path),
                    },
                }
            }
        };

        Ok(parsed_command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_command_release_variant() {
        let release_args = crate::commands::release::ReleaseArgs {
            bump_type: crate::commands::release::BumpType::Patch,
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
