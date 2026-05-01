//! Domain layer module
//!
//! This module contains the core domain logic and business rules
//! separated from CLI infrastructure and command implementations.

pub mod config;
pub mod editor;
pub mod git;
pub mod gitlab;
pub mod runner;

/// Domain error enumeration
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Git error: {0}")]
    Git(#[from] git::GitError),

    #[error("GitLab error: {0}")]
    GitLab(#[from] gitlab::GitLabError),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Editor error: {0}")]
    Editor(#[from] editor::EditorError),

    #[error("Runner error: {0}")]
    Runner(#[from] runner::RunnerError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
