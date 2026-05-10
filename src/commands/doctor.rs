use crate::commands::{RepoPathArgs, init_repo_walker};
use crate::control::command::MultiRepoCommand;
use crate::control::context::collect_context;
use crate::domain::git::remote::diagnose_remote_names;
use crate::domain::git::GitCommandRunner;
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

fn diagnose_repo(repo_path: &Path) -> Vec<String> {
    let mut issues = Vec::new();
    let runner = GitCommandRunner::new();

    if let Ok(output) = runner.execute(&["symbolic-ref", "HEAD"], Some(repo_path))
        && output.trim().is_empty()
    {
        issues.push("HEAD 处于 detached 状态".to_string());
    }

    if let Ok(output) = runner.execute(&["remote"], Some(repo_path))
        && output.trim().is_empty()
    {
        issues.push("没有配置远程仓库".to_string());
    }

    if let Ok(output) = runner.execute(&["branch", "-r"], Some(repo_path)) {
        let remote_branches: Vec<&str> = output.lines().collect();
        if remote_branches.is_empty() {
            issues.push("没有远程跟踪分支".to_string());
        }
    }

    if let Ok(output) = runner.execute(&["branch", "--list"], Some(repo_path)) {
        let local_branches: Vec<&str> = output.lines().collect();
        if local_branches.len() == 1 {
            issues.push("只有一个本地分支".to_string());
        }
    }

    if let Ok(output) = runner.execute(
        &[
            "for-each-ref",
            "--sort=-creatordate",
            "--format=%(creatordate:iso)",
            "refs/stash",
        ],
        Some(repo_path),
    ) && !output.trim().is_empty()
    {
        issues.push("存在 stash 条目".to_string());
    }

    if let Ok(output) = runner.execute(&["remote", "show"], Some(repo_path)) {
        for remote in output.lines() {
            let remote = remote.trim();
            if remote.is_empty() {
                continue;
            }
            if let Ok(remote_output) =
                runner.execute(&["remote", "show", remote], Some(repo_path))
                && remote_output.contains("(stale)")
            {
                issues.push(format!("远程仓库 {} 的引用已陈旧", remote));
            }
        }
    }

    if let Ok(output) = runner.execute(&["count-objects", "-vH"], Some(repo_path)) {
        for line in output.lines() {
            if let Some(size_str) = line.strip_prefix("size-pack:")
                && let Some(size_num) = size_str.split_whitespace().next()
                && let Ok(size) = size_num.parse::<f64>()
                && size > 100.0
            {
                issues.push(format!("仓库大小较大: {}", size_str.trim()));
            }
        }
    }

    for remote_issue in diagnose_remote_names(repo_path) {
        issues.push(format!(
            "remote 名称不匹配: {} -> {} (主机: {})",
            remote_issue.current_name, remote_issue.expected_name, remote_issue.host
        ));
    }

    issues
}
