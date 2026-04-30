//! Command dispatcher module
//!
//! This module routes parsed commands to appropriate handlers.

use super::{CommandDispatcher, ParsedCommand};

/// Command dispatcher implementation
pub struct CommandDispatcherImpl;

impl CommandDispatcher for CommandDispatcherImpl {
    fn dispatch(command: ParsedCommand) -> super::CliResult {
        // Implementation will be added in Task 2.2
        todo!("Command dispatching not yet implemented")
    }
}