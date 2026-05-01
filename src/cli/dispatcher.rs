//! Command dispatcher module
//!
//! This module routes parsed commands to appropriate handlers.

use super::{CommandArgs, CommandDispatcher, CommandName, ParsedCommand};
use crate::commands::{
    Command, CommandError, branch::BranchCommand, config::ConfigCommand, doctor::DoctorCommand,
    fork::ForkCommand, gitlab::GitLabCommand, release::ReleaseCommand, selfman::SelfManCommand,
    snap::SnapCommand, status::StatusCommand, sync::SyncCommand,
};

/// Command dispatcher implementation
pub struct CommandDispatcherImpl;

impl CommandDispatcher for CommandDispatcherImpl {
    fn dispatch(command: ParsedCommand) -> super::CliResult {
        // Route command to appropriate handler based on command name and arguments
        match (command.name, command.args) {
            (CommandName::Release, CommandArgs::Release(args)) => {
                ReleaseCommand::execute(args).map_err(|e| convert_command_error(e, "release"))
            }
            (CommandName::Sync, CommandArgs::Sync(args)) => {
                SyncCommand::execute(args).map_err(|e| convert_command_error(e, "sync"))
            }
            (CommandName::Doctor, CommandArgs::Doctor(args)) => {
                DoctorCommand::execute(args).map_err(|e| convert_command_error(e, "doctor"))
            }
            (CommandName::Fork, CommandArgs::Fork(args)) => {
                ForkCommand::execute(args).map_err(|e| convert_command_error(e, "fork"))
            }
            (CommandName::GitLab, CommandArgs::GitLab(args)) => {
                GitLabCommand::execute(args).map_err(|e| convert_command_error(e, "gitlab"))
            }
            (CommandName::Snap, CommandArgs::Snap(args)) => {
                SnapCommand::execute(args).map_err(|e| convert_command_error(e, "snap"))
            }
            (CommandName::Status, CommandArgs::Status(args)) => {
                StatusCommand::execute(args).map_err(|e| convert_command_error(e, "status"))
            }
            (CommandName::Branch, CommandArgs::Branch(args)) => {
                BranchCommand::execute(args).map_err(|e| convert_command_error(e, "branch"))
            }
            (CommandName::SelfMan, CommandArgs::SelfMan(args)) => {
                SelfManCommand::execute(args).map_err(|e| convert_command_error(e, "self"))
            }
            (CommandName::Config, CommandArgs::Config(args)) => {
                ConfigCommand::execute(args).map_err(|e| convert_command_error(e, "config"))
            }
            // This should never happen if the parser works correctly
            _ => Err(anyhow::anyhow!("Command name and argument type mismatch")),
        }
    }
}

/// Convert command error to CLI error with context
fn convert_command_error(error: CommandError, command_name: &str) -> anyhow::Error {
    match error {
        CommandError::InvalidArguments(msg) => {
            anyhow::anyhow!("Invalid arguments for {} command: {}", command_name, msg)
        }
        CommandError::ExecutionFailed(msg) => {
            anyhow::anyhow!("{} command execution failed: {}", command_name, msg)
        }
        CommandError::Domain(domain_error) => {
            anyhow::anyhow!("Domain error in {} command: {}", command_name, domain_error)
        }
        CommandError::Io(io_error) => {
            anyhow::anyhow!("I/O error in {} command: {}", command_name, io_error)
        }
        CommandError::Validation(msg) => {
            anyhow::anyhow!("Validation error in {} command: {}", command_name, msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{CommandArgs, CommandName, ParsedCommand};

    #[test]
    fn test_dispatcher_routes_to_correct_command() {
        // This test verifies that the dispatcher correctly routes commands
        // Note: Actual command execution is tested in command module tests
        let release_args = crate::commands::release::ReleaseArgs {
            bump_type: crate::commands::release::BumpType::Patch,
            files: vec![],
            no_root: false,
            force: false,
            skip_push: true,
            dry_run: true,
            message: None,
            pre_release: None,
        };

        let command = ParsedCommand {
            name: CommandName::Release,
            args: CommandArgs::Release(release_args),
        };

        // The dispatcher should call ReleaseCommand::execute
        // ReleaseCommand::execute returns a Result, and in test environment
        // without a git repository, it should return an error
        let result = CommandDispatcherImpl::dispatch(command);

        // The dispatcher should return an error since we're not in a git repo
        assert!(result.is_err());
    }

    #[test]
    fn test_error_conversion() {
        use crate::commands::CommandError;

        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let command_error = CommandError::Io(io_error);

        let converted = convert_command_error(command_error, "test");
        let error_string = converted.to_string();

        assert!(error_string.contains("I/O error in test command"));
        assert!(error_string.contains("file not found"));
    }
}
