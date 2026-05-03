pub mod dry_run;

pub use dry_run::DryRunContext;

#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
