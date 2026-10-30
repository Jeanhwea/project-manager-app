//! 编辑器模块的错误类型。

#[derive(Debug, thiserror::Error)]
/// 配置文件解析、版本号定位、改写或落盘过程中可能出现的错误。
pub enum EditorError {
    /// 配置文件本身无法被对应格式的解析器解析。
    #[error("Parse error: {0}")]
    ParseError(String),

    /// 文件写入失败，由 std::io::Error 自动转换而来。
    #[error("Write error: {0}")]
    WriteError(#[from] std::io::Error),

    /// 版本号字符串不符合 semver 时返回。
    #[error("Version format error: {0}")]
    VersionFormatError(String),

    /// 文件中找不到可改写的版本号字段。
    #[error("Version not found: {0}")]
    VersionNotFound(String),

    /// 改写后的内容无法再通过原格式校验，说明可能破坏了文件格式。
    #[error("Format preservation error: {0}")]
    FormatPreservationError(String),
}

/// 编辑器模块统一的 Result 别名。
pub type Result<T> = std::result::Result<T, EditorError>;
