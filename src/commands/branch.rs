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
        help = "Dry run: show what would be deleted"
    )]
    pub dry_run: bool,
    #[arg(
        long,
        short = 'D',
        default_value = "false",
        help = "Also delete branches that have NOT been merged into master/dev"
    )]
    pub force_delete_unmerged: bool,
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
    merged_to_protected: Vec<String>, // C类: 已合入protected的分支
    unmerged_to_protected: Vec<String>, // B类: 未合入protected的分支
    remote_orphan_branches: Vec<(String, String)>, // D类: 远端孤儿分支 (remote, branch_name)
    force_delete_unmerged: bool,
    remote_name: String,
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

        // Protected branches that should never be deleted
        let protected_branches = ["master", "main", "develop", "dev"];

        // Determine which protected branches exist locally (to check merge status against them)
        let local_names: Vec<&str> = git_ctx
            .local_branches()
            .iter()
            .map(|b| b.name.as_str())
            .collect();

        // All non-protected, non-current local branches
        let candidates: Vec<String> = git_ctx
            .local_branches()
            .iter()
            .map(|b| b.name.as_str())
            .filter(|name| !protected_branches.contains(name))
            .filter(|name| *name != git_ctx.current_branch)
            .map(|s| s.to_string())
            .collect();

        let runner = GitCommandRunner::new();

        // C类: 已合入protected的分支 → 默认删除
        let mut merged_to_protected: Vec<String> = candidates.clone();
        for base in &protected_branches {
            if local_names.contains(base) {
                if let Ok(merged) = runner.merged_branches_into(base, repo_path) {
                    merged_to_protected.retain(|name| merged.contains(name));
                }
            }
        }

        // B类: 未合入protected的分支 = 候选集 减去 C类
        let unmerged_to_protected: Vec<String> = candidates
            .iter()
            .filter(|name| !merged_to_protected.contains(name))
            .cloned()
            .collect();

        // D类: 远端孤儿分支 — 本地没有跟踪分支的远端分支
        let runner_for_remote = GitCommandRunner::new();
        let remote_orphan_branches: Vec<(String, String)> = if let Ok(remote_branches_output) =
            runner_for_remote.run_local(&["branch", "-r"], Some(repo_path))
        {
            let remote_branch_names: Vec<String> = remote_branches_output
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty() && line != "->")
                .collect();

            // Build a set of local tracking branch refs: "refs/heads/<name>" from local branches
            let local_tracking: std::collections::HashSet<String> = candidates
                .iter()
                .chain(protected_branches.iter().filter(|name| local_names.contains(name)))
                .map(|name| format!("{}/{}", remote_name, name))
                .collect();

            remote_branch_names
                .into_iter()
                .filter(|rb| {
                    // Skip HEAD pointer
                    !rb.starts_with("HEAD")
                })
                .map(|rb| {
                    // rb is like "origin/some-branch" or "github/some-branch"
                    // Split at first "/" to separate remote name and branch name
                    if let Some(pos) = rb.find('/') {
                        let rem = rb[..pos].to_string();
                        let bn = rb[pos + 1..].to_string();
                        (rem, bn)
                    } else {
                        (remote_name.clone(), rb)
                    }
                })
                .filter(|(rem, bn)| {
                    // Skip if this branch is a protected branch name
                    let is_protected = protected_branches.contains(&bn.as_str());
                    // Skip if there's a local branch that tracks this remote branch
                    let is_local_tracked = local_tracking.contains(&format!("{}/{}", rem, bn));
                    !is_protected && !is_local_tracked
                })
                .collect()
        } else {
            vec![]
        };

        Ok(BranchCleanContext {
            merged_to_protected,
            unmerged_to_protected,
            remote_orphan_branches,
            force_delete_unmerged: self.force_delete_unmerged,
            remote_name,
        })
    }

    fn plan(&self, ctx: &BranchCleanContext, repo_path: &Path) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);

        // Build the list of branches to delete
        let mut to_delete: Vec<String> = vec![];

        // C类: 已合入 protected 的分支 — 始终清理
        to_delete.extend(ctx.merged_to_protected.clone());

        // B类: 未合入 protected 的分支 — 只在 -D 时清理
        if ctx.force_delete_unmerged {
            to_delete.extend(ctx.unmerged_to_protected.clone());
        }

        if to_delete.is_empty() {
            if ctx.force_delete_unmerged {
                plan.add_message(DisplayMessage::Skip {
                    msg: "没有需要清理的分支".to_string(),
                });
            } else {
                plan.add_message(DisplayMessage::Skip {
                    msg: "没有需要清理的已合入分支 (C类). 使用 -D 可清理未合入分支 (B类)".to_string(),
                });
            }
            return Ok(plan);
        }

        let mut clean_phase = if ctx.force_delete_unmerged {
            Phase::new("清理所有非受保护分支 (B类 + C类)")
        } else {
            Phase::new("清理已合入受保护分支 (C类)")
        };

        for branch in &to_delete {
            clean_phase.add(GitOperation::DeleteBranch {
                branch: branch.clone(),
                working_dir: repo_path.to_path_buf(),
            });
            clean_phase.add(GitOperation::DeleteRemoteBranch {
                remote: ctx.remote_name.clone(),
                branch: branch.clone(),
                working_dir: repo_path.to_path_buf(),
            });
        }
        plan.add_phase(clean_phase);

        // 同步 remote: 清理远端已经不存在的跟踪分支 (git remote prune)
        let mut prune_phase = Phase::new("同步远端跟踪分支");
        prune_phase.add(GitOperation::PruneRemote {
            remote: ctx.remote_name.clone(),
            working_dir: repo_path.to_path_buf(),
        });
        plan.add_phase(prune_phase);

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
