pub mod command;
pub mod context;
pub mod dry_run;
pub mod error;
pub mod result;

pub use command::CommandRunner;
pub use command::DefaultCommandRunner;
pub use context::ExecutionContext;
pub use dry_run::DryRunContext;
pub use error::CommandError;
pub use result::CommandResult;

/// 命令输出模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    /// 捕获输出模式 - 命令执行完成后返回完整输出
    /// 适用于需要解析输出的场景（如 git status）
    #[default]
    Capture,

    /// 流式输出模式 - 实时显示命令输出
    /// 适用于长时间运行的命令（如 git pull/push/fetch）
    Streaming,

    /// DryRun 模式 - 仅打印将要执行的命令
    /// 适用于预览变更的场景
    DryRun,
}
