use super::{CommandArgs, CommandDispatcher, CommandName, ParsedCommand};
use crate::commands::{
    Command, CommandError, branch::BranchCommand, config::ConfigCommand, doctor::DoctorCommand,
    fork::ForkCommand, gitlab::GitLabCommand, release::ReleaseCommand, selfman::SelfManCommand,
    snap::SnapCommand, status::StatusCommand, sync::SyncCommand,
};

pub struct CommandDispatcherImpl;

impl CommandDispatcher for CommandDispatcherImpl {
    fn dispatch(command: ParsedCommand) -> super::CliResult {
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
            _ => Err(anyhow::anyhow!("Command name and argument type mismatch")),
        }
    }
}

fn convert_command_error(error: CommandError, command_name: &str) -> anyhow::Error {
    match error {
        CommandError::InvalidArguments(msg) => {
            anyhow::anyhow!("Invalid arguments for {} command: {}", command_name, msg)
        }
        CommandError::ExecutionFailed(msg) => {
            anyhow::anyhow!("{} command execution failed: {}", command_name, msg)
        }
        CommandError::Git(git_error) => {
            anyhow::anyhow!("Git error in {} command: {}", command_name, git_error)
        }
        CommandError::Editor(editor_error) => {
            anyhow::anyhow!("Editor error in {} command: {}", command_name, editor_error)
        }
        CommandError::Config(config_error) => {
            anyhow::anyhow!("Config error in {} command: {}", command_name, config_error)
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
    fn test_dispatch_release_without_git_repo() {
        let args = crate::commands::release::ReleaseArgs {
            bump_type: crate::cli::BumpType::Patch,
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
            args: CommandArgs::Release(args),
        };

        // Should error — no git repo in test environment
        assert!(CommandDispatcherImpl::dispatch(command).is_err());
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let converted =
            convert_command_error(crate::commands::CommandError::Io(io_error), "test");
        let msg = converted.to_string();
        assert!(msg.contains("I/O error in test command"));
        assert!(msg.contains("file not found"));
    }
}
