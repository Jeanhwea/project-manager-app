use crate::domain::git::repository::{RepoInfo, RepoWalker};
use crate::domain::runner::pool;
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

pub fn run_multi_repo_cmd(cmd: &(impl MultiRepo + Sync), repo_path: &RepoPathArgs) -> Result<()> {
    run_multi_repo_cmd_with_jobs(cmd, repo_path, 1)
}

pub fn run_multi_repo_cmd_with_jobs(
    cmd: &(impl MultiRepo + Sync),
    repo_path: &RepoPathArgs,
    jobs: usize,
) -> Result<()> {
    let Some(walker) = init_repo_walker(repo_path)? else {
        return Ok(());
    };
    run_multi_repo(cmd, &walker, jobs)
}

fn run_multi_repo<C: MultiRepo + Sync>(cmd: &C, walker: &RepoWalker, jobs: usize) -> Result<()> {
    let jobs = pool::resolve_jobs(jobs, walker.total());
    if jobs <= 1 {
        run_serial(cmd, walker.repositories());
    } else {
        pool::run_parallel(cmd, walker.repositories(), jobs);
    }
    Ok(())
}

fn run_serial<C: MultiRepo>(cmd: &C, repos: &[RepoInfo]) {
    let total = repos.len();
    for (index, repo_info) in repos.iter().enumerate() {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::runner::pool::SyncProgress;
    use crate::model::plan::{DisplayMessage, ExecutionPlan};
    use std::fs;
    use std::path::PathBuf;

    struct FakeSync;

    impl MultiRepo for FakeSync {
        type Context = PathBuf;
        type Plan = ExecutionPlan;

        fn collect(&self, repo_path: &Path) -> Result<PathBuf> {
            Ok(repo_path.to_path_buf())
        }

        fn plan(&self, ctx: &PathBuf, _repo_path: &Path) -> Result<ExecutionPlan> {
            let mut plan = ExecutionPlan::new();
            plan.add_message(DisplayMessage::Item {
                label: "仓库".to_string(),
                value: ctx.display().to_string(),
            });
            Ok(plan)
        }

        fn execute(&self, _plan: &ExecutionPlan) -> Result<ExecutionResult> {
            let mut result = ExecutionResult::new();
            result.add_executed();
            Ok(result)
        }
    }

    fn temp_repos(count: usize) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..count {
            fs::create_dir_all(dir.path().join(format!("repo{}", i)).join(".git")).unwrap();
        }
        dir
    }

    fn repo_path_args(path: &Path) -> RepoPathArgs {
        RepoPathArgs {
            max_depth: 3,
            path: path.to_string_lossy().into_owned(),
        }
    }

    #[test]
    fn test_resolve_jobs_serial_by_default() {
        assert_eq!(pool::resolve_jobs(1, 10), 1);
    }

    #[test]
    fn test_resolve_jobs_explicit() {
        assert_eq!(pool::resolve_jobs(4, 10), 4);
        assert_eq!(pool::resolve_jobs(2, 1), 1);
    }

    #[test]
    fn test_resolve_jobs_clamped_to_total() {
        assert_eq!(pool::resolve_jobs(100, 3), 3);
    }

    #[test]
    fn test_resolve_jobs_auto_converges() {
        assert_eq!(pool::resolve_jobs(0, 1), 1);
        let jobs = pool::resolve_jobs(0, 8);
        assert!((1..=8).contains(&jobs));
    }

    #[test]
    fn test_run_multi_repo_serial() {
        let dir = temp_repos(3);
        run_multi_repo_cmd(&FakeSync, &repo_path_args(dir.path())).unwrap();
    }

    #[test]
    fn test_run_multi_repo_parallel() {
        let dir = temp_repos(3);
        run_multi_repo_cmd_with_jobs(&FakeSync, &repo_path_args(dir.path()), 2).unwrap();
    }

    #[test]
    fn test_run_multi_repo_parallel_auto() {
        let dir = temp_repos(2);
        run_multi_repo_cmd_with_jobs(&FakeSync, &repo_path_args(dir.path()), 0).unwrap();
    }

    #[test]
    fn test_run_multi_repo_single_repo_stays_serial() {
        let dir = temp_repos(1);
        run_multi_repo_cmd_with_jobs(&FakeSync, &repo_path_args(dir.path()), 4).unwrap();
    }

    #[test]
    fn test_execution_result_merge() {
        let mut total = ExecutionResult::new();
        total.add_executed();
        let mut other = ExecutionResult::new();
        other.add_executed();
        other.add_executed();
        total.merge(other);
        assert_eq!(total.executed_count(), 3);
        assert!(total.is_success());
    }

    #[test]
    fn test_failure_reason_strips_repo_prefix() {
        let mut result = ExecutionResult::new();
        result.add_error(crate::model::plan::OperationError::new(
            "[/data/repo] git pull origin main",
        ));
        assert_eq!(pool::failure_reason(&result), "git pull origin main");
    }

    #[test]
    fn test_failure_reason_without_errors() {
        assert_eq!(pool::failure_reason(&ExecutionResult::new()), "执行失败");
    }

    #[test]
    fn test_first_line_picks_first_non_empty() {
        assert_eq!(pool::first_line("\n  boom \nnext"), "boom");
        assert_eq!(pool::first_line("solo"), "solo");
    }

    #[test]
    fn test_sync_progress_counters() {
        let progress = SyncProgress::new();
        assert_eq!(progress.in_flight(), 0);
        progress.begin();
        assert_eq!(progress.in_flight(), 1);
        progress.begin();
        assert_eq!(progress.in_flight(), 2);
        assert_eq!(progress.complete(), 1);
        assert_eq!(progress.in_flight(), 1);
        assert_eq!(progress.complete(), 2);
        assert_eq!(progress.in_flight(), 0);
    }
}
