use crate::domain::git::repository::RepoWalker;
use crate::error::Result;
use crate::model::plan::ExecutionResult;
use crate::utils::output;
use std::path::Path;

pub(crate) trait MultiRepo {
    type Context;
    type Plan;

    fn collect(&self, repo_path: &Path) -> Result<Self::Context>;
    fn plan(&self, ctx: &Self::Context, repo_path: &Path) -> Result<Self::Plan>;
    fn execute(&self, plan: &Self::Plan) -> Result<ExecutionResult>;
}

#[derive(Debug, clap::Args)]
pub struct RepoPathArgs {
    #[arg(
        long,
        short,
        default_value = "3",
        help = "Maximum depth to search for repositories"
    )]
    pub max_depth: usize,
    #[arg(default_value = ".", help = "Path to search for repositories")]
    pub path: String,
}

fn init_repo_walker(args: &RepoPathArgs) -> Result<Option<RepoWalker>> {
    let search_path = resolve_walker_path(&args.path)?;
    let walker = RepoWalker::new(&search_path, args.max_depth)?;
    if walker.is_empty() {
        output::not_found("未找到 Git 仓库");
        return Ok(None);
    }
    Ok(Some(walker))
}

fn resolve_walker_path(path: &str) -> Result<std::path::PathBuf> {
    if path == "." {
        let cwd = std::env::current_dir()?;
        Ok(crate::domain::git::repository::find_repository_upwards(&cwd).unwrap_or(cwd))
    } else {
        Ok(crate::utils::path::canonicalize_path(path)?)
    }
}

pub fn run_multi_repo_cmd(cmd: &impl MultiRepo, repo_path: &RepoPathArgs) -> Result<()> {
    let Some(walker) = init_repo_walker(repo_path)? else {
        return Ok(());
    };
    run_multi_repo(cmd, &walker)
}

fn run_multi_repo<C: MultiRepo>(cmd: &C, walker: &RepoWalker) -> Result<()> {
    let total = walker.total();
    for (index, repo_info) in walker.repositories().iter().enumerate() {
        let repo_path = &repo_info.path;
        output::repo_header(index + 1, total, repo_path);

        match cmd.collect(repo_path) {
            Ok(ctx) => match cmd.plan(&ctx, repo_path) {
                Ok(plan) => {
                    if let Err(e) = cmd.execute(&plan) {
                        output::error_chain(&e);
                    }
                }
                Err(e) => output::error_chain(&e),
            },
            Err(e) => {
                output::warning(&format!("跳过 {}: {}", repo_path.display(), e));
                continue;
            }
        }
    }
    Ok(())
}
