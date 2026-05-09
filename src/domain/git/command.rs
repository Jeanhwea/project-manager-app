use super::{GitError, Result};
use crate::domain::runner::{CommandResult, CommandRunner, ExecutionContext, OutputMode};
use crate::utils::output::Output;
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

    pub fn execute(&self, args: &[&str]) -> Result<String> {
        let ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .output_mode(OutputMode::Capture);

        let result = self.run(&ctx)?;

        if !result.success {
            let stderr = result.stderr.unwrap_or_default();
            return Err(GitError::CommandFailed(stderr));
        }

        Ok(result.stdout.unwrap_or_default().trim().to_string())
    }

    pub fn execute_in_dir(&self, args: &[&str], dir: &Path) -> Result<String> {
        let ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .working_dir(dir)
            .output_mode(OutputMode::Capture);

        let result = self.run(&ctx)?;

        if !result.success {
            let stderr = result.stderr.unwrap_or_default();
            return Err(GitError::CommandFailed(stderr));
        }

        Ok(result.stdout.unwrap_or_default().trim().to_string())
    }

    pub fn execute_raw_in_dir(&self, args: &[&str], dir: &Path) -> Result<ProcessOutput> {
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

    pub fn execute_with_success(&self, args: &[&str]) -> Result<()> {
        self.check_success(args, None)
    }

    pub fn execute_with_success_in_dir(&self, args: &[&str], dir: &Path) -> Result<()> {
        self.check_success(args, Some(dir))
    }

    pub fn execute_quiet_in_dir(&self, args: &[&str], dir: &Path) -> Result<ProcessOutput> {
        self.execute_raw_in_dir(args, dir)
    }

    pub fn execute_streaming_in_dir(&self, args: &[&str], dir: &Path) -> Result<()> {
        let cmd_str = format!("git {}", args.join(" "));
        Output::cmd(&cmd_str);

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

    fn check_success(&self, args: &[&str], dir: Option<&Path>) -> Result<()> {
        let cmd_str = format!("git {}", args.join(" "));
        Output::cmd(&cmd_str);

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

    pub fn get_current_branch(&self, repo_path: &Path) -> Result<String> {
        self.execute_in_dir(&["branch", "--show-current"], repo_path)
    }

    #[allow(dead_code)]
    pub fn get_remote_urls(&self, repo_path: &Path) -> Result<Vec<String>> {
        let output = self.execute_in_dir(&["remote", "-v"], repo_path)?;
        let urls: Vec<String> = output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    Some(parts[1].to_string())
                } else {
                    None
                }
            })
            .collect();
        Ok(urls)
    }

    pub fn get_remote_list(&self, repo_path: &Path) -> Result<Vec<String>> {
        let output = self.execute_in_dir(&["remote"], repo_path)?;
        let remotes: Vec<String> = output
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|remote| !remote.is_empty())
            .collect();
        Ok(remotes)
    }

    pub fn has_uncommitted_changes(&self, repo_path: &Path) -> Result<bool> {
        let output = self.execute_in_dir(&["status", "--porcelain"], repo_path)?;
        Ok(!output.is_empty())
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
    fn test_execute_in_nonexistent_dir() {
        let runner = GitCommandRunner::new();
        let dir = tempdir().unwrap().path().join("nonexistent");
        assert!(runner.execute_in_dir(&["status"], &dir).is_err());
    }
}
