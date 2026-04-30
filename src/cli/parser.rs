//! CLI parser module
//!
//! This module handles command-line argument parsing and validation.

use super::{CliParser, CommandName, ParsedCommand};

/// CLI parser implementation
pub struct ClapParser;

impl CliParser for ClapParser {
    fn parse() -> Result<ParsedCommand, anyhow::Error> {
        // Implementation will be added in Task 2.1
        todo!("CLI parsing not yet implemented")
    }
}