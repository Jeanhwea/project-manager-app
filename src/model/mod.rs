//! 纯数据模型层：只描述 git 状态、可执行操作和编排结构，不含任何 IO 逻辑。
//! 实际执行由 `crate::engine` 完成，模型与其互不依赖是实现 plan/dry-run 的前提。

pub mod git;
pub mod operation;
pub mod plan;
pub mod project_config;
