#[allow(dead_code)]
pub mod config;
#[allow(dead_code)]
pub mod editor;
#[allow(dead_code)]
pub mod git;
#[allow(dead_code)]
pub mod gitlab;
#[allow(dead_code)]
pub mod runner;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Git error: {0}")]
    Git(#[from] git::GitError),

    #[error("GitLab error: {0}")]
    GitLab(#[from] Box<gitlab::GitLabError>),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Editor error: {0}")]
    Editor(#[from] editor::EditorError),

    #[error("Runner error: {0}")]
    Runner(#[from] runner::RunnerError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
