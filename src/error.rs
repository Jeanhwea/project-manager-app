use crate::commands::gitlab::GitlabApiError;
use crate::commands::snap::SnapshotError;
use crate::domain::editor::EditorError;
use crate::domain::git::{GitError, ReleaseError};
use crate::domain::self_update::SelfUpdateError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Editor(#[from] EditorError),

    #[error(transparent)]
    Git(#[from] GitError),

    #[error(transparent)]
    Release(#[from] ReleaseError),

    #[error(transparent)]
    SelfUpdate(#[from] SelfUpdateError),

    #[error(transparent)]
    Snapshot(#[from] SnapshotError),

    #[error(transparent)]
    GitlabApi(#[from] GitlabApiError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Regex error")]
    Regex(#[from] regex::Error),

    #[error("Parse error")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("Version parsing error")]
    SemVer(#[from] semver::Error),

    #[error("未找到 {resource}: {name}")]
    NotFound { resource: String, name: String },

    #[error("{resource}已存在: {name}")]
    AlreadyExists { resource: String, name: String },

    #[error("非法输入: {reason}")]
    InvalidInput { reason: String },

    #[error("不支持: {what}")]
    NotSupported { what: String },

    #[error("{count} 个操作执行失败")]
    ExecutionFailed { count: usize },
}

pub type Result<T> = std::result::Result<T, AppError>;
