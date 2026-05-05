//! CLI module
//!
//! Provides command-line interface parsing and dispatching.

mod args;
mod commands;
mod dispatcher;
mod parser;
mod styles;

// CLI types
pub use commands::{CommandArgs, CommandName, ParsedCommand};

// Parser
pub use parser::{ClapParser, CliParser};

// Dispatcher
pub use dispatcher::{CommandDispatcher, CommandDispatcherImpl};

// Utilities
pub use args::BumpType;
pub use styles::get_styles;

/// CLI result type
pub type CliResult = Result<(), anyhow::Error>;
