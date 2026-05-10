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

    pub fn execute_with_success(&self, args: &[&str], dir: Option<&Path>) -> Result<()> {
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

    pub fn execute_streaming(&self, args: &[&str], dir: &Path) -> Result<()> {
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

    pub fn get_current_branch(&self, repo_path: &Path) -> Result<String> {
        self.execute(&["branch", "--show-current"], Some(repo_path))
    }

    pub fn get_remote_list(&self, repo_path: &Path) -> Result<Vec<String>> {
        let output = self.execute(&["remote"], Some(repo_path))?;
        let remotes: Vec<String> = output
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|remote| !remote.is_empty())
            .collect();
        Ok(remotes)
    }

    pub fn has_uncommitted_changes(&self, repo_path: &Path) -> Result<bool> {
        let output = self.execute(&["status", "--porcelain"], Some(repo_path))?;
        Ok(!output.is_empty())
    }

    pub fn list_remotes(&self, repo_path: &Path) -> Result<Vec<super::remote::Remote>> {
        let remote_names_result = self.execute(&["remote"], Some(repo_path));

        let remote_names: Vec<String> = match remote_names_result {
            Ok(output) => output
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            Err(_) => return Ok(Vec::new()),
        };

        let mut remotes = Vec::new();
        for name in remote_names {
            if let Ok(url) = self.get_remote_url(repo_path, &name) {
                remotes.push(super::remote::Remote {
                    name: name.to_string(),
                    url,
                });
            }
        }

        Ok(remotes)
    }

    fn get_remote_url(&self, repo_path: &Path, name: &str) -> Result<String> {
        let output = self.execute(&["remote", "get-url", name], Some(repo_path))?;

        if output.trim().is_empty() {
            Err(GitError::RemoteNotFound(name.to_string()))
        } else {
            Ok(output)
        }
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
        assert!(runner.execute(&["status"], Some(&dir)).is_err());
    }
}
