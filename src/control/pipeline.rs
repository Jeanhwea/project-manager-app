use crate::domain::git::repository::RepoWalker;
use crate::model::plan::ExecutionPlan;
use crate::utils::output::Output;
use std::path::Path;

pub struct Pipeline;

impl Pipeline {
    pub fn run<A, C>(
        args: A,
        get_context: impl FnOnce(&A) -> anyhow::Result<C>,
        make_plan: impl FnOnce(&A, &C) -> anyhow::Result<ExecutionPlan>,
    ) -> anyhow::Result<()> {
        let ctx = get_context(&args)?;
        let plan = make_plan(&args, &ctx)?;
        crate::control::plan::run_plan(&plan)
    }

    pub fn run_multi_repo<A, C>(
        args: &A,
        walker: &RepoWalker,
        get_context: impl Fn(&A, &Path) -> anyhow::Result<C>,
        make_plan: impl Fn(&A, &C) -> anyhow::Result<ExecutionPlan>,
    ) -> anyhow::Result<()> {
        let total = walker.total();
        for (index, repo_info) in walker.repositories().iter().enumerate() {
            let repo_path = &repo_info.path;
            Output::repo_header(index + 1, total, repo_path);

            match get_context(args, repo_path) {
                Ok(ctx) => match make_plan(args, &ctx) {
                    Ok(plan) => {
                        if let Err(e) = crate::control::plan::run_plan(&plan) {
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
