use crate::error::{AppError, Result};
use crate::model::plan::ExecutionResult;
use crate::utils::output;

pub(crate) trait Command {
    type Context;
    type Plan;

    fn collect(&self) -> Result<Self::Context>;

    fn plan(&self, ctx: &Self::Context) -> Result<Self::Plan>;

    fn execute(&self, plan: &Self::Plan) -> Result<ExecutionResult>;

    fn run(&self) -> Result<()> {
        let ctx = self.collect()?;
        let plan = self.plan(&ctx)?;
        let result = self.execute(&plan)?;
        if !result.is_success() {
            for err in result.errors() {
                output::error(&format!("执行失败: {}", err.description()));
                if let Some(hint) = err.recovery_hint() {
                    output::detail("恢复指引", hint);
                }
            }
            return Err(AppError::ExecutionFailed {
                count: result.errors().len(),
            });
        }
        Ok(())
    }
}
