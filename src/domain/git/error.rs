//! git 领域模块的错误类型，统一被 [`crate::error::AppError::Git`] 收编。

use thiserror::Error;

/// git 模块内部的 Result 别名，错误侧固定为 [`GitError`]
pub type Result<T> = std::result::Result<T, GitError>;

/// Git 操作失败的原因；`Timeout` 只在流式执行（push/pull/clone 等）路径上产生
#[derive(Debug, Error)]
pub enum GitError {
    #[error("{0}")]
    CommandFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("操作超时: 超过 {timeout} 未完成")]
    Timeout { timeout: String },
}
