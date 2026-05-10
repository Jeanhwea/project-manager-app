use crate::commands::{RepoPathArgs, init_repo_walker};
use crate::control::context::collect_context;
use crate::control::pipeline::Pipeline;
use crate::domain::git::GitCommandRunner;
use crate::error::Result;
use crate::model::git::GitContext;
use crate::model::plan::{ExecutionPlan, GitOperation, MessageOperation};
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

struct BranchListContext {
    git_ctx: GitContext,
}

struct BranchCleanContext {
    branches_to_delete: Vec<String>,
}

struct BranchSwitchContext {
    exists: bool,
}

struct BranchRenameContext {
    exists: bool,
}

fn execute_list(args: BranchListArgs) -> Result<()> {
    let Some(walker) = init_repo_walker(&args.repo_path)? else {
        return Ok(());
    };

    Pipeline::run_multi_repo(&args, &walker, get_list_context, make_list_plan)
}

fn get_list_context(_args: &BranchListArgs, repo_path: &Path) -> Result<BranchListContext> {
    let git_ctx = collect_context(repo_path)?;
    Ok(BranchListContext { git_ctx })
}

fn make_list_plan(_args: &BranchListArgs, ctx: &BranchListContext) -> Result<ExecutionPlan> {
    let mut plan = ExecutionPlan::new();
    if ctx.git_ctx.branches.is_empty() {
        return Ok(plan);
    }

    for branch in ctx.git_ctx.local_branches() {
        if branch.is_current {
            plan.add(MessageOperation::Item {
                label: "当前".to_string(),
                value: branch.name.clone(),
            });
        } else {
            plan.add(MessageOperation::Skip {
                msg: format!("  {}", branch.name),
            });
        }
    }

    Ok(plan)
}

fn execute_clean(args: BranchCleanArgs) -> Result<()> {
    let Some(walker) = init_repo_walker(&args.repo_path)? else {
        return Ok(());
    };

    Pipeline::run_multi_repo(&args, &walker, get_clean_context, make_clean_plan)
}

fn get_clean_context(args: &BranchCleanArgs, repo_path: &Path) -> Result<BranchCleanContext> {
    let git_ctx = collect_context(repo_path)?;

    let branches_to_delete: Vec<String> = git_ctx
        .local_branches()
        .iter()
        .map(|b| b.name.as_str())
        .filter(|name| *name != git_ctx.current_branch)
        .filter(|name| {
            if let Some(ref pattern) = args.pattern {
                match_pattern(name, pattern)
            } else {
                is_merged_branch(name, repo_path)
            }
        })
        .map(|s| s.to_string())
        .collect();

    Ok(BranchCleanContext { branches_to_delete })
}

fn make_clean_plan(args: &BranchCleanArgs, ctx: &BranchCleanContext) -> Result<ExecutionPlan> {
    let mut plan = ExecutionPlan::new().with_dry_run(args.dry_run);
    for branch in &ctx.branches_to_delete {
        plan.add(GitOperation::DeleteBranch {
            branch: branch.clone(),
        });
        if args.remote {
            plan.add(GitOperation::DeleteRemoteBranch {
                remote: "origin".to_string(),
                branch: branch.clone(),
            });
        }
    }
    Ok(plan)
}

fn execute_switch(args: BranchSwitchArgs) -> Result<()> {
    let Some(walker) = init_repo_walker(&args.repo_path)? else {
        return Ok(());
    };

    Pipeline::run_multi_repo(&args, &walker, get_switch_context, make_switch_plan)
}

fn get_switch_context(args: &BranchSwitchArgs, repo_path: &Path) -> Result<BranchSwitchContext> {
    let git_ctx = collect_context(repo_path)?;
    let exists = git_ctx
        .local_branches()
        .iter()
        .any(|b| b.name == args.branch);
    Ok(BranchSwitchContext { exists })
}

fn make_switch_plan(args: &BranchSwitchArgs, ctx: &BranchSwitchContext) -> Result<ExecutionPlan> {
    let mut plan = ExecutionPlan::new();
    if !ctx.exists {
        plan.add(MessageOperation::Skip {
            msg: format!("分支 {} 不存在", args.branch),
        });
        return Ok(plan);
    }

    plan.add(GitOperation::Checkout {
        ref_name: args.branch.clone(),
    });
    plan.add(MessageOperation::Success {
        msg: format!("已切换到 {}", args.branch),
    });
    Ok(plan)
}

fn execute_rename(args: BranchRenameArgs) -> Result<()> {
    let Some(walker) = init_repo_walker(&args.repo_path)? else {
        return Ok(());
    };

    Pipeline::run_multi_repo(&args, &walker, get_rename_context, make_rename_plan)
}

fn get_rename_context(args: &BranchRenameArgs, repo_path: &Path) -> Result<BranchRenameContext> {
    let git_ctx = collect_context(repo_path)?;
    let exists = git_ctx
        .local_branches()
        .iter()
        .any(|b| b.name == args.old_name);
    Ok(BranchRenameContext { exists })
}

fn make_rename_plan(args: &BranchRenameArgs, ctx: &BranchRenameContext) -> Result<ExecutionPlan> {
    let mut plan = ExecutionPlan::new();
    if !ctx.exists {
        plan.add(MessageOperation::Skip {
            msg: format!("分支 {} 不存在", args.old_name),
        });
        return Ok(plan);
    }

    plan.add(GitOperation::RenameBranch {
        old: args.old_name.clone(),
        new: args.new_name.clone(),
    });
    plan.add(MessageOperation::Success {
        msg: format!("{} -> {}", args.old_name, args.new_name),
    });
    Ok(plan)
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
