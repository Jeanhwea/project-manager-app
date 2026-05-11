use super::GitCommandRunner;
use crate::domain::editor::{BumpType, Version};
use crate::error::AppError;
use crate::model::git::GitContext;
use crate::utils::output::Output;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ReleaseGitState {
    pub current_branch: String,
    pub new_tag: String,
    pub commit_message: String,
}

pub fn resolve_git_root() -> crate::error::Result<PathBuf> {
    let runner = GitCommandRunner::new();
    let root = runner.execute(&["rev-parse", "--show-toplevel"], None)?;
    if root.is_empty() {
        return Err(AppError::release("无法确定 git 根目录"));
    }
    Ok(PathBuf::from(root))
}

pub fn validate_git_state(
    repo_path: &Path,
    force: bool,
    bump_type: &BumpType,
    pre_release: &Option<String>,
    message: &Option<String>,
    ctx: &GitContext,
) -> crate::error::Result<ReleaseGitState> {
    if !force && ctx.current_branch != "master" {
        return Err(AppError::release("只能在 master 分支上执行 release"));
    }

    let runner = GitCommandRunner::new();
    let previous_tag = runner
        .execute(&["describe", "--tags", "--match", "v*"], Some(repo_path))
        .ok()
        .and_then(|o| o.split('-').next().map(|s| s.to_string()));
    let current_tag = previous_tag.clone().unwrap_or_else(|| "v0.0.0".to_string());

    if let Some(ref tag) = previous_tag {
        let rev_current_tag = runner.execute(&["rev-parse", tag], Some(repo_path))?;
        let rev_head = runner.execute(&["rev-parse", "HEAD"], Some(repo_path))?;
        if rev_current_tag.trim() == rev_head.trim() {
            return Err(AppError::release(format!("当前 HEAD 已被标记为 {}", tag)));
        }
    }

    let version = Version::from_tag(&current_tag).unwrap_or_default();
    let new_version = version.bump(bump_type);
    let mut new_tag = new_version.to_tag();

    if let Some(pre) = pre_release {
        new_tag = format!("{}-{}", new_tag, pre);
    }

    let commit_message = match message {
        Some(msg) => format!("{} {}", new_tag, msg),
        None => new_tag.clone(),
    };

    Output::item(&format!("版本变更: {} ->", current_tag), &new_tag);

    if message.is_some() {
        Output::item("提交消息", &commit_message);
    }

    Ok(ReleaseGitState {
        current_branch: ctx.current_branch.clone(),
        new_tag,
        commit_message,
    })
}

pub fn is_gitignored(file_path: &Path) -> bool {
    let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let Some(parent) = file_path.parent() else {
        return false;
    };

    let runner = GitCommandRunner::new();
    let output = runner.execute_raw(&["check-ignore", file_name], parent);

    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}
