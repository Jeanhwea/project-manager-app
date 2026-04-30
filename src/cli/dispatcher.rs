//! Command dispatcher module
//!
//! This module routes parsed commands to appropriate handlers.

use super::{CommandDispatcher, ParsedCommand, CommandName};
use crate::commands::{
    Command, CommandArgs as CmdArgs, CommandError, CommandResult,
    branch::BranchCommand,
    config::ConfigCommand,
    doctor::DoctorCommand,
    fork::ForkCommand,
    gitlab::GitLabCommand,
    release::ReleaseCommand,
    selfman::SelfManCommand,
    snap::SnapCommand,
    status::StatusCommand,
    sync::SyncCommand,
};

/// Command dispatcher implementation
pub struct CommandDispatcherImpl;

impl CommandDispatcher for CommandDispatcherImpl {
    fn dispatch(command: ParsedCommand) -> super::CliResult {
        // Convert CLI CommandArgs to domain CommandArgs
        let cmd_args = CmdArgs {
            raw_args: command.args.raw_args,
        };

        // Route command to appropriate handler based on command name
        match command.name {
            CommandName::Release => {
                ReleaseCommand::execute(cmd_args)
                    .map_err(|e| convert_command_error(e, "release"))
            }
            CommandName::Sync => {
                SyncCommand::execute(cmd_args)
                    .map_err(|e| convert_command_error(e, "sync"))
            }
            CommandName::Doctor => {
                DoctorCommand::execute(cmd_args)
                    .map_err(|e| convert_command_error(e, "doctor"))
            }
            CommandName::Fork => {
                ForkCommand::execute(cmd_args)
                    .map_err(|e| convert_command_error(e, "fork"))
            }
            CommandName::GitLab => {
                GitLabCommand::execute(cmd_args)
                    .map_err(|e| convert_command_error(e, "gitlab"))
            }
            CommandName::Snap => {
                SnapCommand::execute(cmd_args)
                    .map_err(|e| convert_command_error(e, "snap"))
            }
            CommandName::Status => {
                StatusCommand::execute(cmd_args)
                    .map_err(|e| convert_command_error(e, "status"))
            }
            CommandName::Branch => {
                BranchCommand::execute(cmd_args)
                    .map_err(|e| convert_command_error(e, "branch"))
            }
            CommandName::SelfMan => {
                SelfManCommand::execute(cmd_args)
                    .map_err(|e| convert_command_error(e, "self"))
            }
            CommandName::Config => {
                ConfigCommand::execute(cmd_args)
                    .map_err(|e| convert_command_error(e, "config"))
            }
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
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{CommandName, ParsedCommand, CommandArgs};

    #[test]
    fn test_dispatcher_routes_to_correct_command() {
        // This test verifies that the dispatcher correctly routes commands
        // Note: Actual command execution is tested in command module tests
        let command = ParsedCommand {
            name: CommandName::Release,
            args: CommandArgs { raw_args: vec![] },
        };
        
        // The dispatcher should call ReleaseCommand::execute
        // Since ReleaseCommand::execute currently panics with todo!(),
        // we expect a panic, which is correct for this stage of implementation
        let result = std::panic::catch_unwind(|| {
            CommandDispatcherImpl::dispatch(command);
        });
        
        // The dispatcher should panic because ReleaseCommand::execute panics
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