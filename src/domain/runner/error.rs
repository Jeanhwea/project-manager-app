#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Failed to start command '{program}': {reason}")]
    FailedToStart { program: String, reason: String },

    #[error("Command '{program}' failed: {reason}")]
    ExecutionFailed { program: String, reason: String },

    #[error("I/O error: {message}")]
    IoError { message: String },
}
