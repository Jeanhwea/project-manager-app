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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    #[default]
    Capture,
    Streaming,
    DryRun,
}
