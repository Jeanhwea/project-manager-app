use crate::commands::{RepoPathArgs, init_repo_walker};
use crate::control::context::collect_context;
use crate::control::plan::run_plan;
use crate::domain::git::GitCommandRunner;
use crate::model::plan::{ExecutionPlan, GitOperation};
use crate::utils::output::Output;
use anyhow::Result;
use std::path::Path;

#[derive(Debug, clap::Subcommand)]
pub enum BranchArgs {
    #[command(visible_alias = "ls")]
    List(BranchListArgs),
    #[command(visible_alias = "cl")]
    Clean(BranchCleanArgs),
    #[command(visible_alias = "sw")]
    Switch(BranchSwitchArgs),
    #[command(visible_alias = "rn")]
    Rename(BranchRenameArgs),
}

#[derive(Debug, clap::Args)]
pub struct BranchListArgs {
    #[command(flatten)]
    pub repo_path: RepoPathArgs,
}

#[derive(Debug, clap::Args)]
pub struct BranchCleanArgs {
    #[command(flatten)]
    pub repo_path: RepoPathArgs,
    #[arg(long, short, help = "Branch name pattern to match (e.g. 'feature/*')")]
    pub pattern: Option<String>,
    #[arg(
        long,
        default_value = "false",
        help = "Also delete matching remote branches"
    )]
    pub remote: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be deleted"
    )]
    pub dry_run: bool,
}

#[derive(Debug, clap::Args)]
pub struct BranchSwitchArgs {
    #[arg(help = "Branch name to switch to")]
    pub branch: String,
    #[command(flatten)]
    pub repo_path: RepoPathArgs,
}

#[derive(Debug, clap::Args)]
pub struct BranchRenameArgs {
    #[arg(help = "Old branch name")]
    pub old_name: String,
    #[arg(help = "New branch name")]
    pub new_name: String,
    #[command(flatten)]
    pub repo_path: RepoPathArgs,
}

pub fn run(args: BranchArgs) -> Result<()> {
    match args {
        BranchArgs::List(args) => execute_list(args),
        BranchArgs::Clean(args) => execute_clean(args),
        BranchArgs::Switch(args) => execute_switch(args),
        BranchArgs::Rename(args) => execute_rename(args),
    }
}

fn execute_list(args: BranchListArgs) -> Result<()> {
    let Some(walker) = init_repo_walker(&args.repo_path)? else {
        return Ok(());
    };

    let total = walker.total();

    for (index, repo_info) in walker.repositories().iter().enumerate() {
        let repo_path = &repo_info.path;
        let Ok(ctx) = collect_context(repo_path) else {
            continue;
        };

        if ctx.branches.is_empty() {
            continue;
        }

        Output::repo_header(index + 1, total, repo_path);

        for branch in ctx.local_branches() {
            if branch.is_current {
                Output::item("当前", &branch.name);
            } else {
                Output::message(&format!("  {}", branch.name));
            }
        }
    }

    Ok(())
}

fn execute_clean(args: BranchCleanArgs) -> Result<()> {
    let Some(walker) = init_repo_walker(&args.repo_path)? else {
        return Ok(());
    };

    let total = walker.total();

    for (index, repo_info) in walker.repositories().iter().enumerate() {
        let repo_path = &repo_info.path;
        let Ok(ctx) = collect_context(repo_path) else {
            continue;
        };

        let branches: Vec<&str> = ctx
            .local_branches()
            .iter()
            .map(|b| b.name.as_str())
            .filter(|name| *name != ctx.current_branch)
            .filter(|name| {
                if let Some(ref pattern) = args.pattern {
                    match_pattern(name, pattern)
                } else {
                    is_merged_branch(name, repo_path)
                }
            })
            .collect();

        if branches.is_empty() {
            continue;
        }

        Output::repo_header(index + 1, total, repo_path);

        let mut plan = ExecutionPlan::new().dry_run(args.dry_run);
        for branch in &branches {
            plan.add(GitOperation::BranchDelete {
                branch: branch.to_string(),
            });
            if args.remote {
                plan.add(GitOperation::RemoteDelete {
                    remote: "origin".to_string(),
                    branch: branch.to_string(),
                });
            }
        }
        run_plan(&plan)?;
    }

    Ok(())
}

fn execute_switch(args: BranchSwitchArgs) -> Result<()> {
    let Some(walker) = init_repo_walker(&args.repo_path)? else {
        return Ok(());
    };

    for repo_info in walker.repositories() {
        let repo_path = &repo_info.path;

        let Ok(ctx) = collect_context(repo_path) else {
            continue;
        };

        if !ctx.local_branches().iter().any(|b| b.name == args.branch) {
            Output::skip(&format!(
                "{}: 分支 {} 不存在",
                repo_path.display(),
                args.branch
            ));
            continue;
        }

        let mut plan = ExecutionPlan::new();
        plan.add(GitOperation::Checkout {
            ref_name: args.branch.clone(),
        });
        run_plan(&plan)?;

        Output::success(&format!(
            "{}: 已切换到 {}",
            repo_path.display(),
            args.branch
        ));
    }

    Ok(())
}

fn execute_rename(args: BranchRenameArgs) -> Result<()> {
    let Some(walker) = init_repo_walker(&args.repo_path)? else {
        return Ok(());
    };

    for repo_info in walker.repositories() {
        let repo_path = &repo_info.path;

        let Ok(ctx) = collect_context(repo_path) else {
            continue;
        };

        if !ctx.local_branches().iter().any(|b| b.name == args.old_name) {
            Output::skip(&format!(
                "{}: 分支 {} 不存在",
                repo_path.display(),
                args.old_name
            ));
            continue;
        }

        let mut plan = ExecutionPlan::new();
        plan.add(GitOperation::BranchRename {
            old: args.old_name.clone(),
            new: args.new_name.clone(),
        });
        run_plan(&plan)?;

        Output::success(&format!(
            "{}: {} -> {}",
            repo_path.display(),
            args.old_name,
            args.new_name
        ));
    }

    Ok(())
}

fn match_pattern(name: &str, pattern: &str) -> bool {
    if pattern.contains('*') {
        let regex_pattern = pattern.replace('*', ".*");
        regex::Regex::new(&format!("^{}$", regex_pattern))
            .map(|re| re.is_match(name))
            .unwrap_or(false)
    } else {
        name == pattern
    }
}

fn is_merged_branch(name: &str, repo_path: &Path) -> bool {
    let runner = GitCommandRunner::new();
    runner
        .execute(&["branch", "--merged", "master"], Some(repo_path))
        .map(|output| {
            output
                .lines()
                .any(|line| line.trim_start_matches("* ").trim() == name)
        })
        .unwrap_or(false)
}
