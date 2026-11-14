//! 查询远端真实存在的分支表：用 `git ls-remote --heads` 拿到的集合与本地 tracking ref 交叉比对，
//! 供 sync / branch clean 判断某分支是否已在远端被清理。查询失败返回 None 由调用方降级处理。

use super::GitCommandRunner;
use std::collections::HashSet;
use std::path::Path;

/// 查询远端真实存在的分支名；远端不可达或命令失败时返回 None
pub fn list_remote_heads(
    runner: &GitCommandRunner,
    repo_path: &Path,
    remote: &str,
) -> Option<HashSet<String>> {
    let output = runner
        .run_local(&["ls-remote", "--heads", remote], Some(repo_path))
        .ok()?;
    Some(output.lines().filter_map(parse_ls_remote_line).collect())
}

/// 解析 git ls-remote 的一行输出 `<sha>\trefs/heads/<branch>`，提取分支名
pub fn parse_ls_remote_line(line: &str) -> Option<String> {
    const PREFIX: &str = "refs/heads/";
    let pos = line.find(PREFIX)?;
    let name = line[pos + PREFIX.len()..].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_branch_name_from_ls_remote_line() {
        assert_eq!(
            parse_ls_remote_line("a1b2c3d\trefs/heads/feat/shen-POC"),
            Some("feat/shen-POC".to_string())
        );
        assert_eq!(
            parse_ls_remote_line("a1b2c3d\trefs/heads/fix/M-lang"),
            Some("fix/M-lang".to_string())
        );
        assert_eq!(parse_ls_remote_line("a1b2c3d\trefs/tags/v1.0.0"), None);
    }

    #[test]
    fn skips_empty_or_malformed_lines() {
        assert_eq!(parse_ls_remote_line(""), None);
        assert_eq!(parse_ls_remote_line("a1b2c3d\trefs/heads/"), None);
        assert_eq!(parse_ls_remote_line("a1b2c3d\trefs/heads/   "), None);
    }
}
