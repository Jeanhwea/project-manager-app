//! Editor domain module
//!
//! This module contains file editing utilities for version bumping.

/// Editor-specific error type
#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Write error: {0}")]
    WriteError(#[from] std::io::Error),
    
    #[error("Version format error: {0}")]
    VersionFormatError(String),
}

/// File types supported for version editing
#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    CargoToml,
    PackageJson,
    PyProject,
    VersionText,
    Cmake,
    PomXml,
    Homebrew,
    ProjectPy,
}

/// Version bump type
#[derive(Debug, Clone, PartialEq)]
pub enum BumpType {
    Major,
    Minor,
    Patch,
    PreRelease(String),
    Build(String),
}

/// Version editing configuration
#[derive(Debug, Clone)]
pub struct EditorConfig {
    pub dry_run: bool,
    pub skip_push: bool,
    pub force: bool,
    pub message: Option<String>,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            dry_run: false,
            skip_push: false,
            force: false,
            message: None,
        }
    }
}

/// Common result type for editor operations
pub type Result<T> = std::result::Result<T, EditorError>;