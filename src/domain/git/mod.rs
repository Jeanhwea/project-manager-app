//! Git 领域层：把 git CLI 的输入输出收敛成领域模型与领域操作，
//! 命令层据此组装计划，不再自己拼接 git 参数。
//! 子模块分工：`command` 底层调用、`context` 只读快照、`diagnose` 健康检查、`operation` 可执行变更、
//! `release` 发布前校验、`repository` 仓库发现、`remote` 命名规约、`snapshot` 快照查询。

mod command;
mod context;
mod diagnose;
mod error;
pub mod operation;
pub mod release;
mod remote;
pub mod repository;
pub mod snapshot;

pub use command::GitCommandRunner;
pub use context::collect_context;
pub use diagnose::{Diagnosis, diagnose_repo};
pub use error::{GitError, Result};
pub use operation::GitOperation;
pub use release::{ReleaseError, ReleaseGitState, resolve_git_root, validate_git_state};
pub use remote::resolve_remote_name;
