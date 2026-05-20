use super::GitCommandRunner;
use crate::error::{AppError, Result as AppResult};
use std::path::Path;

pub fn head_commit_count(repo: &Path) -> AppResult<usize> {
    let runner = GitCommandRunner::new();
    let s = runner.run_local(&["rev-list", "--count", "HEAD"], Some(repo))?;
    s.trim().parse::<usize>().map_err(AppError::from)
}

pub fn list_snapshot_oneline(repo: &Path) -> AppResult<Vec<String>> {
    let runner = GitCommandRunner::new();
    let out = runner.run_local(&["log", "--oneline"], Some(repo))?;
    Ok(out
        .lines()
        .filter(|line| line.contains("snap-"))
        .map(|s| s.to_string())
        .collect())
}

pub fn search_oneline(repo: &Path, grep: &str) -> AppResult<Vec<String>> {
    let runner = GitCommandRunner::new();
    let out = runner.run_local(&["log", "--oneline", "--grep", grep], Some(repo))?;
    Ok(out.lines().map(|s| s.to_string()).collect())
}

pub fn rev_parse_verify(repo: &Path, refname: &str) -> AppResult<String> {
    let runner = GitCommandRunner::new();
    runner
        .run_local(&["rev-parse", "--verify", refname], Some(repo))
        .map_err(AppError::from)
}
