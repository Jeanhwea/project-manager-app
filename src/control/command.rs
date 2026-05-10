use crate::control::plan;
use crate::domain::git::repository::RepoWalker;
use crate::error::Result;
use crate::model::plan::ExecutionPlan;
use crate::utils::output::Output;
use std::path::Path;

pub(crate) trait Command {
    type Context;

    fn context(&self) -> Result<Self::Context>;
    fn plan(&self, ctx: &Self::Context) -> Result<ExecutionPlan>;

    fn execute(plan: &ExecutionPlan) -> Result<()> {
        plan::run_plan(plan)
    }

    fn run(&self) -> Result<()> {
        let ctx = self.context()?;
        let plan = self.plan(&ctx)?;
        Self::execute(&plan)
    }
}

pub(crate) trait MultiRepoCommand {
    type Context;

    fn context(&self, repo_path: &Path) -> Result<Self::Context>;
    fn plan(&self, ctx: &Self::Context) -> Result<ExecutionPlan>;

    fn execute(plan: &ExecutionPlan) -> Result<()> {
        plan::run_plan(plan)
    }

    fn run(&self, walker: &RepoWalker) -> Result<()> {
        let total = walker.total();
        for (index, repo_info) in walker.repositories().iter().enumerate() {
            let repo_path = &repo_info.path;
            Output::repo_header(index + 1, total, repo_path);

            match self.context(repo_path) {
                Ok(ctx) => match self.plan(&ctx) {
                    Ok(plan) => {
                        if let Err(e) = Self::execute(&plan) {
                            Output::error(&format!("{}", e));
                        }
                    }
                    Err(e) => Output::error(&format!("{}", e)),
                },
                Err(_) => continue,
            }
        }
        Ok(())
    }
}
