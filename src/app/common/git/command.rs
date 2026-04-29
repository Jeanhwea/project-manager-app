use crate::app::common::runner::CommandRunner;
use anyhow::{Context, Result};
use std::path::Path;
use std::path::PathBuf;

pub fn get_rev_revision(ref_name: &str) -> Result<String> {
    let output = CommandRunner::run_quiet("git", &["rev-parse", ref_name])
        .with_context(|| format!("无法获取 {} 的 revision", ref_name))?;

    let revision = String::from_utf8(output.stdout).with_context(|| "无法解析 revision 输出")?;
    let revision = revision.trim();

    if revision.is_empty() {
        anyhow::bail!("{} 的 revision 为空", ref_name);
    }

    Ok(revision.to_string())
}

pub fn get_current_version() -> Option<String> {
    let output = CommandRunner::run_quiet("git", &["tag", "-l", "v*"]).ok()?;

    let tags = String::from_utf8(output.stdout).ok()?;
    let mut tags: Vec<&str> = tags.lines().collect();

    if tags.is_empty() {
        return None;
    }

    tags.sort_by(|a, b| crate::app::common::version::compare_versions(a, b));

    tags.first().map(|s| s.to_string())
}

pub fn get_current_branch() -> Option<String> {
    let output = CommandRunner::run_quiet("git", &["branch", "--show-current"]).ok()?;

    let branch = String::from_utf8(output.stdout).ok()?;
    let branch = branch.trim();

    if branch.is_empty() {
        None
    } else {
        Some(branch.to_string())
    }
}

pub fn get_current_branch_in_dir(dir: &Path) -> Option<String> {
    let output = CommandRunner::run_quiet_in_dir("git", &["branch", "--show-current"], dir).ok()?;

    let branch = String::from_utf8(output.stdout).ok()?;
    let branch = branch.trim();

    if branch.is_empty() {
        None
    } else {
        Some(branch.to_string())
    }
}

pub fn is_workdir_clean(dir: &Path) -> bool {
    CommandRunner::run_quiet_in_dir("git", &["status", "--porcelain"], dir)
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .is_some_and(|s| s.trim().is_empty())
}

pub fn get_remote_list() -> Option<Vec<String>> {
    let output = CommandRunner::run_quiet("git", &["remote"]).ok()?;

    let remotes = String::from_utf8(output.stdout).ok()?;
    let remotes: Vec<String> = remotes
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if remotes.is_empty() {
        None
    } else {
        Some(remotes)
    }
}

pub fn get_top_level_dir() -> Option<PathBuf> {
    let output = CommandRunner::run_quiet("git", &["rev-parse", "--show-toplevel"]).ok()?;

    let top_level = String::from_utf8(output.stdout).ok()?;
    let top_level = top_level.trim();

    if top_level.is_empty() {
        None
    } else {
        Some(PathBuf::from(top_level))
    }
}

pub fn list_cached_changes() -> Result<()> {
    CommandRunner::run_with_success("git", &["diff", "--cached"])
        .with_context(|| "无法列出缓存的更改")
        .map(|_| ())
}

pub fn clone(repo_url: &str, name: &str) -> Result<()> {
    CommandRunner::run_with_success("git", &["clone", repo_url, name])
        .with_context(|| format!("无法克隆仓库 {} 到 {}", repo_url, name))
        .map(|_| ())
}

pub fn add_file(file: &str) -> Result<()> {
    CommandRunner::run_with_success("git", &["add", file])
        .with_context(|| format!("无法添加文件 {}", file))
        .map(|_| ())
}

pub fn commit(message: &str) -> Result<()> {
    CommandRunner::run_with_success("git", &["commit", "-m", message])
        .with_context(|| format!("无法提交: {}", message))
        .map(|_| ())
}

pub fn create_tag(tag: &str) -> Result<()> {
    CommandRunner::run_with_success("git", &["tag", tag])
        .with_context(|| format!("无法创建标签 {}", tag))
        .map(|_| ())
}

pub fn push_tag(remote: &str, tag: &str) -> Result<()> {
    CommandRunner::run_with_success("git", &["push", remote, tag])
        .with_context(|| format!("无法推送标签 {} 到 {}", tag, remote))
        .map(|_| ())
}

pub fn push_branch(remote: &str, branch: &str) -> Result<()> {
    CommandRunner::run_with_success("git", &["push", remote, branch])
        .with_context(|| format!("无法推送分支 {} 到 {}", branch, remote))
        .map(|_| ())
}

pub fn get_remote_name(work_dir: &Path) -> Vec<String> {
    let output = match CommandRunner::run_quiet_in_dir("git", &["remote"], work_dir) {
        Ok(out) => out,
        Err(_) => return Vec::new(),
    };

    let remotes = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    remotes
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn get_remote_info(work_dir: &Path) -> Vec<(String, String)> {
    let remotes = get_remote_name(work_dir);
    if remotes.is_empty() {
        return Vec::new();
    }

    let mut remote_info = Vec::new();
    for remote in remotes {
        let output = match CommandRunner::run_quiet_in_dir(
            "git",
            &["remote", "get-url", &remote],
            work_dir,
        ) {
            Ok(out) => out,
            Err(_) => continue,
        };

        let url = match String::from_utf8(output.stdout) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let url = url.trim();
        if url.is_empty() {
            continue;
        }

        remote_info.push((remote, url.to_string()));
    }

    remote_info
}

pub fn get_local_branches(dir: &Path) -> Vec<String> {
    let output = match CommandRunner::run_quiet_in_dir("git", &["branch", "--list"], dir) {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stdout
        .lines()
        .map(|line| line.trim_start_matches('*').trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn get_remote_branches(dir: &Path) -> Vec<String> {
    let output = match CommandRunner::run_quiet_in_dir("git", &["branch", "-r"], dir) {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|s| !s.is_empty() && !s.contains("->"))
        .collect()
}

pub fn get_merged_branches(dir: &Path, current_branch: &str) -> Vec<String> {
    let output = match CommandRunner::run_quiet_in_dir(
        "git",
        &["branch", "--merged", current_branch],
        dir,
    ) {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stdout
        .lines()
        .map(|line| line.trim_start_matches('*').trim().to_string())
        .filter(|s| !s.is_empty() && s != current_branch)
        .collect()
}

pub fn get_remote_merged_branches(dir: &Path, current_branch: &str) -> Vec<String> {
    let output = match CommandRunner::run_quiet_in_dir(
        "git",
        &["branch", "-r", "--merged", current_branch],
        dir,
    ) {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let current_remote = format!("origin/{}", current_branch);

    stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|s| {
            !s.is_empty() && !s.contains("->") && *s != current_remote && s.starts_with("origin/")
        })
        .map(|s| s.strip_prefix("origin/").unwrap_or(&s).to_string())
        .collect()
}

pub fn check_command_exists(cmd: &str) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", cmd, "--version"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new(cmd)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }
}

pub fn run_command(cmd: &str, args: &[&str], dir: &Path) -> Result<std::process::ExitStatus> {
    #[cfg(windows)]
    {
        let mut all_args = vec!["/C", cmd];
        all_args.extend(args.iter().copied());
        std::process::Command::new("cmd")
            .args(&all_args)
            .current_dir(dir)
            .status()
            .context(format!("无法执行 {} {}", cmd, args.join(" ")))
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new(cmd)
            .args(args)
            .current_dir(dir)
            .status()
            .context(format!("无法执行 {} {}", cmd, args.join(" ")))
    }
}
