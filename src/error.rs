//! 全局统一的错误类型 `AppError`。
//! 各层（domain / commands / 外部库）的专项错误通过 `#[from]` 收拢到这里，
//! 再由 main 统一展开成错误链打印，业务代码只面对单一错误类型。

use crate::commands::gitlab::GitlabApiError;
use crate::commands::snap::SnapshotError;
use crate::domain::editor::EditorError;
use crate::domain::git::{GitError, ReleaseError};
use crate::domain::self_update::SelfUpdateError;

/// 应用层统一的错误枚举，各变体外的 `#[error(transparent)]` 表示直接透传下层错误的展示文案
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

    #[error("操作超时: 超过 {timeout} 未完成")]
    Timeout { timeout: String },
}

/// 业务代码统一使用的 Result 别名，错误侧固定为 `AppError`
pub type Result<T> = std::result::Result<T, AppError>;
