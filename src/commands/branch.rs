use crate::commands::MultiRepo;
use crate::commands::RepoPathArgs;
use crate::domain::git::GitCommandRunner;
use crate::domain::git::GitOperation;
use crate::domain::git::collect_context;
use crate::engine::plan;
use crate::error::Result;
use crate::model::git::{Branch, GitContext};
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase};
use std::io::{self, Write};
use std::path::Path;

/// 从 stdin 读取一行用户输入（用于交互确认），出错或 EOF 时返回 None
fn try_readline(prompt: &str) -> Option<String> {
    print!("{}", prompt);
    let _ = io::stdout().flush();
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) => None, // EOF
        Ok(_) => {
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }
        Err(_) => None,
    }
}

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
    #[arg(
        short = 'y',
        long = "yes",
        default_value = "false",
        help = "Skip confirmation prompt for Branches Clean"
    )]
    pub yes: bool,
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
    /// A 类分支: 保护分支，永远不删（仅显示提示）
    protected_branches_to_skip: Vec<String>,
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
            .filter(|name| merged.contains(name.to_string().as_str()))
            .map(|s| s.to_string())
            .collect();

        // B 类: 未合并到 protected 分支的 → 仅加 -D 时删除
        let unmerged_branches: Vec<String> = candidates
            .iter()
            .filter(|name| !merged.contains(name.to_string().as_str()))
            .map(|s| s.to_string())
            .collect();

        // D类: 远端孤儿分支 — 本地没有跟踪分支的远端分支
        // 使用 git ls-remote 直接查询远端真实存在的分支列表，避免本地缓存过时的问题
        let runner_for_remote = GitCommandRunner::new();
        let remote_orphan_branches: Vec<(String, String)> = {
            // 遍历所有 remote，分别查询 ls-remote 获取真实的分支列表
            let all_remotes: Vec<String> = git_ctx
                .remote_names()
                .iter()
                .map(|s| s.to_string())
                .collect();
            let remotes_to_check = if all_remotes.is_empty() {
                vec![remote_name.clone()]
            } else {
                all_remotes
            };

            let mut orphan_branches: Vec<(String, String)> = Vec::new();

            for rem in &remotes_to_check {
                // 用 ls-remote 获取远端真实存在的分支列表（refs/heads/*）
                let ls_remote_output = match runner_for_remote
                    .run_local(&["ls-remote", "--heads", rem], Some(repo_path))
                {
                    Ok(output) => output,
                    Err(_) => continue,
                };

                // 从 ls-remote 输出中提取分支名
                // 输出格式: "<sha>\trefs/heads/<branch_name>"
                let remote_branch_names: Vec<String> = ls_remote_output
                    .lines()
                    .filter_map(|line| {
                        let line = line.trim();
                        if line.is_empty() {
                            return None;
                        }
                        // 提取 refs/heads/ 之后的部分
                        if let Some(pos) = line.find("refs/heads/") {
                            let name = line[pos + "refs/heads/".len()..].to_string();
                            Some(name)
                        } else {
                            None
                        }
                    })
                    .filter(|name| !name.is_empty())
                    .collect();

                // Build the set of tracking refs for ALL local branches, so any remote branch
                // that has a corresponding local branch (regardless of merge status) is excluded
                // from the "orphan" set.
                // Use the local branch's actual tracking_branch info for matching,
                // since different remotes may track different branches.
                let local_tracking: std::collections::HashSet<String> = git_ctx
                    .local_branches()
                    .iter()
                    .filter_map(|b| b.tracking_branch.clone())
                    .collect();
                // Also add the simple "{remote}/{branch}" form for backward compatibility
                // with branches that track the primary remote
                let simple_local_tracking: std::collections::HashSet<String> = git_ctx
                    .local_branches()
                    .iter()
                    .map(|b| format!("{}/{}", rem, b.name))
                    .collect();

                for bn in &remote_branch_names {
                    // Skip if this branch is a protected branch name
                    let is_protected = protected_branches.contains(&bn.as_str());
                    // Skip if there's a local branch that tracks this remote branch.
                    // tracking_branch is in the format "refs/remotes/<remote>/<branch>"
                    let tracking_ref = format!("refs/remotes/{}/{}", rem, bn);
                    let simple_ref = format!("{}/{}", rem, bn);
                    let is_local_tracked = local_tracking.contains(&tracking_ref)
                        || local_tracking.contains(&simple_ref)
                        || simple_local_tracking.contains(&simple_ref);
                    if !is_protected && !is_local_tracked {
                        orphan_branches.push((rem.clone(), bn.clone()));
                    }
                }

                // Also check for local tracking refs that have no corresponding remote branch
                // (e.g., the remote branch was deleted but the tracking ref remains)
                for tracking in &local_tracking {
                    // Extract remote name and branch name from tracking ref
                    // tracking is in format "refs/remotes/<remote>/<branch>"
                    if let Some(tracking_rem) = tracking.strip_prefix("refs/remotes/") {
                        if let Some(slash_pos) = tracking_rem.rfind('/') {
                            let rem = &tracking_rem[..slash_pos];
                            let bn = &tracking_rem[slash_pos + 1..];
                            // Check if this remote branch exists on the remote
                            if !remote_branch_names.iter().any(|rb| rb == bn) {
                                orphan_branches.push((rem.to_string(), bn.to_string()));
                            }
                        }
                    } else if let Some(slash_pos) = tracking.rfind('/') {
                        // Handle the case where tracking is in the format "remote/branch"
                        let rem = &tracking[..slash_pos];
                        let bn = &tracking[slash_pos + 1..];
                        if !remote_branch_names.iter().any(|rb| rb == bn) {
                            orphan_branches.push((rem.to_string(), bn.to_string()));
                        }
                    }
                }

                // NEW: 使用 git branch -r 获取所有本地 remote tracking refs，
                // 与 git ls-remote --heads 的结果对比，找出远端已删除但本地仍残留的 tracking refs。
                // 这样直接读 Git 命令输出，比依赖 git_ctx.branches 对象的格式更可靠
                if let Ok(branch_r_output) =
                    runner_for_remote.run_local(&["branch", "-r"], Some(repo_path))
                {
                    for line in branch_r_output.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with("origin/HEAD") {
                            continue;
                        }
                        // git branch -r 输出格式: "  origin/feat/mac-m2p"
                        // 提取 remote 和 branch 名
                        if let Some(rest) = line.strip_prefix(&format!("{}/", rem)) {
                            let bn = rest.trim();
                            // Skip protected branches
                            if protected_branches.contains(&bn) {
                                continue;
                            }
                            // Check if this remote branch actually exists on the remote
                            if !remote_branch_names.iter().any(|rb| rb == bn) {
                                orphan_branches.push((rem.clone(), bn.to_string()));
                            }
                        }
                    }
                }
            }

            orphan_branches
        };

        // Collect A类: 保护分支的 tracking ref 信息
        let protected_branches_to_skip: Vec<String> =
            protected_branches.iter().map(|s| s.to_string()).collect();

        Ok(BranchCleanContext {
            to_delete,
            unmerged_branches,
            remote_orphan_branches,
            remote_name,
            protected_branches_to_skip,
        })
    }

    fn plan(&self, ctx: &BranchCleanContext, repo_path: &Path) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);

        let to_delete = &ctx.to_delete;
        let unmerged = &ctx.unmerged_branches;

        let has_c_or_b_work =
            !to_delete.is_empty() || (!unmerged.is_empty() && self.delete_unmerged);
        let has_orphan_work = !ctx.remote_orphan_branches.is_empty();

        // === 汇总信息 ===
        // A 类: 保护分支，显示已跳过
        for a in &ctx.protected_branches_to_skip {
            plan.add_message(DisplayMessage::Skip {
                msg: format!("  [A 类 - 跳过] {}", a),
            });
        }

        if !has_c_or_b_work && !has_orphan_work {
            let mut msg = String::new();
            if to_delete.is_empty() {
                msg.push_str("没有 C 类分支（已合并分支）需要清理");
            }
            if !unmerged.is_empty() && !self.delete_unmerged {
                if !msg.is_empty() {
                    msg.push('，');
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

        // === 确认提示：D 类分支数量超过阈值（10条）===
        let d_class_count = ctx.remote_orphan_branches.len();
        if !self.dry_run && !self.yes && d_class_count > 10 {
            let d_names: Vec<String> = ctx
                .remote_orphan_branches
                .iter()
                .map(|(rem, bn)| format!("{}/{}", rem, bn))
                .collect();
            let confirm_msg = format!(
                "即将清理 {} 条 D 类（孤儿分支）\n  {}\n输入 y / yes 确认执行，输入其他任意内容取消: ",
                d_class_count,
                d_names.join(", ")
            );
            let confirmed = match try_readline(&confirm_msg) {
                Some(line) => matches!(line.trim().to_lowercase().as_str(), "y" | "yes"),
                None => false,
            };
            if !confirmed {
                plan.add_message(DisplayMessage::Skip {
                    msg: format!("用户取消：D 类 {} 条孤儿分支未清理", d_class_count),
                });
                // 跳过 D 类清理，但继续处理 C/B 类和 prune
                // return Ok(plan); // We still process C/B below
            }
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
                // 添加判断依据输出
                plan.add_message(DisplayMessage::Detail {
                    label: "D 类".to_string(),
                    value: format!(
                        "{}/{} → ls-remote 未返回，分类为 D 类孤儿分支",
                        remote, branch
                    ),
                });
                orphan_phase.add(GitOperation::DeleteRemoteTrackingBranch {
                    remote: remote.clone(),
                    branch: branch.clone(),
                    working_dir: repo_path.to_path_buf(),
                });
            }
            plan.add_phase(orphan_phase);
        }

        // 同步 remote: 清理远端已经不存在的跟踪分支 (git remote prune)
        let mut prune_phase = Phase::new("同步远端跟踪分支");
        for rem in ctx
            .remote_orphan_branches
            .iter()
            .map(|(r, _)| r)
            .collect::<std::collections::HashSet<_>>()
        {
            prune_phase.add(GitOperation::PruneRemote {
                remote: rem.clone(),
                working_dir: repo_path.to_path_buf(),
            });
        }
        // Also prune the primary remote even if no orphan branches found
        if ctx.remote_orphan_branches.is_empty() {
            prune_phase.add(GitOperation::PruneRemote {
                remote: ctx.remote_name.clone(),
                working_dir: repo_path.to_path_buf(),
            });
        }
        plan.add_phase(prune_phase);

        // === 清理结果汇总 ===
        let c_count = ctx.to_delete.len();
        let b_count = ctx.unmerged_branches.len();
        let d_count = ctx.remote_orphan_branches.len();
        let a_count = ctx.protected_branches_to_skip.len();
        plan.add_message(DisplayMessage::Blank);
        plan.add_message(DisplayMessage::Item {
            label: "清理汇总".to_string(),
            value: format!(
                "C类已合并 {}条 | B类未合并 {}条 | A类保护 {}条 | D类孤儿 {}条",
                c_count, b_count, d_count, a_count,
            ),
        });

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
