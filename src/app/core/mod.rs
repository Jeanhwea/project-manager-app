mod context;
mod registry;

pub use context::CommandContext;
pub use registry::CommandRegistry;

use anyhow::Result;

pub trait Command {
    fn name(&self) -> &'static str;
    fn execute(&self, ctx: &CommandContext) -> Result<()>;
}
