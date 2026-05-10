use super::models::{Branch, Remote, Tag};
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

    pub fn get_remote(&self, name: &str, repo_path: &Path) -> Result<Remote> {
        let url = self.execute(&["remote", "get-url", name], Some(repo_path))?;
        let fetch_url = self
            .execute(&["remote", "get-url", "--push", name], Some(repo_path))
            .ok();
        let fetch_url = fetch_url.filter(|u| *u != url);

        Ok(Remote {
            name: name.to_string(),
            url,
            fetch_url,
        })
    }

    pub fn get_all_remotes(&self, repo_path: &Path) -> Result<Vec<Remote>> {
        let names = self.get_remote_list(repo_path)?;
        let mut remotes = Vec::new();
        for name in &names {
            if let Ok(remote) = self.get_remote(name, repo_path) {
                remotes.push(remote);
            }
        }
        Ok(remotes)
    }

    pub fn get_all_branches(&self, repo_path: &Path) -> Result<Vec<Branch>> {
        let output = self.execute(&["branch", "-vv", "--all"], Some(repo_path))?;
        let mut branches = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let is_current = line.starts_with("* ");
            let line = line.trim_start_matches("* ").trim_start_matches("  ");

            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            let name = parts.first().unwrap_or(&line).to_string();
            let name = name.trim_start_matches("remotes/").to_string();

            let is_remote = line.contains("remotes/");
            let tracking_branch = Self::extract_tracking_branch(parts.get(1).unwrap_or(&""));
            let ahead_behind = Self::extract_ahead_behind(parts.get(1).unwrap_or(&""));

            branches.push(Branch {
                name,
                is_current: is_current && !is_remote,
                is_remote,
                tracking_branch,
                ahead_behind,
            });
        }

        Ok(branches)
    }

    pub fn get_all_tags(&self, repo_path: &Path) -> Result<Vec<Tag>> {
        let output = self.execute(
            &[
                "for-each-ref",
                "--format=%(refname:short) %(objectname:short) %(objecttype)",
                "refs/tags",
            ],
            Some(repo_path),
        )?;

        let mut tags = Vec::new();
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                tags.push(Tag {
                    name: parts[0].to_string(),
                    commit: parts[1].to_string(),
                    is_annotated: parts.get(2).map(|t| t == &"tag").unwrap_or(false),
                    message: None,
                });
            }
        }

        Ok(tags)
    }

    pub fn has_uncommitted_changes(&self, repo_path: &Path) -> Result<bool> {
        let output = self.execute(&["status", "--porcelain"], Some(repo_path))?;
        Ok(!output.is_empty())
    }

    fn extract_tracking_branch(info: &str) -> Option<String> {
        if let Some(start) = info.find('[')
            && let Some(end) = info.find(']')
        {
            let inner = &info[start + 1..end];
            if inner.contains(":") {
                Some(inner.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn extract_ahead_behind(info: &str) -> Option<(usize, usize)> {
        if let Some(start) = info.find('[')
            && let Some(end) = info.find(']')
        {
            let inner = &info[start + 1..end];
            if let Some(ahead) = inner.strip_prefix("ahead ") {
                if let Some(space) = ahead.find(' ')
                    && let Some(behind) = ahead[space + 1..].strip_prefix("behind ")
                    && let (Ok(a), Ok(b)) = (ahead[..space].parse(), behind.parse())
                {
                    return Some((a, b));
                }
            } else if let Some(behind) = inner.strip_prefix("behind ")
                && let Ok(b) = behind.parse::<usize>()
            {
                return Some((0, b));
            } else if let Some(ahead) = inner.strip_prefix("ahead ")
                && let Ok(a) = ahead.parse::<usize>()
            {
                return Some((a, 0));
            }
        }
        None
    }
}

impl Default for GitCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}
