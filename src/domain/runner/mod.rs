//! 外部命令执行层：把「程序 + 参数 + 工作目录 + 输出方式 + 超时」抽象成执行上下文。
//! git 之外的外部命令也应走这里，以便统一超时策略与输出处理。

mod command;
mod context;
pub(crate) mod pool;

pub use command::CommandRunner;
pub use context::{CommandResult, ExecutionContext, OutputMode};
