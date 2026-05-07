#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Failed to start command '{program}': {reason}")]
    FailedToStart { program: String, reason: String },

    #[error("Command '{program}' failed: {reason}")]
    ExecutionFailed { program: String, reason: String },

    #[error("Command '{command}' exited with non-zero status: {exit_code}")]
    NonZeroExitCode { command: String, exit_code: i32 },

    #[error("I/O error: {message}")]
    IoError { message: String },

    #[error("Timeout after {timeout_ms}ms waiting for command '{program}'")]
    Timeout { program: String, timeout_ms: u64 },
}
