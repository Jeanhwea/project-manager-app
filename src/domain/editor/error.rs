#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Write error: {0}")]
    WriteError(#[from] std::io::Error),

    #[error("Version format error: {0}")]
    VersionFormatError(String),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("Format preservation error: {0}")]
    FormatPreservationError(String),
}

pub type Result<T> = std::result::Result<T, EditorError>;
