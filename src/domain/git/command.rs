use super::{GitError, Result};
use crate::domain::runner::{CommandResult, CommandRunner, ExecutionContext, OutputMode};
use std::path::Path;
use std::process::{ExitStatus, Output as ProcessOutput};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

pub struct GitCommandRunner;

impl GitCommandRunner {
    pub fn new() -> Self {
        Self
    }

    fn run(&self, context: &ExecutionContext) -> Result<CommandResult> {
        CommandRunner
            .execute(context)
            .map_err(|e| GitError::CommandFailed(e.to_string()))
    }

    pub fn execute(&self, args: &[&str], dir: Option<&Path>) -> Result<String> {
        let mut ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .output_mode(OutputMode::Capture);

        if let Some(dir) = dir {
            ctx = ctx.working_dir(dir);
        }

        let result = self.run(&ctx)?;

        if !result.success {
            let stderr = result.stderr.unwrap_or_default();
            return Err(GitError::CommandFailed(stderr));
        }

        Ok(result.stdout.unwrap_or_default().trim().to_string())
    }

    pub fn execute_with_success(&self, args: &[&str], dir: Option<&Path>) -> Result<()> {
        let mut ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .output_mode(OutputMode::Capture);

        if let Some(dir) = dir {
            ctx = ctx.working_dir(dir);
        }

        let result = self.run(&ctx)?;

        if !result.success {
            let stderr = result.stderr.unwrap_or_default();
            return Err(GitError::CommandFailed(format!(
                "Git command failed: {}",
                stderr.trim()
            )));
        }
        Ok(())
    }

    pub fn execute_streaming(&self, args: &[&str], dir: &Path) -> Result<()> {
        let ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .working_dir(dir)
            .output_mode(OutputMode::Streaming);

        let result = self.run(&ctx)?;

        if !result.success {
            return Err(GitError::CommandFailed(format!(
                "Git command exited with code {}",
                result.exit_code
            )));
        }

        Ok(())
    }

    pub fn execute_raw(&self, args: &[&str], dir: &Path) -> Result<ProcessOutput> {
        let ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .working_dir(dir)
            .output_mode(OutputMode::Capture);

        let result = self.run(&ctx)?;

        #[cfg(unix)]
        let status = ExitStatus::from_raw(result.exit_code);
        #[cfg(windows)]
        let status = ExitStatus::from_raw(result.exit_code as u32);

        let stdout = result.stdout.unwrap_or_default().into_bytes();
        let stderr = result.stderr.unwrap_or_default().into_bytes();

        Ok(ProcessOutput {
            status,
            stdout,
            stderr,
        })
    }

    pub fn get_current_branch(&self, repo_path: &Path) -> Result<String> {
        self.execute(&["branch", "--show-current"], Some(repo_path))
    }

    pub fn get_remote_list(&self, repo_path: &Path) -> Result<Vec<String>> {
        let output = self.execute(&["remote"], Some(repo_path))?;
        Ok(output
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }

    pub fn has_uncommitted_changes(&self, repo_path: &Path) -> Result<bool> {
        let output = self.execute(&["status", "--porcelain"], Some(repo_path))?;
        Ok(!output.is_empty())
    }
}

impl Default for GitCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}
