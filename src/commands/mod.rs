//! 子命令实现，每个文件对应一个 pma 子命令，统一走 collect → plan → execute 三段式。
//! `multi_repo` 与 `runtime` 是两种共用骨架（批量遍历 / 单项目），故保持模块私有再重导出。

pub mod branch;
pub mod config;
pub mod doctor;
pub mod fork;
pub mod gitlab;
pub mod release;
pub mod self_update;
pub mod snap;
pub mod status;
pub mod sync;

mod multi_repo;
mod runtime;

pub(crate) use multi_repo::{
    MultiRepo, RepoPathArgs, run_multi_repo_cmd, run_multi_repo_cmd_with_jobs,
};
pub(crate) use runtime::Command;
