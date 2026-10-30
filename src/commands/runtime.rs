//! 单项目命令（如 fork）的执行骨架：`Command` trait 把流程固定为 collect → plan → execute。

use crate::error::Result;
use crate::model::plan::ExecutionResult;
use crate::utils::output;

/// 只作用于单个目录的命令；批量场景请用 [`crate::commands::MultiRepo`]
pub(crate) trait Command {
    /// 采集阶段读到的输入快照
    type Context;
    /// 由 Context 推导出的待执行计划
    type Plan;

    /// 读取并校验外部状态，不满足前提就在这里报错
    fn collect(&self) -> Result<Self::Context>;

    /// 把 Context 转成 `ExecutionPlan`，此阶段不产生副作用
    fn plan(&self, ctx: &Self::Context) -> Result<Self::Plan>;

    /// 落地执行计划
    fn execute(&self, plan: &Self::Plan) -> Result<ExecutionResult>;

    /// 默认编排：三段顺序执行，失败时打印错误链与恢复指引后仍返回 Ok
    // 不向上传播 Err 意味着进程退出码保持成功，错误交给用户人工处理
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
        }
        Ok(())
    }
}
