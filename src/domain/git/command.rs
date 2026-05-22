use super::{GitError, Result};
use crate::domain::runner::{CommandRunner, ExecutionContext, OutputMode};
use std::path::Path;

pub struct GitCommandRunner;

impl GitCommandRunner {
    pub fn new() -> Self {
        Self
    }

    pub fn run_local(&self, args: &[&str], dir: Option<&Path>) -> Result<String> {
        let mut ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .output_mode(OutputMode::Capture);

        if let Some(d) = dir {
            ctx = ctx.working_dir(d);
        }

        let result = CommandRunner
            .execute(&ctx)
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;

        if !result.success {
            let stderr = result.stderr.unwrap_or_default().trim().to_string();
            return Err(GitError::CommandFailed(stderr));
        }
        Ok(result.stdout.unwrap_or_default().trim().to_string())
    }

    pub fn run_streaming(&self, args: &[&str], dir: &Path) -> Result<()> {
        let ctx = ExecutionContext::new("git")
            .args(args.iter().copied())
            .working_dir(dir)
            .output_mode(OutputMode::Streaming);

        let result = CommandRunner
            .execute(&ctx)
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;

        if !result.success {
            return Err(GitError::CommandFailed(format!(
                "Git command exited with code {}",
                result.exit_code
            )));
        }
        Ok(())
    }

    pub fn current_branch(&self, repo_path: &Path) -> Result<String> {
        self.run_local(&["branch", "--show-current"], Some(repo_path))
    }

    pub fn remote_names(&self, repo_path: &Path) -> Result<Vec<String>> {
        let output = self.run_local(&["remote"], Some(repo_path))?;
        Ok(output
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }

    pub fn has_uncommitted_changes(&self, repo_path: &Path) -> Result<bool> {
        let output = self.run_local(&["status", "--porcelain"], Some(repo_path))?;
        Ok(!output.is_empty())
    }

    pub fn is_merged_branch(&self, name: &str, repo_path: &Path) -> bool {
        self.run_local(&["branch", "--merged", "master"], Some(repo_path))
            .map(|output| {
                output
                    .lines()
                    .any(|line| line.trim_start_matches("* ").trim() == name)
            })
            .unwrap_or(false)
    }
}

impl Default for GitCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}
