use crate::commands::RepoPathArgs;
use crate::control::command::MultiRepoCommand;
use crate::domain::git::GitCommandRunner;
use crate::domain::git::collect_context;
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
        short,
        default_value = "origin",
        help = "Remote name for deleting remote branches"
    )]
    pub remote: String,
    #[arg(
        long,
        default_value = "false",
        help = "Also delete matching remote branches"
    )]
    pub delete_remote: bool,
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

#[derive(Debug)]
pub(crate) struct BranchListContext {
    git_ctx: GitContext,
}

#[derive(Debug)]
pub(crate) struct BranchCleanContext {
    branches_to_delete: Vec<String>,
    remote_name: String,
    delete_remote: bool,
}

#[derive(Debug)]
pub(crate) struct BranchSwitchContext {
    exists: bool,
}

#[derive(Debug)]
pub(crate) struct BranchRenameContext {
    exists: bool,
}

impl MultiRepoCommand for BranchListArgs {
    type Context = BranchListContext;

    fn context(&self, repo_path: &Path) -> Result<BranchListContext> {
        let git_ctx = collect_context(repo_path)?;
        Ok(BranchListContext { git_ctx })
    }

    fn plan(&self, ctx: &BranchListContext, repo_path: &Path) -> Result<ExecutionPlan> {
        let _ = repo_path;
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
}

impl MultiRepoCommand for BranchCleanArgs {
    type Context = BranchCleanContext;

    fn context(&self, repo_path: &Path) -> Result<BranchCleanContext> {
        let git_ctx = collect_context(repo_path)?;

        let remote_name = if git_ctx.has_remote(&self.remote) {
            self.remote.clone()
        } else {
            git_ctx
                .preferred_remote()
                .or_else(|| git_ctx.first_remote_name())
                .unwrap_or_else(|| self.remote.clone())
        };

        let branches_to_delete: Vec<String> = git_ctx
            .local_branches()
            .iter()
            .map(|b| b.name.as_str())
            .filter(|name| *name != git_ctx.current_branch)
            .filter(|name| {
                if let Some(ref pattern) = self.pattern {
                    match_pattern(name, pattern)
                } else {
                    GitCommandRunner::new().is_merged_branch(name, repo_path)
                }
            })
            .map(|s| s.to_string())
            .collect();

        Ok(BranchCleanContext {
            branches_to_delete,
            remote_name,
            delete_remote: self.delete_remote,
        })
    }

    fn plan(&self, ctx: &BranchCleanContext, repo_path: &Path) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);
        for branch in &ctx.branches_to_delete {
            plan.add(GitOperation::DeleteBranch {
                branch: branch.clone(),
                working_dir: repo_path.to_path_buf(),
            });
            if ctx.delete_remote {
                plan.add(GitOperation::DeleteRemoteBranch {
                    remote: ctx.remote_name.clone(),
                    branch: branch.clone(),
                    working_dir: repo_path.to_path_buf(),
                });
            }
        }
        Ok(plan)
    }
}

impl MultiRepoCommand for BranchSwitchArgs {
    type Context = BranchSwitchContext;

    fn context(&self, repo_path: &Path) -> Result<BranchSwitchContext> {
        let git_ctx = collect_context(repo_path)?;
        let exists = git_ctx
            .local_branches()
            .iter()
            .any(|b| b.name == self.branch);
        Ok(BranchSwitchContext { exists })
    }

    fn plan(&self, ctx: &BranchSwitchContext, repo_path: &Path) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();
        if !ctx.exists {
            plan.add(MessageOperation::Skip {
                msg: format!("分支 {} 不存在", self.branch),
            });
            return Ok(plan);
        }

        plan.add(GitOperation::Checkout {
            ref_name: self.branch.clone(),
            working_dir: repo_path.to_path_buf(),
        });
        plan.add(MessageOperation::Success {
            msg: format!("已切换到 {}", self.branch),
        });
        Ok(plan)
    }
}

impl MultiRepoCommand for BranchRenameArgs {
    type Context = BranchRenameContext;

    fn context(&self, repo_path: &Path) -> Result<BranchRenameContext> {
        let git_ctx = collect_context(repo_path)?;
        let exists = git_ctx
            .local_branches()
            .iter()
            .any(|b| b.name == self.old_name);
        Ok(BranchRenameContext { exists })
    }

    fn plan(&self, ctx: &BranchRenameContext, repo_path: &Path) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();
        if !ctx.exists {
            plan.add(MessageOperation::Skip {
                msg: format!("分支 {} 不存在", self.old_name),
            });
            return Ok(plan);
        }

        plan.add(GitOperation::RenameBranch {
            old: self.old_name.clone(),
            new: self.new_name.clone(),
            working_dir: repo_path.to_path_buf(),
        });
        plan.add(MessageOperation::Success {
            msg: format!("{} -> {}", self.old_name, self.new_name),
        });
        Ok(plan)
    }
}

pub fn run(args: BranchArgs) -> Result<()> {
    match args {
        BranchArgs::List(args) => crate::commands::run_multi_repo(&args, &args.repo_path),
        BranchArgs::Clean(args) => crate::commands::run_multi_repo(&args, &args.repo_path),
        BranchArgs::Switch(args) => crate::commands::run_multi_repo(&args, &args.repo_path),
        BranchArgs::Rename(args) => crate::commands::run_multi_repo(&args, &args.repo_path),
    }
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
