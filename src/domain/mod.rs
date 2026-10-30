//! domain 层：承载与外部世界打交道并实现领域能力的代码。
//! 命令层只依赖这里暴露的类型与函数，不直接调用外部进程；其中 `git` 负责 Git 仓库操作，
//! `runner` 负责外部命令的统一执行。

pub mod config;
pub mod editor;
pub mod git;
pub mod project_config;
pub mod runner;
pub mod self_update;
