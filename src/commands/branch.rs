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
    #[command(visible_alias = "cl", visible_alias = "pr")]
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
        help = "Dry run: show what would be deleted without actually deleting"
    )]
    pub dry_run: bool,
    #[arg(
        short = 'D',
        long = "delete-unmerged",
        default_value = "false",
        help = "Also delete B-class branches (branches not merged into master/dev)"
    )]
    pub delete_unmerged: bool,
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
    /// C 类分支: 已合并到 protected 分支，始终删除
    to_delete: Vec<String>,
    /// B 类分支: 未合并到 protected 分支，仅加 -D 时删除
    unmerged_branches: Vec<String>,
    remote_orphan_branches: Vec<(String, String)>,
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

        // A 类: 保护分支，永远不删
        let protected_branches = ["master", "main", "develop", "dev"];

        // 获取所有非保护、非当前分支
        let candidates: Vec<&str> = git_ctx
            .local_branches()
            .iter()
            .map(|b| b.name.as_str())
            .filter(|name| !protected_branches.contains(name))
            .filter(|name| *name != git_ctx.current_branch)
            .collect();

        // 用 `git branch --merged` 判断哪些已合并到 master 或 dev
        let merged: std::collections::HashSet<String> = {
            let runner = GitCommandRunner::new();
            let mut merged_set = std::collections::HashSet::new();
            let check_protected = ["master", "dev"];
            for target in &check_protected {
                // git branch --merged <target> 列出已合并到 target 的所有分支
                if let Ok(output) =
                    runner.run_local(&["branch", "--merged", target], Some(repo_path))
                {
                    for line in output.lines() {
                        let name = line.trim().trim_start_matches("* ").trim();
                        if !name.is_empty() && name != *target {
                            merged_set.insert(name.to_string());
                        }
                    }
                }
            }
            merged_set
        };

        // C 类: 已合并到 protected 分支的 → 始终删除
        let to_delete: Vec<String> = candidates
            .iter()
            .filter(|name| merged.contains(&name.to_string()))
            .map(|s| s.to_string())
            .collect();

        // B 类: 未合并到 protected 分支的 → 仅加 -D 时删除
        let unmerged_branches: Vec<String> = candidates
            .iter()
            .filter(|name| !merged.contains(&name.to_string()))
            .map(|s| s.to_string())
            .collect();

        // D类: 远端孤儿分支 — 本地没有跟踪分支的远端分支
        // 先 prune 远端缓存，确保 git branch -r 是最新状态
        let prune_runner = GitCommandRunner::new();
        let _ = prune_runner.run_local(&["remote", "prune", &remote_name], Some(repo_path));

        let runner_for_remote = GitCommandRunner::new();
        let remote_branches_output =
            runner_for_remote.run_local(&["branch", "-r"], Some(repo_path))?;
        let local_names: Vec<&str> = git_ctx
            .local_branches()
            .iter()
            .map(|b| b.name.as_str())
            .collect();
        let remote_orphan_branches: Vec<(String, String)> = {
            let remote_branch_names: Vec<String> = remote_branches_output
                .lines()
                .map(|line: &str| line.trim().to_string())
                .filter(|line: &String| !line.is_empty() && !line.contains("->"))
                .collect();

            // Build a set of local tracking refs: "<remote>/<branch>" for all local branches
            let protected_strings: Vec<String> = protected_branches
                .iter()
                .filter(|name| local_names.contains(name))
                .map(|name| name.to_string())
                .collect();
            // Build the set of tracking refs for ALL local branches (not just C and protected)
            let local_tracking: std::collections::HashSet<String> = to_delete
                .iter()
                .chain(unmerged_branches.iter())
                .chain(protected_strings.iter())
                .map(|name| format!("{}/{}", remote_name, name))
                .collect();

            remote_branch_names
                .into_iter()
                .filter(|rb: &String| {
                    // Skip HEAD pointer
                    !rb.starts_with("HEAD")
                })
                .map(|rb: String| {
                    // rb is like "origin/some-branch" or "github/some-branch"
                    // Split at first "/" to separate remote name and branch name
                    if let Some(pos) = rb.find('/') {
                        let rem = rb[..pos].to_string();
                        let bn = rb[pos + 1..].to_string();
                        (rem, bn)
                    } else {
                        (remote_name.clone(), rb.clone())
                    }
                })
                .filter(|(rem, bn): &(String, String)| {
                    // Skip if this branch is a protected branch name
                    let is_protected = protected_branches.contains(&bn.as_str());
                    // Skip if there's a local branch that tracks this remote branch
                    let is_local_tracked = local_tracking.contains(&format!("{}/{}", rem, bn));
                    !is_protected && !is_local_tracked
                })
                .collect()
        };

        Ok(BranchCleanContext {
            to_delete,
            unmerged_branches,
            remote_orphan_branches,
            remote_name,
        })
    }

    fn plan(&self, ctx: &BranchCleanContext, repo_path: &Path) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);

        let to_delete = &ctx.to_delete;
        let unmerged = &ctx.unmerged_branches;

        let has_c_or_b_work =
            !to_delete.is_empty() || (!unmerged.is_empty() && self.delete_unmerged);
        let has_orphan_work = !ctx.remote_orphan_branches.is_empty();

        if !has_c_or_b_work && !has_orphan_work {
            let mut msg = String::new();
            if to_delete.is_empty() {
                msg.push_str("没有 C 类分支（已合并分支）需要清理");
            }
            if !unmerged.is_empty() && !self.delete_unmerged {
                if !msg.is_empty() {
                    msg.push_str("，");
                }
                msg.push_str(&format!(
                    "B 类分支（未合并分支）共 {} 个，使用 -D 可同时清理",
                    unmerged.len()
                ));
            }
            plan.add_message(DisplayMessage::Skip { msg });
            if !unmerged.is_empty() {
                for b in unmerged {
                    plan.add_message(DisplayMessage::Skip {
                        msg: format!("  [B 类 - 跳过] {}", b),
                    });
                }
            }
            return Ok(plan);
        }

        // 当只有 D 类工作时，显示提示信息
        if !has_c_or_b_work && has_orphan_work {
            plan.add_message(DisplayMessage::Skip {
                msg: "没有本地分支需要清理，仅清理远端孤儿分支 (D类)".to_string(),
            });
        }

        // C 类分支: 始终清理
        if !to_delete.is_empty() {
            let mut clean_phase = Phase::new("清理 C 类分支（已合并到 master/dev）");
            for branch in to_delete {
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
        }

        // B 类分支: 仅当加 -D 时清理（使用强制删除 -D，因为 B 类分支未合并到保护分支）
        if !unmerged.is_empty() && self.delete_unmerged {
            let mut unmerged_phase = Phase::new("清理 B 类分支（未合并到 master/dev）");
            for branch in unmerged {
                unmerged_phase.add(GitOperation::DeleteBranchForce {
                    branch: branch.clone(),
                    working_dir: repo_path.to_path_buf(),
                });
                unmerged_phase.add(GitOperation::DeleteRemoteBranch {
                    remote: ctx.remote_name.clone(),
                    branch: branch.clone(),
                    working_dir: repo_path.to_path_buf(),
                });
            }
            plan.add_phase(unmerged_phase);
        }

        // D类: 远端孤儿分支 — 本地没有跟踪分支的远端分支，始终清理
        if !ctx.remote_orphan_branches.is_empty() {
            let mut orphan_phase = Phase::new("清理远端孤儿分支 (D类)");
            for (remote, branch) in &ctx.remote_orphan_branches {
                orphan_phase.add(GitOperation::DeleteRemoteBranch {
                    remote: remote.clone(),
                    branch: branch.clone(),
                    working_dir: repo_path.to_path_buf(),
                });
            }
            plan.add_phase(orphan_phase);
        }

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
