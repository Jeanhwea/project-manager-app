use crate::commands::MultiRepo;
use crate::commands::RepoPathArgs;
use crate::domain::git::GitCommandRunner;
use crate::domain::git::GitOperation;
use crate::domain::git::collect_context;
use crate::engine::plan;
use crate::error::Result;
use crate::model::git::{Branch, GitContext};
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase};
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
    #[command(visible_alias = "a")]
    All(BranchAllArgs),
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

#[derive(Debug, clap::Args)]
pub struct BranchAllArgs {
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

#[derive(Debug)]
pub(crate) struct BranchAllContext {
    git_ctx: GitContext,
}

impl MultiRepo for BranchListArgs {
    type Context = BranchListContext;
    type Plan = ExecutionPlan;

    fn collect(&self, repo_path: &Path) -> Result<BranchListContext> {
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
                plan.add_message(DisplayMessage::Item {
                    label: "当前".to_string(),
                    value: branch.name.clone(),
                });
            } else {
                plan.add_message(DisplayMessage::Skip {
                    msg: format!("  {}", branch.name),
                });
            }
        }

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

impl MultiRepo for BranchCleanArgs {
    type Context = BranchCleanContext;
    type Plan = ExecutionPlan;

    fn collect(&self, repo_path: &Path) -> Result<BranchCleanContext> {
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

        let mut clean_phase = Phase::new("清理分支");
        for branch in &ctx.branches_to_delete {
            clean_phase.add(GitOperation::DeleteBranch {
                branch: branch.clone(),
                working_dir: repo_path.to_path_buf(),
            });
            if ctx.delete_remote {
                clean_phase.add(GitOperation::DeleteRemoteBranch {
                    remote: ctx.remote_name.clone(),
                    branch: branch.clone(),
                    working_dir: repo_path.to_path_buf(),
                });
            }
        }
        if !clean_phase.is_empty() {
            plan.add_phase(clean_phase);
        }

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

impl MultiRepo for BranchSwitchArgs {
    type Context = BranchSwitchContext;
    type Plan = ExecutionPlan;

    fn collect(&self, repo_path: &Path) -> Result<BranchSwitchContext> {
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
            plan.add_message(DisplayMessage::Skip {
                msg: format!("分支 {} 不存在", self.branch),
            });
            return Ok(plan);
        }

        let mut switch_phase = Phase::new("切换分支");
        switch_phase.add(GitOperation::Checkout {
            ref_name: self.branch.clone(),
            working_dir: repo_path.to_path_buf(),
        });
        plan.add_phase(switch_phase);

        plan.add_message(DisplayMessage::Success {
            msg: format!("已切换到 {}", self.branch),
        });
        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

impl MultiRepo for BranchRenameArgs {
    type Context = BranchRenameContext;
    type Plan = ExecutionPlan;

    fn collect(&self, repo_path: &Path) -> Result<BranchRenameContext> {
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
            plan.add_message(DisplayMessage::Skip {
                msg: format!("分支 {} 不存在", self.old_name),
            });
            return Ok(plan);
        }

        let mut rename_phase = Phase::new("重命名分支");
        rename_phase.add(GitOperation::RenameBranch {
            old: self.old_name.clone(),
            new: self.new_name.clone(),
            working_dir: repo_path.to_path_buf(),
        });
        plan.add_phase(rename_phase);

        plan.add_message(DisplayMessage::Success {
            msg: format!("{} -> {}", self.old_name, self.new_name),
        });
        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

impl MultiRepo for BranchAllArgs {
    type Context = BranchAllContext;
    type Plan = ExecutionPlan;

    fn collect(&self, repo_path: &Path) -> Result<BranchAllContext> {
        let git_ctx = collect_context(repo_path)?;
        Ok(BranchAllContext { git_ctx })
    }

    fn plan(&self, ctx: &BranchAllContext, repo_path: &Path) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();
        let current_branch = &ctx.git_ctx.current_branch;

        let other_branches: Vec<&Branch> = ctx
            .git_ctx
            .local_branches()
            .iter()
            .filter(|b| b.name != current_branch.as_str())
            .cloned()
            .collect();

        if other_branches.is_empty() {
            plan.add_message(DisplayMessage::Skip {
                msg: "没有其他本地分支需要处理".to_string(),
            });
            return Ok(plan);
        }

        let preferred_remote = ctx.git_ctx.preferred_remote();

        let mut sync_phase = Phase::new("同步分支");
        for branch in &other_branches {
            sync_phase.add(GitOperation::Checkout {
                ref_name: branch.name.clone(),
                working_dir: repo_path.to_path_buf(),
            });

            if let Some(ref remote) = preferred_remote {
                if ctx.git_ctx.has_remote_branch(remote, &branch.name) {
                    sync_phase.add(GitOperation::Pull {
                        remote: remote.clone(),
                        branch: branch.name.clone(),
                        working_dir: repo_path.to_path_buf(),
                    });
                } else {
                    sync_phase.add_message(DisplayMessage::Skip {
                        msg: format!("跳过拉取 {}/{} (远程无此分支)", remote, branch.name),
                    });
                }
            } else {
                sync_phase.add_message(DisplayMessage::Skip {
                    msg: format!("跳过拉取 {} (无绑定远端)", branch.name),
                });
            }
        }

        sync_phase.add(GitOperation::Checkout {
            ref_name: current_branch.clone(),
            working_dir: repo_path.to_path_buf(),
        });
        plan.add_phase(sync_phase);

        plan.add_message(DisplayMessage::Success {
            msg: format!(
                "已处理 {} 个分支，当前分支: {}",
                other_branches.len(),
                current_branch
            ),
        });

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

pub fn run(args: BranchArgs) -> Result<()> {
    match args {
        BranchArgs::List(args) => crate::commands::run_multi_repo_cmd(&args, &args.repo_path),
        BranchArgs::Clean(args) => crate::commands::run_multi_repo_cmd(&args, &args.repo_path),
        BranchArgs::Switch(args) => crate::commands::run_multi_repo_cmd(&args, &args.repo_path),
        BranchArgs::Rename(args) => crate::commands::run_multi_repo_cmd(&args, &args.repo_path),
        BranchArgs::All(args) => crate::commands::run_multi_repo_cmd(&args, &args.repo_path),
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
