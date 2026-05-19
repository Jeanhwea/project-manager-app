use crate::control::plan;
use crate::domain::git::repository::RepoWalker;
use crate::error::Result;
use crate::model::plan::{ExecutionPlan, ExecutionResult};
use crate::utils::output::Output;
use std::path::Path;

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
                Output::error(&format!("执行失败: {}", err.description()));
                if let Some(hint) = err.recovery_hint() {
                    Output::detail("恢复指引", hint);
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn run_multi_repo<C: MultiRepo>(cmd: &C, walker: &RepoWalker) -> Result<()> {
    let total = walker.total();
    for (index, repo_info) in walker.repositories().iter().enumerate() {
        let repo_path = &repo_info.path;
        Output::repo_header(index + 1, total, repo_path);

        match cmd.collect(repo_path) {
            Ok(ctx) => match cmd.plan(&ctx, repo_path) {
                Ok(plan) => {
                    if let Err(e) = cmd.execute(&plan) {
                        Output::error(&format!("{}", e));
                    }
                }
                Err(e) => Output::error(&format!("{}", e)),
            },
            Err(e) => {
                Output::warning(&format!("跳过 {}: {}", repo_path.display(), e));
                continue;
            }
        }
    }
    Ok(())
}

pub(crate) trait MultiRepo {
    type Context;
    type Plan;

    fn collect(&self, repo_path: &Path) -> Result<Self::Context>;
    fn plan(&self, ctx: &Self::Context, repo_path: &Path) -> Result<Self::Plan>;
    fn execute(&self, plan: &Self::Plan) -> Result<ExecutionResult>;
}
