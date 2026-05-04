//! CLI parser implementation

use super::commands::{
    BranchCommands, Cli, CloneProtocol, CommandArgs, CommandName, Commands, ConfigCommands,
    GitlabCommands, ParsedCommand, SelfCommands, SnapCommands,
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
            Commands::Release(cmd) => ParsedCommand {
                name: CommandName::Release,
                args: CommandArgs::Release(ReleaseArgs {
                    bump_type: cmd.bump_type,
                    files: cmd.files,
                    no_root: cmd.no_root,
                    force: cmd.force,
                    skip_push: cmd.skip_push,
                    dry_run: cmd.dry_run,
                    message: cmd.message,
                    pre_release: cmd.pre_release,
                }),
            },
            Commands::Sync(cmd) => ParsedCommand {
                name: CommandName::Sync,
                args: CommandArgs::Sync(SyncArgs {
                    max_depth: cmd.max_depth,
                    skip_remotes: cmd.skip_remotes,
                    all_branch: cmd.all_branch,
                    path: cmd.path,
                    dry_run: cmd.dry_run,
                    fetch_only: cmd.fetch_only,
                    rebase: cmd.rebase,
                }),
            },
            Commands::Doctor(cmd) => ParsedCommand {
                name: CommandName::Doctor,
                args: CommandArgs::Doctor(DoctorArgs {
                    max_depth: cmd.max_depth,
                    gc: cmd.gc,
                    rename: cmd.rename,
                    fix: cmd.fix,
                    path: cmd.path,
                    dry_run: cmd.dry_run,
                }),
            },
            Commands::Fork(cmd) => ParsedCommand {
                name: CommandName::Fork,
                args: CommandArgs::Fork(ForkArgs {
                    path: cmd.path,
                    name: cmd.name,
                    dry_run: cmd.dry_run,
                }),
            },
            Commands::Gitlab { command } => parse_gitlab_command(command),
            Commands::Snap { command } => parse_snap_command(command),
            Commands::Status(cmd) => ParsedCommand {
                name: CommandName::Status,
                args: CommandArgs::Status(StatusArgs {
                    max_depth: cmd.max_depth,
                    short: cmd.short,
                    filter: cmd.filter.map(convert_status_filter),
                    path: cmd.path,
                }),
            },
            Commands::Branch { command } => parse_branch_command(command),
            Commands::Self_ { command } => parse_self_command(command),
            Commands::Config { command } => parse_config_command(command),
        };

        Ok(parsed_command)
    }
}

/// Parse GitLab subcommand
fn parse_gitlab_command(command: GitlabCommands) -> ParsedCommand {
    match command {
        GitlabCommands::Login {
            server,
            token,
            protocol,
        } => ParsedCommand {
            name: CommandName::GitLab,
            args: CommandArgs::GitLab(GitLabArgs::Login(LoginArgs {
                server,
                token,
                protocol: convert_clone_protocol(protocol),
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
                protocol: protocol.map(convert_clone_protocol),
                output,
                include_archived,
                recursive,
                dry_run,
            })),
        },
    }
}

/// Parse Snap subcommand
fn parse_snap_command(command: SnapCommands) -> ParsedCommand {
    match command {
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
    }
}

/// Parse Branch subcommand
fn parse_branch_command(command: BranchCommands) -> ParsedCommand {
    match command {
        BranchCommands::List { max_depth, path } => ParsedCommand {
            name: CommandName::Branch,
            args: CommandArgs::Branch(BranchArgs::List(BranchListArgs { max_depth, path })),
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
    }
}

/// Parse Self management subcommand
fn parse_self_command(command: SelfCommands) -> ParsedCommand {
    match command {
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
    }
}

/// Parse Config subcommand
fn parse_config_command(command: ConfigCommands) -> ParsedCommand {
    match command {
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
    }
}

/// Convert CLI CloneProtocol to commands CloneProtocol
fn convert_clone_protocol(protocol: CloneProtocol) -> crate::commands::gitlab::CloneProtocol {
    match protocol {
        CloneProtocol::Ssh => crate::commands::gitlab::CloneProtocol::Ssh,
        CloneProtocol::Https => crate::commands::gitlab::CloneProtocol::Https,
    }
}

/// Convert CLI StatusFilter to commands StatusFilter
fn convert_status_filter(
    filter: super::commands::StatusFilter,
) -> crate::commands::status::StatusFilter {
    use super::commands::StatusFilter as CliFilter;
    use crate::commands::status::StatusFilter as CmdFilter;
    match filter {
        CliFilter::Dirty => CmdFilter::Dirty,
        CliFilter::Clean => CmdFilter::Clean,
        CliFilter::Ahead => CmdFilter::Ahead,
        CliFilter::Behind => CmdFilter::Behind,
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
