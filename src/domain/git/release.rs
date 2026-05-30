use super::GitCommandRunner;
use crate::domain::editor::{BumpType, Version};
use crate::model::git::GitContext;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ReleaseError {
    #[error("无法确定 git 根目录")]
    GitRootNotFound,

    #[error("只能在 master 分支上执行 release")]
    NotOnMaster,

    #[error("当前 HEAD 已被标记为 {tag}")]
    HeadAlreadyTagged { tag: String },

    #[error("未检测到可编辑的配置文件")]
    NoConfigFiles,

    #[error("无法识别文件类型: {path}")]
    UnknownFileType { path: String },

    #[error("无法读取 {path}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("{path}: 未找到版本字段")]
    VersionFieldNotFound { path: String },

    #[error("未在 {path} 中找到 [package] name")]
    PackageNameNotFound { path: String },
}

#[derive(Debug)]
pub struct ReleaseGitState {
    pub current_tag: String,
    pub current_branch: String,
    pub new_tag: String,
    pub commit_message: String,
    pub used_fallback: bool,
}

pub fn resolve_git_root() -> crate::error::Result<PathBuf> {
    let runner = GitCommandRunner::new();
    let root = runner.run_local(&["rev-parse", "--show-toplevel"], None)?;
    if root.is_empty() {
        return Err(ReleaseError::GitRootNotFound.into());
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
    fallback_version: Option<&str>,
) -> crate::error::Result<ReleaseGitState> {
    if !force && ctx.current_branch != "master" {
        return Err(ReleaseError::NotOnMaster.into());
    }

    let runner = GitCommandRunner::new();
    let previous_tag = runner
        .run_local(&["describe", "--tags", "--match", "v*"], Some(repo_path))
        .ok()
        .and_then(|o| o.split('-').next().map(|s| s.to_string()))
        .or_else(|| find_max_semver_tag(&runner, repo_path));

    let (current_tag, used_fallback) = if let Some(ref tag) = previous_tag {
        (tag.clone(), false)
    } else if let Some(fallback) = fallback_version {
        let fb = if fallback.starts_with('v') {
            fallback.to_string()
        } else {
            format!("v{}", fallback)
        };
        (fb, true)
    } else {
        ("v0.0.0".to_string(), false)
    };

    if let Some(ref tag) = previous_tag {
        let rev_current_tag = runner.run_local(&["rev-parse", tag], Some(repo_path))?;
        let rev_head = runner.run_local(&["rev-parse", "HEAD"], Some(repo_path))?;
        if rev_current_tag.trim() == rev_head.trim() {
            return Err(ReleaseError::HeadAlreadyTagged { tag: tag.clone() }.into());
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

    Ok(ReleaseGitState {
        current_tag,
        current_branch: ctx.current_branch.clone(),
        new_tag,
        commit_message,
        used_fallback,
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
    runner
        .run_local(&["check-ignore", file_name], Some(parent))
        .is_ok()
}

fn find_max_semver_tag(runner: &GitCommandRunner, repo_path: &Path) -> Option<String> {
    let output = runner
        .run_local(
            &["tag", "--list", "v*", "--format=%(refname:short)"],
            Some(repo_path),
        )
        .ok()?;

    let mut best: Option<(Version, String)> = None;
    for line in output.lines() {
        let tag = line.trim();
        if tag.is_empty() {
            continue;
        }
        let Some(ver) = Version::from_tag(tag) else {
            continue;
        };
        if best.as_ref().is_none_or(|(b, _)| ver > *b) {
            best = Some((ver, tag.to_string()));
        }
    }

    best.map(|(_, tag)| tag)
}
