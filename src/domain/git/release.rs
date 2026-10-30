//! release 的 git 侧前置计算：确定基准 tag，推导出下一个版本号与提交信息。
//! 只做判定与计算，写文件、提交、打 tag 由命令层丢给 plan/execute。

use super::GitCommandRunner;
use crate::domain::editor::{BumpType, Version};
use crate::model::git::GitContext;
use std::path::{Path, PathBuf};

/// release 前置检查失败的原因
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

/// 一次 release 推导出的版本信息
#[derive(Debug)]
pub struct ReleaseGitState {
    pub current_tag: String,
    pub current_branch: String,
    pub new_tag: String,
    pub commit_message: String,
    /// true 表示 `current_tag` 取自命令行给的兜底版本号，而非仓库里真实存在的 tag
    pub used_fallback: bool,
}

/// 解析 git 仓库根目录；当前不在任何仓库内时报 [`ReleaseError::GitRootNotFound`]
pub fn resolve_git_root() -> crate::error::Result<PathBuf> {
    let runner = GitCommandRunner::new();
    let root = runner.run_local(&["rev-parse", "--show-toplevel"], None)?;
    if root.is_empty() {
        return Err(ReleaseError::GitRootNotFound.into());
    }
    Ok(PathBuf::from(root))
}

/// 校验发布前提并推导出新版本号：`force` 为 true 时放开 master 分支限制，
/// `fallback_version` 用于仓库还没有任何 tag 时指定起始版本
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
    // describe 给的是提交图上最近的 tag，max_tag 给的是版本号最大的 tag，
    // 二者未必一致（例如在 release 分支上给旧版本打补丁），取语义版本更大的那个
    let describe_tag = runner
        .run_local(&["describe", "--tags", "--match", "v*"], Some(repo_path))
        .ok()
        .and_then(|o| o.split('-').next().map(|s| s.to_string()));
    let max_tag = find_max_semver_tag(&runner, repo_path);

    let previous_tag = match (&describe_tag, &max_tag) {
        (Some(d), Some(m)) => {
            let dv = Version::from_tag(d);
            let mv = Version::from_tag(m);
            match (dv, mv) {
                (Some(dv), Some(mv)) => {
                    if mv > dv {
                        Some(m.clone())
                    } else {
                        Some(d.clone())
                    }
                }
                _ => Some(d.clone()),
            }
        }
        (Some(d), None) => Some(d.clone()),
        (None, Some(m)) => Some(m.clone()),
        (None, None) => None,
    };

    let (current_tag, used_fallback) = if let Some(ref tag) = previous_tag {
        (tag.clone(), false)
    } else if let Some(fallback) = fallback_version {
        // 兜底值可能写成 1.2.3 或 v1.2.3，统一补上 v 前缀
        let fb = if fallback.starts_with('v') {
            fallback.to_string()
        } else {
            format!("v{}", fallback)
        };
        (fb, true)
    } else {
        // 首次发布：既无历史 tag 也无兜底值，从 v0.0.0 起步
        ("v0.0.0".to_string(), false)
    };

    if let Some(ref tag) = previous_tag {
        let rev_current_tag = runner.run_local(&["rev-parse", tag], Some(repo_path))?;
        let rev_head = runner.run_local(&["rev-parse", "HEAD"], Some(repo_path))?;
        // HEAD 已经落在这个 tag 上说明没有新提交，重复打 tag 会被 git 拒绝
        if rev_current_tag.trim() == rev_head.trim() {
            return Err(ReleaseError::HeadAlreadyTagged { tag: tag.clone() }.into());
        }
    }

    // 基准 tag 可能带 prerelease 后缀（如 v1.2.3-beta），这里只取三段主版本号
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

/// 判断文件是否被 .gitignore 忽略；取不到文件名或父目录时保守返回 false
pub fn is_gitignored(file_path: &Path) -> bool {
    let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let Some(parent) = file_path.parent() else {
        return false;
    };

    // check-ignore 用退出码表达结果：0 表示命中忽略规则，非 0 表示未被忽略
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
