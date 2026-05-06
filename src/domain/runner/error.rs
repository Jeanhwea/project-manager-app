/// 命令执行错误
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// 命令无法启动
    #[error("Failed to start command '{program}': {reason}")]
    FailedToStart { program: String, reason: String },

    /// 命令执行过程中失败
    #[error("Command '{program}' failed: {reason}")]
    ExecutionFailed { program: String, reason: String },

    /// 命令以非零退出码结束
    #[error("Command '{command}' exited with non-zero status: {exit_code}")]
    NonZeroExitCode { command: String, exit_code: i32 },

    /// I/O 错误
    #[error("I/O error: {message}")]
    IoError { message: String },

    /// 命令执行超时
    #[error("Timeout after {timeout_ms}ms waiting for command '{program}'")]
    Timeout { program: String, timeout_ms: u64 },
}
