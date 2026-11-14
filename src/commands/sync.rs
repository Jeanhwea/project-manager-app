//! 批量同步目录下的 Git 仓库：`pma sync` 对每个仓库依次做
//! 「可选地 checkout 其它分支并 pull → pull 当前分支 → fetch tags → push 分支与 tag」。
//! 是否推送取决于 remote 类型与配置；仓库之间可由 `-j` 并发执行，单个仓库由 `--timeout` 限时。
//!
//! 拉取前先用 `git ls-remote --heads` 取远端真实分支表，避免本地残留的 tracking ref
//! 让已在远端清理的分支生成必然失败的 pull；各阶段默认 continue_on_error，
//! 单点失败降级为告警而非中断整次同步，`--fail-fast` 可恢复严格模式。

use crate::commands::MultiRepo;
use crate::commands::RepoPathArgs;
use crate::domain::config::ConfigManager;
use crate::domain::git::GitCommandRunner;
use crate::domain::git::GitOperation;
use crate::domain::git::collect_context;
use crate::domain::git::list_remote_heads;
use crate::domain::git::resolve_remote_name;
use crate::engine::plan;
use crate::error::{AppError, Result};
use crate::model::git::{GitContext, Remote};
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase, Step};
use crate::utils::output;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// 跳过拉取类消息的统一前缀，汇总时据此与"跳过推送"等其它跳过区分开
const SKIP_PULL_TAG: &str = "跳过拉取";

/// `pma sync` 的参数集合
#[derive(Debug, clap::Args)]
pub struct SyncArgs {
    #[command(flatten)]
    pub repo_path: RepoPathArgs,
    #[arg(long, short, help = "Target remote name (e.g. origin, upstream)")]
    pub remote: Option<String>,
    /// 未指定 `--remote` 时的取值开关；当前实现里 true / false 行为一致，都退化为全部 remote
    #[arg(
        long,
        short = 'A',
        default_value = "true",
        help = "Push to all remotes when no remote is specified"
    )]
    pub all_remotes: bool,
    #[arg(
        long,
        short = 'a',
        default_value = "false",
        help = "Sync all branches before pulling (pull current branch first)"
    )]
    pub all_branches: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show commands without executing"
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Abort the whole sync on the first failed git operation instead of skipping it"
    )]
    pub fail_fast: bool,
    #[arg(
        long,
        short,
        default_value = "1",
        help = "Parallel jobs across repositories: 1 = serial, 0 = auto (CPU threads), N >= 2 = N parallel"
    )]
    pub jobs: usize,
    #[arg(
        long,
        default_value = "30",
        help = "Per-repository sync timeout in minutes (0 = no timeout)"
    )]
    pub timeout: u64,
}

#[derive(Debug)]
pub(crate) struct SyncContext {
    git_ctx: GitContext,
    /// 本次要 pull / push 的 remote 列表；空表示无 remote 可操作
    target_remotes: Vec<String>,
    /// true 表示其中至少一个 remote 允许推送
    should_push: bool,
    /// `-a` 打开时为 true：除当前分支外的本地分支也逐个 checkout 后 pull
    sync_all_branches: bool,
    /// 远端真实存在的分支表；只收录 ls-remote 成功的 remote，查不到的回退到本地跟踪引用
    live_remote_branches: HashMap<String, HashSet<String>>,
}

impl SyncContext {
    /// 该 remote 的远端分支表是否成功获取，决定跳过原因的文案
    fn has_live_view(&self, remote: &str) -> bool {
        self.live_remote_branches.contains_key(remote)
    }

    /// 远端是否真的还有该分支；远端不可达时退回本地 tracking ref 快照
    fn has_live_remote_branch(&self, remote: &str, branch: &str) -> bool {
        match self.live_remote_branches.get(remote) {
            Some(heads) => heads.contains(branch),
            None => self.git_ctx.has_remote_branch(remote, branch),
        }
    }
}

impl MultiRepo for SyncArgs {
    type Context = SyncContext;
    type Plan = ExecutionPlan;

    fn collect(&self, repo_path: &Path) -> Result<SyncContext> {
        let git_ctx = collect_context(repo_path)?;

        if git_ctx.remotes.is_empty() {
            return Ok(SyncContext {
                git_ctx,
                target_remotes: vec![],
                should_push: false,
                sync_all_branches: self.all_branches,
                live_remote_branches: HashMap::new(),
            });
        }

        let target_remotes =
            resolve_target_remotes(&git_ctx, self.remote.as_deref(), self.all_remotes)?;

        let target_remote_objs: Vec<&Remote> = target_remotes
            .iter()
            .filter_map(|name| git_ctx.remotes.iter().find(|r| &r.name == name))
            .collect();

        // 只要有一个 target remote 值得推就进入 push 阶段，不可推的 remote 在下面逐条跳过，
        // 这样单 remote 仓库不会因为没有 push 操作而丢失 warning 说明
        let should_push = !target_remote_objs.is_empty()
            && target_remote_objs
                .iter()
                .any(|remote| should_push_to_remote(remote));

        // 每个 remote 只查一次真实分支表；失败时留空位，plan 阶段回退到本地 tracking ref 快照
        let runner = GitCommandRunner::new();
        let mut live_remote_branches = HashMap::new();
        for name in &target_remotes {
            match list_remote_heads(&runner, repo_path, name) {
                Some(heads) => {
                    live_remote_branches.insert(name.clone(), heads);
                }
                None => output::warning(&format!(
                    "无法查询远端 {} 的分支列表，回退到本地跟踪引用",
                    name
                )),
            }
        }

        Ok(SyncContext {
            git_ctx,
            target_remotes,
            should_push,
            sync_all_branches: self.all_branches,
            live_remote_branches,
        })
    }

    fn plan(&self, ctx: &SyncContext, repo_path: &Path) -> Result<ExecutionPlan> {
        // 没有 remote 的仓库（或未解析出 target remote）不做任何网络操作
        if ctx.git_ctx.remotes.is_empty() || ctx.target_remotes.is_empty() {
            return skip_plan("无远程仓库");
        }

        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);
        let current_branch = &ctx.git_ctx.current_branch;

        if ctx.sync_all_branches {
            let other_branches: Vec<&crate::model::git::Branch> = ctx
                .git_ctx
                .local_branches()
                .iter()
                .filter(|b| b.name != current_branch.as_str())
                .cloned()
                .collect();

            if !other_branches.is_empty() {
                let mut branch_phase = sync_phase("同步其他分支", self.fail_fast);

                // 外层是分支、内层是 remote：多个 remote 都有该分支时会重复 checkout + pull，
                // 代价可接受，换来的是每个 remote 都能按自己的 tracking 关系更新
                for branch in &other_branches {
                    for remote in &ctx.target_remotes {
                        if ctx.has_live_remote_branch(remote, &branch.name) {
                            branch_phase.add(GitOperation::Checkout {
                                ref_name: branch.name.clone(),
                                working_dir: repo_path.to_path_buf(),
                            });
                            branch_phase.add(GitOperation::Pull {
                                remote: remote.clone(),
                                branch: branch.name.clone(),
                                working_dir: repo_path.to_path_buf(),
                            });
                        } else {
                            branch_phase.add_message(DisplayMessage::Skip {
                                msg: skip_pull_message(ctx, remote, &branch.name),
                            });
                        }
                    }
                }

                // 收尾切回进入时的分支：branch_phase 里 checkout 过一串分支，
                // 即使中途有 pull 被跳过或降级失败，这一步也必须执行
                branch_phase.add(GitOperation::Checkout {
                    ref_name: current_branch.clone(),
                    working_dir: repo_path.to_path_buf(),
                });

                if !branch_phase.is_empty() {
                    plan.add_phase(branch_phase);
                }
            }
        }

        let mut pull_phase = sync_phase("拉取当前分支", self.fail_fast);
        for remote in &ctx.target_remotes {
            if ctx.has_live_remote_branch(remote, current_branch) {
                pull_phase.add(GitOperation::Pull {
                    remote: remote.clone(),
                    branch: current_branch.clone(),
                    working_dir: repo_path.to_path_buf(),
                });
            } else {
                pull_phase.add_message(DisplayMessage::Skip {
                    msg: skip_pull_message(ctx, remote, current_branch),
                });
            }
        }

        // 兜底再拉一次：不带 remote/branch，完全按 git 自己的 upstream 配置走，
        // 这样即使上面的判断为假（例如 tracking 关系未保存进快照）也能更新当前分支
        pull_phase.add(GitOperation::PullDefault {
            working_dir: repo_path.to_path_buf(),
        });

        if !pull_phase.is_empty() {
            plan.add_phase(pull_phase);
        }

        let mut tag_phase = sync_phase("拉取标签", self.fail_fast);
        for remote in &ctx.target_remotes {
            tag_phase.add(GitOperation::FetchTags {
                remote: remote.clone(),
                working_dir: repo_path.to_path_buf(),
            });
        }
        if !tag_phase.is_empty() {
            plan.add_phase(tag_phase);
        }

        if ctx.should_push {
            let mut push_phase = sync_phase("推送", self.fail_fast);
            for remote_name in &ctx.target_remotes {
                let remote_obj = ctx.git_ctx.remotes.iter().find(|r| &r.name == remote_name);
                if let Some(remote) = remote_obj
                    && !should_push_to_remote(remote)
                {
                    push_phase.add_message(DisplayMessage::Skip {
                        msg: skip_push_reason(remote),
                    });
                    continue;
                }
                push_phase.add(GitOperation::PushAll {
                    remote: remote_name.clone(),
                    working_dir: repo_path.to_path_buf(),
                });
                push_phase.add(GitOperation::PushTags {
                    remote: remote_name.clone(),
                    working_dir: repo_path.to_path_buf(),
                });
            }
            if !push_phase.is_empty() {
                plan.add_phase(push_phase);
            }
        } else {
            for remote_name in &ctx.target_remotes {
                let remote_obj = ctx.git_ctx.remotes.iter().find(|r| &r.name == remote_name);
                let msg = match remote_obj {
                    Some(remote) => skip_push_reason(remote),
                    None => format!("跳过推送到 {}", remote_name),
                };
                plan.add_message(DisplayMessage::Skip { msg });
            }
        }

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        // deadline 在每次 execute 开始时现算，所以限时是针对单个仓库的，不会被并发的其它仓库吃满
        let deadline = self
            .timeout_duration()
            .map(|timeout| std::time::Instant::now() + timeout);
        let result = plan::run_plan_with_deadline(plan, deadline)?;

        let skipped = count_skipped_pulls(plan);
        if plan.dry_run() {
            output::info(&format!(
                "[DRY] 同步汇总: 跳过 {} 条拉取 (远端无对应分支)",
                skipped
            ));
        } else {
            output::info(&format!(
                "同步汇总: 完成 {} 条操作 · 跳过 {} 条拉取 (远端无对应分支) · 失败降级 {} 条",
                result.executed_count(),
                skipped,
                result.warning_count()
            ));
        }

        Ok(result)
    }
}

/// 批量同步入口；`jobs` 直接透传给遍历骨架决定并发度
pub fn run(args: SyncArgs) -> Result<()> {
    crate::commands::run_multi_repo_cmd_with_jobs(&args, &args.repo_path, args.jobs)
}

impl SyncArgs {
    // `--timeout`（分钟）换算成 Duration；0 表示不限时（返回 None）
    fn timeout_duration(&self) -> Option<std::time::Duration> {
        if self.timeout == 0 {
            None
        } else {
            Some(std::time::Duration::from_secs(self.timeout * 60))
        }
    }
}

fn resolve_target_remotes(
    git_ctx: &GitContext,
    explicit_remote: Option<&str>,
    all_remotes: bool,
) -> Result<Vec<String>> {
    if let Some(name) = explicit_remote {
        if !git_ctx.has_remote(name) {
            return Err(AppError::NotFound {
                resource: "远程仓库".into(),
                name: name.to_string(),
            });
        }
        return Ok(vec![name.to_string()]);
    }

    if all_remotes {
        return Ok(git_ctx.remotes.iter().map(|r| r.name.clone()).collect());
    }

    // 未指定 remote 时无论 -A 真假都退化为全部 remote：没有"首选 remote"的信息可用
    Ok(git_ctx.remotes.iter().map(|r| r.name.clone()).collect())
}

fn should_push_to_remote(remote: &Remote) -> bool {
    // GitHub 的 http/https 推送需要 token 或交互式登录，默认不推，SSH remote 不受影响
    if is_github_http_remote(remote) {
        return false;
    }

    let config = ConfigManager::load_config();

    !config
        .sync
        .skip_push_remotes
        .iter()
        .any(|r| r == &remote.name)
}

// 判定 remote 是否指向 GitHub 且走 http/https：主机名要命中配置里的 `github` rule 才算
fn is_github_http_remote(remote: &Remote) -> bool {
    let url = remote.url.as_str();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return false;
    }
    match remote.extract_host() {
        Some(host) => resolve_remote_name(&host).as_deref() == Some("github"),
        None => false,
    }
}

fn skip_push_reason(remote: &Remote) -> String {
    if is_github_http_remote(remote) {
        format!("跳过推送到 {} (GitHub http/https 协议)", remote.name)
    } else {
        format!("跳过推送到 {} (配置 skip_push_remotes)", remote.name)
    }
}

/// 同步类阶段默认容错：单条 git 失败降级为告警并继续，避免一个已清理的远端分支中断整次同步
fn sync_phase(label: &str, fail_fast: bool) -> Phase {
    let phase = Phase::new(label);
    if fail_fast {
        phase
    } else {
        phase.continue_on_error()
    }
}

/// 跳过拉取的原因：区分"远端确认已清理"与"远端不可达、仅有本地快照"
fn skip_pull_message(ctx: &SyncContext, remote: &str, branch: &str) -> String {
    let reason = if ctx.has_live_view(remote) {
        "远端已无此分支（可能已被清理）"
    } else {
        "远端不可达且本地无跟踪引用"
    };
    format!("{} {}/{} ({})", SKIP_PULL_TAG, remote, branch, reason)
}

/// 统计计划里因"远端无对应分支"而跳过的拉取条数，不含"跳过推送"等其它跳过
fn count_skipped_pulls(plan: &ExecutionPlan) -> usize {
    plan.phases()
        .iter()
        .flat_map(|phase| phase.steps())
        .filter(|step| {
            matches!(step, Step::Msg(DisplayMessage::Skip { msg }) if msg.starts_with(SKIP_PULL_TAG))
        })
        .count()
}

fn skip_plan(msg: &str) -> Result<ExecutionPlan> {
    let mut plan = ExecutionPlan::new();
    plan.add_message(DisplayMessage::Skip {
        msg: msg.to_string(),
    });
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::git::Branch;
    use std::time::Duration;

    fn sync_args(timeout: u64) -> SyncArgs {
        SyncArgs {
            repo_path: RepoPathArgs {
                max_depth: 3,
                path: ".".to_string(),
            },
            remote: None,
            all_remotes: true,
            all_branches: false,
            dry_run: false,
            fail_fast: false,
            jobs: 1,
            timeout,
        }
    }

    /// 构造一个只有 gitana、当前分支为 master 的上下文；should_push 关闭以避开配置依赖
    fn test_context(live: HashMap<String, HashSet<String>>) -> SyncContext {
        let branches = vec![
            Branch {
                name: "master".to_string(),
                is_current: true,
                is_remote: false,
                tracking_branch: None,
            },
            Branch {
                name: "fix/M-lang".to_string(),
                is_current: false,
                is_remote: false,
                tracking_branch: None,
            },
            Branch {
                name: "gitana/master".to_string(),
                is_current: false,
                is_remote: true,
                tracking_branch: None,
            },
            // 关键：远端已删掉 fix/M-lang，但本地 tracking ref 仍残留
            Branch {
                name: "gitana/fix/M-lang".to_string(),
                is_current: false,
                is_remote: true,
                tracking_branch: None,
            },
        ];
        SyncContext {
            git_ctx: GitContext {
                current_branch: "master".to_string(),
                remotes: vec![Remote {
                    name: "gitana".to_string(),
                    url: "git@example.com:org/repo.git".to_string(),
                }],
                branches,
                tags: vec![],
                has_uncommitted_changes: false,
            },
            target_remotes: vec!["gitana".to_string()],
            should_push: false,
            sync_all_branches: true,
            live_remote_branches: live,
        }
    }

    fn operation_descriptions(plan: &ExecutionPlan) -> Vec<String> {
        plan.phases()
            .iter()
            .flat_map(|phase| phase.steps())
            .filter_map(|step| match step {
                Step::Op(op) => Some(op.description()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_timeout_duration_default() {
        assert_eq!(
            sync_args(30).timeout_duration(),
            Some(Duration::from_secs(30 * 60))
        );
    }

    #[test]
    fn test_timeout_duration_zero_disables_timeout() {
        assert_eq!(sync_args(0).timeout_duration(), None);
    }

    #[test]
    fn test_timeout_duration_custom() {
        assert_eq!(
            sync_args(5).timeout_duration(),
            Some(Duration::from_secs(5 * 60))
        );
    }

    #[test]
    fn test_sync_phase_tolerance_follows_fail_fast() {
        assert!(sync_phase("拉取", false).is_continue_on_error());
        assert!(!sync_phase("拉取", true).is_continue_on_error());
    }

    #[test]
    fn test_live_view_overrides_stale_tracking_ref() {
        let mut live = HashMap::new();
        live.insert("gitana".to_string(), HashSet::from(["master".to_string()]));
        let ctx = test_context(live);
        // 本地快照说有，但远端真实分支表说没有，必须以远端为准
        assert!(ctx.git_ctx.has_remote_branch("gitana", "fix/M-lang"));
        assert!(!ctx.has_live_remote_branch("gitana", "fix/M-lang"));
        assert!(ctx.has_live_remote_branch("gitana", "master"));
    }

    #[test]
    fn test_falls_back_to_snapshot_when_remote_unreachable() {
        let ctx = test_context(HashMap::new());
        assert!(!ctx.has_live_view("gitana"));
        assert!(ctx.has_live_remote_branch("gitana", "fix/M-lang"));
    }

    #[test]
    fn test_plan_skips_pull_for_branch_deleted_on_remote() {
        let mut live = HashMap::new();
        live.insert("gitana".to_string(), HashSet::from(["master".to_string()]));
        let ctx = test_context(live);
        let plan = sync_args(30).plan(&ctx, Path::new(".")).unwrap();

        let descriptions = operation_descriptions(&plan);
        assert!(
            !descriptions
                .iter()
                .any(|d| d.contains("git pull gitana fix/M-lang"))
        );
        assert_eq!(count_skipped_pulls(&plan), 1);
        // 切回原分支的收尾步骤仍在，不会把仓库留在 fix/M-lang 上
        assert!(
            descriptions
                .iter()
                .any(|d| d.ends_with("git checkout master"))
        );
    }

    #[test]
    fn test_plan_keeps_pull_when_remote_unreachable() {
        let ctx = test_context(HashMap::new());
        let plan = sync_args(30).plan(&ctx, Path::new(".")).unwrap();

        // 降级场景按旧快照生成 pull，由 continue_on_error 的 phase 兜底
        assert!(
            operation_descriptions(&plan)
                .iter()
                .any(|d| d.contains("git pull gitana fix/M-lang"))
        );
        assert_eq!(count_skipped_pulls(&plan), 0);
    }

    #[test]
    fn test_plan_phases_are_tolerant_by_default() {
        let ctx = test_context(HashMap::new());
        let plan = sync_args(30).plan(&ctx, Path::new(".")).unwrap();
        assert!(
            plan.phases()
                .iter()
                .all(|phase| phase.is_continue_on_error())
        );
    }

    #[test]
    fn test_fail_fast_makes_all_phases_strict() {
        let ctx = test_context(HashMap::new());
        let args = SyncArgs {
            fail_fast: true,
            ..sync_args(30)
        };
        let plan = args.plan(&ctx, Path::new(".")).unwrap();
        assert!(
            plan.phases()
                .iter()
                .all(|phase| !phase.is_continue_on_error())
        );
    }

    #[test]
    fn test_skip_pull_message_explains_reason() {
        let mut live = HashMap::new();
        live.insert("gitana".to_string(), HashSet::from(["master".to_string()]));
        let ctx = test_context(live);
        assert!(skip_pull_message(&ctx, "gitana", "fix/M-lang").contains("远端已无此分支"));
        assert!(skip_pull_message(&ctx, "origin", "fix/M-lang").contains("远端不可达"));
    }
}
