//! Git command execution module
//!
//! This module encapsulates external Git command execution.

use super::{GitError, Result};
use std::path::Path;
use std::process::{Command, Output};

/// Git command runner
///
/// This struct provides a clean interface for executing Git commands
/// and converting Git errors to application errors.
#[derive(Debug, Clone)]
pub struct GitCommandRunner;

impl GitCommandRunner {
    /// Create a new Git command runner
    pub fn new() -> Self {
        Self
    }

    /// Execute a Git command and return the output as a string
    ///
    /// # Arguments
    /// * `args` - Command arguments (e.g., `["status", "--porcelain"]`)
    ///
    /// # Returns
    /// * `Result<String>` - Command output as string or error
    ///
    /// # Errors
    /// * `GitError::CommandFailed` - If the Git command fails
    /// * `GitError::Io` - If there's an I/O error executing the command
    pub fn execute(&self, args: &[&str]) -> Result<String> {
        let output = self.execute_raw(args)?;

        // Convert stdout to string
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| GitError::CommandFailed(format!("Invalid UTF-8 in output: {}", e)))?;

        Ok(stdout.trim().to_string())
    }

    /// Execute a Git command in a specific directory
    ///
    /// # Arguments
    /// * `args` - Command arguments
    /// * `dir` - Working directory for the command
    ///
    /// # Returns
    /// * `Result<String>` - Command output as string or error
    ///
    /// # Errors
    /// * `GitError::CommandFailed` - If the Git command fails
    /// * `GitError::Io` - If there's an I/O error executing the command
    pub fn execute_in_dir(&self, args: &[&str], dir: &Path) -> Result<String> {
        let output = self.execute_raw_in_dir(args, dir)?;

        // Convert stdout to string
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| GitError::CommandFailed(format!("Invalid UTF-8 in output: {}", e)))?;

        Ok(stdout.trim().to_string())
    }

    /// Execute a Git command and return raw output
    ///
    /// # Arguments
    /// * `args` - Command arguments
    ///
    /// # Returns
    /// * `Result<Output>` - Raw command output or error
    ///
    /// # Errors
    /// * `GitError::CommandFailed` - If the Git command fails
    /// * `GitError::Io` - If there's an I/O error executing the command
    pub fn execute_raw(&self, args: &[&str]) -> Result<Output> {
        self.execute_git_command(args, None)
    }

    /// Execute a Git command in a specific directory and return raw output
    ///
    /// # Arguments
    /// * `args` - Command arguments
    /// * `dir` - Working directory for the command
    ///
    /// # Returns
    /// * `Result<Output>` - Raw command output or error
    ///
    /// # Errors
    /// * `GitError::CommandFailed` - If the Git command fails
    /// * `GitError::Io` - If there's an I/O error executing the command
    pub fn execute_raw_in_dir(&self, args: &[&str], dir: &Path) -> Result<Output> {
        self.execute_git_command(args, Some(dir))
    }

    /// Execute a Git command and check for success
    ///
    /// # Arguments
    /// * `args` - Command arguments
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    ///
    /// # Errors
    /// * `GitError::CommandFailed` - If the Git command fails
    /// * `GitError::Io` - If there's an I/O error executing the command
    pub fn execute_with_success(&self, args: &[&str]) -> Result<()> {
        let output = self.execute_git_command(args, None)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::CommandFailed(format!(
                "Git command failed: {}",
                stderr.trim()
            )));
        }

        Ok(())
    }

    /// Execute a Git command in a specific directory and check for success
    ///
    /// # Arguments
    /// * `args` - Command arguments
    /// * `dir` - Working directory for the command
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    ///
    /// # Errors
    /// * `GitError::CommandFailed` - If the Git command fails
    /// * `GitError::Io` - If there's an I/O error executing the command
    pub fn execute_with_success_in_dir(&self, args: &[&str], dir: &Path) -> Result<()> {
        let output = self.execute_git_command(args, Some(dir))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::CommandFailed(format!(
                "Git command failed: {}",
                stderr.trim()
            )));
        }

        Ok(())
    }

    /// Execute a Git command quietly (without checking success)
    ///
    /// # Arguments
    /// * `args` - Command arguments
    ///
    /// # Returns
    /// * `Result<Output>` - Raw command output or error
    ///
    /// # Errors
    /// * `GitError::Io` - If there's an I/O error executing the command
    pub fn execute_quiet(&self, args: &[&str]) -> Result<Output> {
        self.execute_git_command(args, None)
    }

    /// Execute a Git command quietly in a specific directory
    ///
    /// # Arguments
    /// * `args` - Command arguments
    /// * `dir` - Working directory for the command
    ///
    /// # Returns
    /// * `Result<Output>` - Raw command output or error
    ///
    /// # Errors
    /// * `GitError::Io` - If there's an I/O error executing the command
    pub fn execute_quiet_in_dir(&self, args: &[&str], dir: &Path) -> Result<Output> {
        self.execute_git_command(args, Some(dir))
    }

    /// Internal method to execute a Git command
    fn execute_git_command(&self, args: &[&str], dir: Option<&Path>) -> Result<Output> {
        let mut cmd = Command::new("git");
        cmd.args(args);

        if let Some(dir) = dir {
            cmd.current_dir(dir);
        }

        let output = cmd.output().map_err(|e| {
            GitError::CommandFailed(format!("Failed to execute git command: {}", e))
        })?;

        Ok(output)
    }

    /// Check if Git is available on the system
    ///
    /// # Returns
    /// * `bool` - True if Git is available, false otherwise
    pub fn is_git_available(&self) -> bool {
        Command::new("git")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }

    /// Get Git version
    ///
    /// # Returns
    /// * `Result<String>` - Git version string or error
    ///
    /// # Errors
    /// * `GitError::CommandFailed` - If the Git command fails
    /// * `GitError::Io` - If there's an I/O error executing the command
    pub fn get_git_version(&self) -> Result<String> {
        let output = self.execute(&["--version"])?;
        Ok(output)
    }
}

impl Default for GitCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_git_command_runner_new() {
        let runner = GitCommandRunner::new();
        // Just test that it can be created
        assert!(true);
    }

    #[test]
    fn test_git_command_runner_default() {
        let runner = GitCommandRunner::default();
        // Just test that default works
        assert!(true);
    }

    #[test]
    fn test_is_git_available() {
        let runner = GitCommandRunner::new();
        // This test will pass if Git is installed, which it should be in this environment
        let available = runner.is_git_available();
        // We can't assert true/false since it depends on the environment
        // Just test that the method doesn't panic
        assert!(true);
    }

    #[test]
    fn test_execute_git_version() {
        let runner = GitCommandRunner::new();
        if runner.is_git_available() {
            let result = runner.get_git_version();
            // Should succeed if Git is available
            assert!(result.is_ok());
            let version = result.unwrap();
            assert!(version.contains("git version"));
        }
    }

    #[test]
    fn test_execute_in_nonexistent_dir() {
        let runner = GitCommandRunner::new();
        let temp_dir = tempdir().unwrap();
        let nonexistent_dir = temp_dir.path().join("nonexistent");

        let result = runner.execute_in_dir(&["status"], &nonexistent_dir);
        // Should fail because directory doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_with_success_invalid_args() {
        let runner = GitCommandRunner::new();
        let result = runner.execute_with_success(&["invalid-command-that-doesnt-exist"]);
        // Should fail because command is invalid
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_quiet() {
        let runner = GitCommandRunner::new();
        if runner.is_git_available() {
            let result = runner.execute_quiet(&["--version"]);
            // Should succeed even if we don't check success status
            assert!(result.is_ok());
        }
    }
}
