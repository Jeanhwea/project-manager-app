use crate::commands::MultiRepo;
use crate::commands::RepoPathArgs;
use crate::domain::git::GitCommandRunner;
use crate::domain::git::GitOperation;
use crate::domain::git::collect_context;
use crate::engine::plan;
use crate::error::Result;
use crate::model::git::{Branch, GitContext};
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase};
use std::collections::{HashMap, HashSet};
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

/// A 类保护分支，本地分支与远端跟踪引用都永不删除
const PROTECTED_BRANCHES: [&str; 4] = ["master", "main", "develop", "dev"];

fn is_protected(branch: &str) -> bool {
    PROTECTED_BRANCHES.contains(&branch)
}

/// 解析 git branch / git branch --merged 的一行输出，
/// 剥离 `*`（当前分支）与 `+`（已在其它 worktree 检出）标记，
/// 返回 (分支名, 是否被其它 worktree 占用)；detached HEAD 等非分支行返回 None
fn parse_local_branch_line(line: &str) -> Option<(String, bool)> {
    let line = line.trim();
    let (marker, name) = match line.split_once(' ') {
        Some((mark, rest)) if mark == "*" || mark == "+" => (mark, rest.trim()),
        _ => ("", line),
    };
    if name.is_empty() || name.starts_with('(') || name.contains("->") {
        return None;
    }
    Some((name.to_string(), marker == "+"))
}

/// 读取本地真实存在的远端跟踪引用（git branch -r），返回 (remote, branch) 列表
fn list_local_tracking_refs(
    runner: &GitCommandRunner,
    repo_path: &Path,
) -> Vec<(String, String)> {
    let Ok(output) = runner.run_local(&["branch", "-r"], Some(repo_path)) else {
        return Vec::new();
    };
    output.lines().filter_map(parse_tracking_ref_line).collect()
}

/// 解析 git branch -r 的一行输出，跳过 `origin/HEAD -> origin/master` 这类符号引用
fn parse_tracking_ref_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.contains("->") {
        return None;
    }
    // 去掉 "remotes/" 前缀
    let line = if let Some(stripped) = line.strip_prefix("remotes/") {
        stripped
    } else {
        line
    };
    let (remote, branch) = line.split_once('/')?;
    if remote.is_empty() || branch.is_empty() {
        return None;
    }
    Some((remote.to_string(), branch.to_string()))
}

/// 仅当远端真实存在同名分支时才追加远端删除操作，
/// 否则 `git push <remote> --delete` 必然失败，破坏幂等性
fn add_remote_branch_deletion(
    phase: &mut Phase,
    ctx: &BranchCleanContext,
    branch: &str,
    repo_path: &Path,
) {
    if ctx.primary_remote_has(branch) {
        phase.add(GitOperation::DeleteRemoteBranch {
            remote: ctx.remote_name.clone(),
            branch: branch.to_string(),
            working_dir: repo_path.to_path_buf(),
        });
    } else {
        phase.add_message(DisplayMessage::Skip {
            msg: format!("  远端 {}/{} 不存在，跳过远端删除", ctx.remote_name, branch),
        });
    }
}

/// 结构化清理汇总，作为 footer 消息在所有 Phase 执行完之后输出
fn add_clean_summary(plan: &mut ExecutionPlan, ctx: &BranchCleanContext, delete_unmerged: bool) {
    let c_count = ctx.to_delete.len();
    let b_count = ctx.unmerged_branches.len();
    let d_count = ctx.remote_orphan_branches.len();
    let a_count = ctx.protected_branches_to_skip.len();
    let e_branches: Vec<(String, String)> = ctx
        .remote_unmerged_branches
        .iter()
        .filter(|(rem, _)| *rem != ctx.remote_name)
        .cloned()
        .collect();
    let e_count = e_branches.len();

    plan.add_footer_message(DisplayMessage::Blank);
    plan.add_footer_message(DisplayMessage::Item {
        label: "清理汇总".to_string(),
        value: format!(
            "C类已合并 {}条 | B类未合并 {}条 | A类保护 {}条 | D类孤儿 {}条 | E类远端未合并 {}条",
            c_count, b_count, a_count, d_count, e_count,
        ),
    });

    if c_count > 0 {
        plan.add_footer_message(DisplayMessage::Detail {
            label: "C 类".to_string(),
            value: format!("删除 {} 条 → {}", c_count, ctx.to_delete.join(", ")),
        });
    }
    if d_count > 0 {
        let names: Vec<String> = ctx
            .remote_orphan_branches
            .iter()
            .map(|(rem, bn)| format!("{}/{}", rem, bn))
            .collect();
        plan.add_footer_message(DisplayMessage::Detail {
            label: "D 类".to_string(),
            value: format!("清除 {} 条 → {}", d_count, names.join(", ")),
        });
    }
    if e_count > 0 {
        let names: Vec<String> = e_branches
            .iter()
            .map(|(rem, bn)| format!("{}/{}", rem, bn))
            .collect();
        plan.add_footer_message(DisplayMessage::Detail {
            label: "E 类".to_string(),
            value: format!("清除 {} 条 → {}", e_count, names.join(", ")),
        });
    }
    if b_count > 0 {
        let action = if delete_unmerged {
            "删除"
        } else {
            "保留（需 -D 强制删除）"
        };
        plan.add_footer_message(DisplayMessage::Detail {
            label: "B 类".to_string(),
            value: format!(
                "{} {} 条 → {}",
                action,
                b_count,
                ctx.unmerged_branches.join(", ")
            ),
        });
    }
    if a_count > 0 {
        plan.add_footer_message(DisplayMessage::Detail {
            label: "A 类".to_string(),
            value: format!(
                "跳过 {} 条 → {}",
                a_count,
                ctx.protected_branches_to_skip.join(", ")
            ),
        });
    }
}

/// 查询远端真实存在的分支名（git ls-remote --heads），远端不可达时返回 None
fn list_remote_heads(
    runner: &GitCommandRunner,
    repo_path: &Path,
    remote: &str,
) -> Option<HashSet<String>> {
    let output = runner
        .run_local(&["ls-remote", "--heads", remote], Some(repo_path))
        .ok()?;
    Some(output.lines().filter_map(parse_ls_remote_line).collect())
}

/// 解析 git ls-remote 的一行输出 `<sha>\trefs/heads/<branch>`，提取分支名
fn parse_ls_remote_line(line: &str) -> Option<String> {
    const PREFIX: &str = "refs/heads/";
    let pos = line.find(PREFIX)?;
    let name = line[pos + PREFIX.len()..].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
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
    /// C 类分支: 已合并到保护分支，始终删除
    to_delete: Vec<String>,
    /// B 类分支: 未合并到保护分支，仅加 -D 时删除
    unmerged_branches: Vec<String>,
    /// D 类: 本地残留的远端跟踪引用 (remote, branch)，远端分支已不存在
    remote_orphan_branches: Vec<(String, String)>,
    /// E 类: 远端存在但未合并到保护分支的跟踪引用 (remote, branch)
    remote_unmerged_branches: Vec<(String, String)>,
    /// 保护分支的跟踪引用虽已在远端消失，但按安全底线保留，仅提示
    protected_stale_refs: Vec<(String, String)>,
    /// ls-remote 不可达的 remote，其 D 类检测已降级跳过
    unreachable_remotes: Vec<String>,
    /// 主 remote 上真实存在的分支名；None 表示该 remote 不可达
    primary_remote_heads: Option<HashSet<String>>,
    remote_name: String,
    /// A 类分支: 保护分支，永远不删（仅显示提示）
    protected_branches_to_skip: Vec<String>,
    /// 在其它 worktree 中已检出、无法删除的分支
    worktree_locked_branches: Vec<String>,
    local_branch_names: HashSet<String>,
    /// 所有 remote 的远端分支列表（用于 E 类判断远端是否存在）
    all_remote_heads: HashMap<String, HashSet<String>>,
}

impl BranchCleanContext {
    /// 主 remote 上是否真实存在该分支；不可达时按"不存在"处理，避免必然失败的 push --delete
    fn primary_remote_has(&self, branch: &str) -> bool {
        self.primary_remote_heads
            .as_ref()
            .is_some_and(|heads| heads.contains(branch))
    }
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

        let runner = GitCommandRunner::new();

        let local_branch_names: HashSet<String> = git_ctx
            .local_branches()
            .iter()
            .map(|b| b.name.clone())
            .collect();

        // A 类: 仓库中真实存在的保护分支，永远不删
        let protected_branches_to_skip: Vec<String> = PROTECTED_BRANCHES
            .iter()
            .filter(|name| local_branch_names.contains(**name))
            .map(|s| s.to_string())
            .collect();

        // 在其它 worktree 中已检出的分支（git branch 输出的 `+` 标记）删不掉，提前排除
        let worktree_locked: HashSet<String> =
            match runner.run_local(&["branch", "--list"], Some(repo_path)) {
                Ok(output) => output
                    .lines()
                    .filter_map(parse_local_branch_line)
                    .filter(|(_, locked_by_worktree)| *locked_by_worktree)
                    .map(|(name, _)| name)
                    .collect(),
                Err(_) => HashSet::new(),
            };

        // 获取所有非保护、非当前、未被其它 worktree 占用的本地分支
        let candidates: Vec<String> = git_ctx
            .local_branches()
            .iter()
            .map(|b| b.name.clone())
            .filter(|name| !is_protected(name))
            .filter(|name| *name != git_ctx.current_branch)
            .filter(|name| !worktree_locked.contains(name))
            .collect();

        let worktree_locked_branches: Vec<String> = {
            let mut names: Vec<String> = worktree_locked
                .iter()
                .filter(|name| !is_protected(name))
                .filter(|name| **name != git_ctx.current_branch)
                .cloned()
                .collect();
            names.sort();
            names
        };

        // 合并判定目标取「仓库中真实存在的全部保护分支」，而不是硬编码 master/dev，
        // 否则以 main / develop 为主干的仓库永远算不出 C 类
        let merged: HashSet<String> = {
            let mut merged_set = HashSet::new();
            for target in &protected_branches_to_skip {
                let Ok(output) =
                    runner.run_local(&["branch", "--merged", target.as_str()], Some(repo_path))
                else {
                    continue;
                };
                for (name, _) in output.lines().filter_map(parse_local_branch_line) {
                    if name != *target {
                        merged_set.insert(name);
                    }
                }
            }
            merged_set
        };

        // C 类: 已合并到保护分支 → 始终删除
        let to_delete: Vec<String> = candidates
            .iter()
            .filter(|name| merged.contains(name.as_str()))
            .cloned()
            .collect();

        // B 类: 未合并到保护分支 → 仅加 -D 时删除
        let unmerged_branches: Vec<String> = candidates
            .iter()
            .filter(|name| !merged.contains(name.as_str()))
            .cloned()
            .collect();

        /// 获取所有非保护远端分支的 merge 状态（相对于本地保护分支）
        /// 使用 `git branch -r --merged <ref>` 检查远程跟踪引用是否已合并到某个保护分支
        fn list_merged_remote_refs(
            runner: &GitCommandRunner,
            repo_path: &Path,
            protected_branches: &[String],
        ) -> HashSet<String> {
            let mut merged_refs = HashSet::new();
            for pb in protected_branches {
                let Ok(output) =
                    runner.run_local(&["branch", "-r", "--merged", pb.as_str()], Some(repo_path))
                else {
                    continue;
                };
                for line in output.lines() {
                    if let Some((_, branch)) = parse_tracking_ref_line(line) {
                        merged_refs.insert(branch);
                    }
                }
            }
            merged_refs
        }

        // D 类: 本地残留的远端跟踪引用 —— 本地还有 refs/remotes/<remote>/<branch>，
        // 但 ls-remote 已经查不到对应的远端分支。
        // git branch -d -r 只能删除本地真实存在的引用，所以候选集必须以 git branch -r 为准，
        // 而不是以 ls-remote 返回的远端分支为准（后者是"远端有、本地无"，方向正好相反）。
        let local_tracking_refs = list_local_tracking_refs(&runner, repo_path);

        // 逐个 remote 查询远端真实分支列表；primary remote 也要查，
        // 因为 C/B 类是否需要 push --delete 取决于远端分支是否真的存在
        let mut remotes_to_check: Vec<String> =
            local_tracking_refs.iter().map(|(r, _)| r.clone()).collect();
        remotes_to_check.push(remote_name.clone());
        remotes_to_check.sort();
        remotes_to_check.dedup();

        let mut remote_heads: HashMap<String, HashSet<String>> = HashMap::new();
        let mut unreachable_remotes: Vec<String> = Vec::new();
        for rem in &remotes_to_check {
            match list_remote_heads(&runner, repo_path, rem) {
                Some(heads) => {
                    remote_heads.insert(rem.clone(), heads);
                }
                None => unreachable_remotes.push(rem.clone()),
            }
        }

        let mut remote_orphan_branches: Vec<(String, String)> = Vec::new();
        let mut protected_stale_refs: Vec<(String, String)> = Vec::new();
        for (rem, branch) in &local_tracking_refs {
            // remote 不可达时无法判断，保持原样（降级）
            let Some(heads) = remote_heads.get(rem) else {
                continue;
            };
            if heads.contains(branch) {
                continue;
            }
            if is_protected(branch) {
                protected_stale_refs.push((rem.clone(), branch.clone()));
                continue;
            }
            remote_orphan_branches.push((rem.clone(), branch.clone()));
        }
        remote_orphan_branches.sort();
        remote_orphan_branches.dedup();
        protected_stale_refs.sort();
        protected_stale_refs.dedup();

        let primary_remote_heads = remote_heads.remove(&remote_name);

        // E 类: 远端存在但未合并到保护分支的远程跟踪引用
        // 这些分支在远端存在（非孤儿），但未合并到任何本地保护分支，因此属于"无用分支"。
        // 清理时会同时删除远端分支和本地跟踪引用。
        let merged_remote_refs =
            list_merged_remote_refs(&runner, repo_path, &protected_branches_to_skip);
        let mut remote_unmerged_branches: Vec<(String, String)> = Vec::new();
        for (rem, branch) in &local_tracking_refs {
            // 跳过找不到远端的情况
            let Some(heads) = remote_heads.get(rem) else {
                continue;
            };
            if !heads.contains(branch) {
                continue; // 远端已不存在，属于 D 类
            }
            if is_protected(branch) {
                continue;
            }
            if merged_remote_refs.contains(branch) {
                continue; // 已合并到保护分支，无需清理
            }
            // 跳过当前分支
            if git_ctx.current_branch == *branch {
                continue;
            }
            remote_unmerged_branches.push((rem.clone(), branch.clone()));
        }
        remote_unmerged_branches.sort();
        remote_unmerged_branches.dedup();

        Ok(BranchCleanContext {
            to_delete,
            unmerged_branches,
            remote_orphan_branches,
            remote_unmerged_branches,
            protected_stale_refs,
            unreachable_remotes,
            primary_remote_heads,
            all_remote_heads: remote_heads,
            remote_name,
            protected_branches_to_skip,
            worktree_locked_branches,
            local_branch_names,
        })
    }

    fn plan(&self, ctx: &BranchCleanContext, repo_path: &Path) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);

        let to_delete = &ctx.to_delete;
        let unmerged = &ctx.unmerged_branches;
        let clean_unmerged = self.delete_unmerged && !unmerged.is_empty();

        // === 前置提示 ===
        // remote 不可达时 D 类检测降级跳过，但不影响 C/B 类清理
        for rem in &ctx.unreachable_remotes {
            plan.add_message(DisplayMessage::Warning {
                msg: format!("remote {} 不可达，已跳过其 D 类孤儿分支检测", rem),
            });
        }
        // A 类: 保护分支，显示已跳过
        for a in &ctx.protected_branches_to_skip {
            plan.add_message(DisplayMessage::Skip {
                msg: format!("  [A 类 - 跳过] {}", a),
            });
        }
        // 安全底线: 保护分支的跟踪引用即使远端已删除也保留
        for (rem, branch) in &ctx.protected_stale_refs {
            plan.add_message(DisplayMessage::Skip {
                msg: format!(
                    "  [A 类 - 跳过] {}/{} 远端已删除，保护分支跟踪引用仍保留",
                    rem, branch
                ),
            });
        }
        for name in &ctx.worktree_locked_branches {
            plan.add_message(DisplayMessage::Skip {
                msg: format!("  [跳过] {} 已在其它 worktree 检出，无法删除", name),
            });
        }

        // === 确认提示：D 类分支数量超过阈值（10条）===
        let d_class_count = ctx.remote_orphan_branches.len();
        let mut clean_orphans = d_class_count > 0;
        if clean_orphans && !self.dry_run && !self.yes && d_class_count > 10 {
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
            clean_orphans = match try_readline(&confirm_msg) {
                Some(line) => matches!(line.trim().to_lowercase().as_str(), "y" | "yes"),
                None => false,
            };
            if !clean_orphans {
                plan.add_message(DisplayMessage::Skip {
                    msg: format!("用户取消：D 类 {} 条孤儿分支未清理", d_class_count),
                });
            }
        }

        let has_work = !to_delete.is_empty() || clean_unmerged || clean_orphans;
        if !has_work {
            if to_delete.is_empty() {
                plan.add_message(DisplayMessage::Skip {
                    msg: "没有 C 类分支（已合并分支）需要清理".to_string(),
                });
            }
            if !unmerged.is_empty() && !self.delete_unmerged {
                plan.add_message(DisplayMessage::Skip {
                    msg: format!(
                        "B 类分支（未合并分支）共 {} 个，使用 -D 可同时清理",
                        unmerged.len()
                    ),
                });
                for b in unmerged {
                    plan.add_message(DisplayMessage::Skip {
                        msg: format!("  [B 类 - 跳过] {}", b),
                    });
                }
            }
            add_clean_summary(&mut plan, ctx, self.delete_unmerged);
            return Ok(plan);
        }

        // 当只有 D 类工作时，显示提示信息
        if to_delete.is_empty() && !clean_unmerged {
            plan.add_message(DisplayMessage::Skip {
                msg: "没有本地分支需要清理，仅清理远端孤儿分支 (D类)".to_string(),
            });
        }
        if !unmerged.is_empty() && !self.delete_unmerged {
            plan.add_message(DisplayMessage::Skip {
                msg: format!(
                    "B 类分支（未合并分支）共 {} 个，使用 -D 可同时清理",
                    unmerged.len()
                ),
            });
        }

        // 清理类操作彼此独立且幂等，单条失败（引用已不存在、远端拒绝等）不应中断后续清理
        // C 类分支: 始终清理
        if !to_delete.is_empty() {
            let mut clean_phase =
                Phase::new("清理 C 类分支（已合并到保护分支）").continue_on_error();
            for branch in to_delete {
                clean_phase.add(GitOperation::DeleteBranch {
                    branch: branch.clone(),
                    working_dir: repo_path.to_path_buf(),
                });
                add_remote_branch_deletion(&mut clean_phase, ctx, branch, repo_path);
            }
            plan.add_phase(clean_phase);
        }

        // B 类分支: 仅当加 -D 时清理（使用强制删除 -D，因为 B 类分支未合并到保护分支）
        if clean_unmerged {
            let mut unmerged_phase =
                Phase::new("清理 B 类分支（未合并到保护分支）").continue_on_error();
            for branch in unmerged {
                unmerged_phase.add(GitOperation::DeleteBranchForce {
                    branch: branch.clone(),
                    working_dir: repo_path.to_path_buf(),
                });
                add_remote_branch_deletion(&mut unmerged_phase, ctx, branch, repo_path);
            }
            plan.add_phase(unmerged_phase);
        }

        // D 类: 本地残留的远端跟踪引用（远端分支已删除）
        // 不使用 git remote prune —— 它会无差别删除包括保护分支在内的全部 stale ref，
        // 这里逐条删除才能保证保护分支的跟踪引用不被清掉
        if clean_orphans {
            let mut orphan_phase = Phase::new("清理远端孤儿分支 (D类)").continue_on_error();
            for (remote, branch) in &ctx.remote_orphan_branches {
                // 判断依据紧跟在对应命令之前输出
                let local_state = if ctx.local_branch_names.contains(branch) {
                    "本地分支仍存在"
                } else {
                    "本地分支不存在"
                };
                orphan_phase.add_message(DisplayMessage::Detail {
                    label: "D 类".to_string(),
                    value: format!(
                        "{}/{} → ls-remote 未返回，{}，分类为 D 类孤儿分支",
                        remote, branch, local_state
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

        // E 类: 远端未合并分支（非主 remote 上还活着的分支，远端存在但未合并到保护分支）
        // 这些分支在远端还活着，但已无用（非主 remote 上的老旧分支），需要清理远端分支本身及其跟踪引用
        let e_branches: Vec<(String, String)> = ctx
            .remote_unmerged_branches
            .iter()
            .filter(|(rem, _)| *rem != ctx.remote_name)
            .cloned()
            .collect();
        if !e_branches.is_empty() {
            let mut e_phase = Phase::new("清理 E 类分支（远端未合并分支）").continue_on_error();
            for (remote, branch) in &e_branches {
                let remote_exists = ctx
                    .all_remote_heads
                    .get(remote)
                    .is_some_and(|heads| heads.contains(branch));
                if remote_exists {
                    e_phase.add_message(DisplayMessage::Detail {
                        label: "E 类".to_string(),
                        value: format!(
                            "{}/{} → 远端存在且未合并，删除远端分支及跟踪引用",
                            remote, branch
                        ),
                    });
                    e_phase.add(GitOperation::DeleteRemoteBranch {
                        remote: remote.clone(),
                        branch: branch.clone(),
                        working_dir: repo_path.to_path_buf(),
                    });
                } else {
                    e_phase.add_message(DisplayMessage::Detail {
                        label: "E 类".to_string(),
                        value: format!("{}/{} → 远端不存在，仅删除本地跟踪引用", remote, branch),
                    });
                }
                e_phase.add(GitOperation::DeleteRemoteTrackingBranch {
                    remote: remote.clone(),
                    branch: branch.clone(),
                    working_dir: repo_path.to_path_buf(),
                });
            }
            plan.add_phase(e_phase);
        }

        add_clean_summary(&mut plan, ctx, self.delete_unmerged);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_branch_line() {
        assert_eq!(
            parse_local_branch_line("  feat/login"),
            Some(("feat/login".to_string(), false))
        );
    }

    #[test]
    fn strips_current_branch_marker() {
        assert_eq!(
            parse_local_branch_line("* master"),
            Some(("master".to_string(), false))
        );
    }

    #[test]
    fn flags_worktree_checked_out_branch() {
        assert_eq!(
            parse_local_branch_line("+ feat/wip"),
            Some(("feat/wip".to_string(), true))
        );
    }

    #[test]
    fn ignores_detached_head_line() {
        assert_eq!(
            parse_local_branch_line("* (HEAD detached at abc1234)"),
            None
        );
        assert_eq!(parse_local_branch_line(""), None);
    }

    #[test]
    fn splits_tracking_ref_on_first_slash_only() {
        assert_eq!(
            parse_tracking_ref_line("  origin/shen/POC一致性校验"),
            Some(("origin".to_string(), "shen/POC一致性校验".to_string()))
        );
    }

    #[test]
    fn ignores_symbolic_tracking_ref_for_any_remote() {
        assert_eq!(
            parse_tracking_ref_line("  origin/HEAD -> origin/master"),
            None
        );
        assert_eq!(
            parse_tracking_ref_line("  gitana/HEAD -> gitana/master"),
            None
        );
    }

    #[test]
    fn extracts_branch_name_from_ls_remote_line() {
        assert_eq!(
            parse_ls_remote_line("a1b2c3d\trefs/heads/feat/shen-POC"),
            Some("feat/shen-POC".to_string())
        );
        assert_eq!(parse_ls_remote_line("a1b2c3d\trefs/tags/v1.0.0"), None);
    }
}
