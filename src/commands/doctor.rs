use crate::commands::{RepoPathArgs, init_repo_walker};
use crate::control::command::MultiRepoCommand;
use crate::control::context::collect_context;
use crate::domain::git::diagnose_repo;
use crate::domain::git::repository::RepoWalker;
use crate::error::{AppError, Result};
use crate::model::git::GitContext;
use crate::model::plan::{ExecutionPlan, GitOperation, MessageOperation, Operation};
use crate::utils::output::Output;
use std::path::Path;

#[derive(Debug, clap::Args)]
pub struct DoctorArgs {
    #[command(flatten)]
    pub repo_path: RepoPathArgs,
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Automatically fix detected issues"
    )]
    pub fix: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be fixed"
    )]
    pub dry_run: bool,
}

pub(crate) struct DoctorContext {
    git_ctx: Option<GitContext>,
    issues: Vec<String>,
}

impl MultiRepoCommand for DoctorArgs {
    type Context = DoctorContext;

    fn context(&self, repo_path: &Path) -> Result<DoctorContext> {
        let issues = diagnose_repo(repo_path);
        let git_ctx = if self.fix && !issues.is_empty() {
            collect_context(repo_path).ok()
        } else {
            None
        };

        Ok(DoctorContext { git_ctx, issues })
    }

    fn plan(&self, ctx: &DoctorContext) -> Result<ExecutionPlan> {
        let Some(git_ctx) = &ctx.git_ctx else {
            return Ok(ExecutionPlan::new());
        };

        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);

        for issue in &ctx.issues {
            if issue.contains("陈旧") {
                plan.add(GitOperation::PruneRemote {
                    remote: "origin".to_string(),
                });
            } else if issue.contains("上游跟踪分支") || issue.contains("只有一个本地分支")
            {
                plan.add(GitOperation::SetUpstream {
                    remote: "origin".to_string(),
                    branch: git_ctx.current_branch.clone(),
                });
            } else if issue.contains("仓库大小较大") {
                plan.add(GitOperation::Gc);
            } else if issue.contains("stash") {
                plan.add(MessageOperation::Warning {
                    msg: "stash 条目需要手动处理".to_string(),
                });
            } else if let Some(rest) = issue.strip_prefix("remote 名称不匹配: ")
                && let Some((current, expected_with_host)) = rest.split_once(" -> ")
            {
                let expected = expected_with_host
                    .split(' ')
                    .next()
                    .unwrap_or(expected_with_host);
                if git_ctx.has_remote(expected) {
                    plan.add(MessageOperation::Warning {
                        msg: format!("目标 remote 名称 {} 已存在，跳过", expected),
                    });
                } else {
                    plan.add(GitOperation::RenameRemote {
                        old: current.to_string(),
                        new: expected.to_string(),
                    });
                }
            }
        }

        Ok(plan)
    }

    fn run(&self, walker: &RepoWalker) -> Result<()> {
        if self.fix {
            check_prerequisites()?;
        }

        let total = walker.total();
        let mut total_issues = 0;
        let mut total_fixed = 0;

        for (index, repo_info) in walker.repositories().iter().enumerate() {
            let repo_path = &repo_info.path;
            let ctx = self.context(repo_path);

            let issues = ctx.as_ref().map(|c| c.issues.clone()).unwrap_or_default();
            if issues.is_empty() {
                Output::repo_header(index + 1, total, repo_path);
                Output::success(&format!("{}: 健康", repo_path.display()));
                continue;
            }

            total_issues += issues.len();
            Output::repo_header(index + 1, total, repo_path);
            Output::warning(&format!("{}: {} 个问题", repo_path.display(), issues.len()));

            for issue in &issues {
                Output::detail("问题", issue);
            }

            if self.fix
                && let Ok(ctx) = ctx
            {
                let plan = self.plan(&ctx)?;
                let fixed = plan
                    .operations
                    .iter()
                    .filter(|op| !matches!(op, Operation::Message(_)))
                    .count();
                Self::execute(&plan)?;
                total_fixed += fixed;
            }
        }

        Output::header("诊断汇总");
        Output::item("检查仓库", &walker.total().to_string());
        Output::item("发现问题", &total_issues.to_string());
        if self.fix {
            Output::item("已修复", &total_fixed.to_string());
        }

        Ok(())
    }
}

pub fn run(args: DoctorArgs) -> Result<()> {
    let Some(walker) = init_repo_walker(&args.repo_path)? else {
        return Ok(());
    };

    MultiRepoCommand::run(&args, &walker)
}

fn check_prerequisites() -> Result<()> {
    let tools = ["git"];
    let missing: Vec<&str> = tools
        .iter()
        .filter(|tool| !crate::utils::is_command_available(tool))
        .copied()
        .collect();

    if !missing.is_empty() {
        return Err(AppError::command_not_available(&missing.join(", ")));
    }

    Ok(())
}
