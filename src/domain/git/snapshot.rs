//! snap 子命令依赖的 git 查询：提交计数与快照提交检索。

use super::GitCommandRunner;
use crate::error::{AppError, Result as AppResult};
use std::path::Path;

/// HEAD 可达的提交总数
pub fn head_commit_count(runner: &GitCommandRunner, repo: &Path) -> AppResult<usize> {
    let s = runner.run_local(&["rev-list", "--count", "HEAD"], Some(repo))?;
    s.trim().parse::<usize>().map_err(AppError::from)
}

/// 列出历史中的快照提交（commit message 里含 `snap-` 的记录）
pub fn list_snapshot_oneline(runner: &GitCommandRunner, repo: &Path) -> AppResult<Vec<String>> {
    let out = runner.run_local(&["log", "--oneline"], Some(repo))?;
    Ok(out
        .lines()
        .filter(|line| line.contains("snap-"))
        .map(|s| s.to_string())
        .collect())
}

/// 按 commit message 关键字检索提交
pub fn search_oneline(
    runner: &GitCommandRunner,
    repo: &Path,
    grep: &str,
) -> AppResult<Vec<String>> {
    let out = runner.run_local(&["log", "--oneline", "--grep", grep], Some(repo))?;
    Ok(out.lines().map(|s| s.to_string()).collect())
}

/// 校验 ref 是否存在并返回其 commit hash；ref 不存在时报错
pub fn rev_parse_verify(
    runner: &GitCommandRunner,
    repo: &Path,
    refname: &str,
) -> AppResult<String> {
    runner
        .run_local(&["rev-parse", "--verify", refname], Some(repo))
        .map_err(AppError::from)
}
