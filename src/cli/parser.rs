//! CLI parser module
//!
//! This module handles command-line argument parsing and validation.

use anyhow;
use super::{CliParser, CommandArgs, CommandName, ParsedCommand};
use crate::cli_old::Cli;

/// CLI parser implementation
pub struct ClapParser;

impl CliParser for ClapParser {
    fn parse() -> Result<ParsedCommand, anyhow::Error> {
        let cli = Cli::parse();
        
        // Convert clap command to CommandName
        let command_name = match cli.command {
            crate::cli_old::Commands::Release { .. } => CommandName::Release,
            crate::cli_old::Commands::Sync { .. } => CommandName::Sync,
            crate::cli_old::Commands::Doctor { .. } => CommandName::Doctor,
            crate::cli_old::Commands::Fork { .. } => CommandName::Fork,
            crate::cli_old::Commands::Gitlab { .. } => CommandName::GitLab,
            crate::cli_old::Commands::Snap { .. } => CommandName::Snap,
            crate::cli_old::Commands::Status { .. } => CommandName::Status,
            crate::cli_old::Commands::Branch { .. } => CommandName::Branch,
            crate::cli_old::Commands::Self_ { .. } => CommandName::SelfMan,
            crate::cli_old::Commands::Config { .. } => CommandName::Config,
        };
        
        // Convert raw arguments to string representation for now
        // In later tasks, this will be converted to domain-specific types
        let raw_args: Vec<String> = std::env::args().skip(1).collect();
        
        // Create parsed command
        let parsed_command = ParsedCommand {
            name: command_name,
            args: CommandArgs { raw_args },
        };
        
        // Perform basic validation
        validate_command(&parsed_command)?;
        
        Ok(parsed_command)
    }
}

/// Validate parsed command
fn validate_command(command: &ParsedCommand) -> Result<(), anyhow::Error> {
    match command.name {
        CommandName::Release => validate_release_command(&command.args),
        CommandName::Sync => validate_sync_command(&command.args),
        CommandName::Doctor => validate_doctor_command(&command.args),
        CommandName::Fork => validate_fork_command(&command.args),
        CommandName::GitLab => validate_gitlab_command(&command.args),
        CommandName::Snap => validate_snap_command(&command.args),
        CommandName::Status => validate_status_command(&command.args),
        CommandName::Branch => validate_branch_command(&command.args),
        CommandName::SelfMan => validate_selfman_command(&command.args),
        CommandName::Config => validate_config_command(&command.args),
    }
}

/// Validate release command arguments
fn validate_release_command(args: &CommandArgs) -> Result<(), anyhow::Error> {
    // Basic validation: ensure we have at least the bump type
    if args.raw_args.is_empty() {
        return Err(anyhow::anyhow!("Release command requires at least a bump type"));
    }
    
    // Check if bump type is valid (clap already validates this, but we double-check)
    let bump_type_arg = args.raw_args.get(0).unwrap();
    if !["major", "minor", "patch", "ma", "mi", "pa"].contains(&bump_type_arg.as_str()) {
        return Err(anyhow::anyhow!(
            "Invalid bump type: {}. Valid values are: major, minor, patch",
            bump_type_arg
        ));
    }
    
    Ok(())
}

/// Validate sync command arguments
fn validate_sync_command(args: &CommandArgs) -> Result<(), anyhow::Error> {
    // Basic validation for sync command
    // Check max_depth if provided
    for (i, arg) in args.raw_args.iter().enumerate() {
        if arg == "--max-depth" || arg == "-d" {
            if let Some(next_arg) = args.raw_args.get(i + 1) {
                if let Err(_) = next_arg.parse::<usize>() {
                    return Err(anyhow::anyhow!(
                        "Invalid max-depth value: {}. Must be a positive integer",
                        next_arg
                    ));
                }
            }
        }
    }
    
    Ok(())
}

/// Validate doctor command arguments
fn validate_doctor_command(args: &CommandArgs) -> Result<(), anyhow::Error> {
    // Similar validation as sync for max-depth
    for (i, arg) in args.raw_args.iter().enumerate() {
        if arg == "--max-depth" || arg == "-d" {
            if let Some(next_arg) = args.raw_args.get(i + 1) {
                if let Err(_) = next_arg.parse::<usize>() {
                    return Err(anyhow::anyhow!(
                        "Invalid max-depth value: {}. Must be a positive integer",
                        next_arg
                    ));
                }
            }
        }
    }
    
    Ok(())
}

/// Validate fork command arguments
fn validate_fork_command(args: &CommandArgs) -> Result<(), anyhow::Error> {
    // Fork command requires path and name arguments
    // Check that we have at least 2 positional arguments
    let positional_args: Vec<&String> = args.raw_args.iter()
        .filter(|arg| !arg.starts_with('-'))
        .collect();
    
    if positional_args.len() < 2 {
        return Err(anyhow::anyhow!(
            "Fork command requires path and name arguments"
        ));
    }
    
    Ok(())
}

/// Validate gitlab command arguments
fn validate_gitlab_command(args: &CommandArgs) -> Result<(), anyhow::Error> {
    // GitLab command requires a subcommand
    if args.raw_args.is_empty() {
        return Err(anyhow::anyhow!(
            "GitLab command requires a subcommand: login or clone"
        ));
    }
    
    let subcommand = &args.raw_args[0];
    if !["login", "clone"].contains(&subcommand.as_str()) {
        return Err(anyhow::anyhow!(
            "Invalid GitLab subcommand: {}. Valid values are: login, clone",
            subcommand
        ));
    }
    
    Ok(())
}

/// Validate snap command arguments
fn validate_snap_command(args: &CommandArgs) -> Result<(), anyhow::Error> {
    // Snap command requires a subcommand
    if args.raw_args.is_empty() {
        return Err(anyhow::anyhow!(
            "Snap command requires a subcommand: create, list, or restore"
        ));
    }
    
    let subcommand = &args.raw_args[0];
    if !["create", "list", "restore"].contains(&subcommand.as_str()) {
        return Err(anyhow::anyhow!(
            "Invalid snap subcommand: {}. Valid values are: create, list, restore",
            subcommand
        ));
    }
    
    Ok(())
}

/// Validate status command arguments
fn validate_status_command(args: &CommandArgs) -> Result<(), anyhow::Error> {
    // Similar validation as sync for max-depth
    for (i, arg) in args.raw_args.iter().enumerate() {
        if arg == "--max-depth" || arg == "-d" {
            if let Some(next_arg) = args.raw_args.get(i + 1) {
                if let Err(_) = next_arg.parse::<usize>() {
                    return Err(anyhow::anyhow!(
                        "Invalid max-depth value: {}. Must be a positive integer",
                        next_arg
                    ));
                }
            }
        }
    }
    
    Ok(())
}

/// Validate branch command arguments
fn validate_branch_command(args: &CommandArgs) -> Result<(), anyhow::Error> {
    // Branch command requires a subcommand
    if args.raw_args.is_empty() {
        return Err(anyhow::anyhow!(
            "Branch command requires a subcommand: list, clean, switch, or rename"
        ));
    }
    
    let subcommand = &args.raw_args[0];
    if !["list", "clean", "switch", "rename"].contains(&subcommand.as_str()) {
        return Err(anyhow::anyhow!(
            "Invalid branch subcommand: {}. Valid values are: list, clean, switch, rename",
            subcommand
        ));
    }
    
    Ok(())
}

/// Validate selfman command arguments
fn validate_selfman_command(args: &CommandArgs) -> Result<(), anyhow::Error> {
    // Self command requires a subcommand
    if args.raw_args.is_empty() {
        return Err(anyhow::anyhow!(
            "Self command requires a subcommand: update or version"
        ));
    }
    
    let subcommand = &args.raw_args[0];
    if !["update", "version"].contains(&subcommand.as_str()) {
        return Err(anyhow::anyhow!(
            "Invalid self subcommand: {}. Valid values are: update, version",
            subcommand
        ));
    }
    
    Ok(())
}

/// Validate config command arguments
fn validate_config_command(args: &CommandArgs) -> Result<(), anyhow::Error> {
    // Config command requires a subcommand
    if args.raw_args.is_empty() {
        return Err(anyhow::anyhow!(
            "Config command requires a subcommand: init, show, or path"
        ));
    }
    
    let subcommand = &args.raw_args[0];
    if !["init", "show", "path"].contains(&subcommand.as_str()) {
        return Err(anyhow::anyhow!(
            "Invalid config subcommand: {}. Valid values are: init, show, path",
            subcommand
        ));
    }
    
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_release_command() {
        // Test that release command parsing works
        // Note: This test is limited because we can't easily mock std::env::args
        // In a real test, we would use a test harness or mock the environment
        // For now, we'll test the validation functions
        
        let valid_args = CommandArgs {
            raw_args: vec!["patch".to_string(), "file1.txt".to_string()],
        };
        
        assert!(validate_release_command(&valid_args).is_ok());
        
        let invalid_args = CommandArgs {
            raw_args: vec!["invalid".to_string()],
        };
        
        assert!(validate_release_command(&invalid_args).is_err());
    }
    
    #[test]
    fn test_validate_sync_command() {
        let valid_args = CommandArgs {
            raw_args: vec!["--max-depth".to_string(), "3".to_string(), ".".to_string()],
        };
        
        assert!(validate_sync_command(&valid_args).is_ok());
        
        let invalid_args = CommandArgs {
            raw_args: vec!["--max-depth".to_string(), "invalid".to_string()],
        };
        
        assert!(validate_sync_command(&invalid_args).is_err());
    }
    
    #[test]
    fn test_validate_fork_command() {
        let valid_args = CommandArgs {
            raw_args: vec!["/some/path".to_string(), "project-name".to_string()],
        };
        
        assert!(validate_fork_command(&valid_args).is_ok());
        
        let invalid_args = CommandArgs {
            raw_args: vec!["/some/path".to_string()], // Missing name
        };
        
        assert!(validate_fork_command(&invalid_args).is_err());
    }
    
    #[test]
    fn test_validate_gitlab_command() {
        let valid_args = CommandArgs {
            raw_args: vec!["login".to_string()],
        };
        
        assert!(validate_gitlab_command(&valid_args).is_ok());
        
        let invalid_args = CommandArgs {
            raw_args: vec!["invalid".to_string()],
        };
        
        assert!(validate_gitlab_command(&invalid_args).is_err());
        
        let empty_args = CommandArgs {
            raw_args: vec![],
        };
        
        assert!(validate_gitlab_command(&empty_args).is_err());
    }
    
    #[test]
    fn test_command_name_conversion() {
        // Test that we can create CommandName from clap Commands
        // This is more of a compile-time test
        let release_cmd = crate::cli_old::Commands::Release {
            bump_type: crate::cli_old::BumpType::Patch,
            files: vec![],
            no_root: false,
            force: false,
            skip_push: false,
            dry_run: false,
            message: None,
            pre_release: None,
        };
        
        // This would be tested in the parse() method
        // For now, just verify the code compiles
    }
    
    #[test]
    fn test_clap_parser_implements_trait() {
        // Test that ClapParser implements CliParser trait
        let parser = ClapParser;
        // This is a compile-time test - if it compiles, the trait is implemented
        let _: &dyn CliParser = &parser;
    }
    
    #[test]
    fn test_parsed_command_structure() {
        // Test that ParsedCommand can be created
        let command = ParsedCommand {
            name: CommandName::Release,
            args: CommandArgs {
                raw_args: vec!["patch".to_string()],
            },
        };
        
        assert_eq!(command.name, CommandName::Release);
        assert_eq!(command.args.raw_args.len(), 1);
        assert_eq!(command.args.raw_args[0], "patch");
    }
}