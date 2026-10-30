//! 批量同步目录下的 Git 仓库：`pma sync` 对每个仓库依次做
//! 「可选地 checkout 其它分支并 pull → pull 当前分支 → fetch tags → push 分支与 tag」。
//! 是否推送取决于 remote 类型与配置；仓库之间可由 `-j` 并发执行，单个仓库由 `--timeout` 限时。

use crate::commands::MultiRepo;
use crate::commands::RepoPathArgs;
use crate::domain::config::ConfigManager;
use crate::domain::git::GitOperation;
use crate::domain::git::collect_context;
use crate::domain::git::resolve_remote_name;
use crate::engine::plan;
use crate::error::{AppError, Result};
use crate::model::git::{GitContext, Remote};
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase};
use std::path::Path;

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

        Ok(SyncContext {
            git_ctx,
            target_remotes,
            should_push,
            sync_all_branches: self.all_branches,
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
                let mut branch_phase = Phase::new("同步其他分支");

                // 外层是分支、内层是 remote：多个 remote 都有该分支时会重复 checkout + pull，
                // 代价可接受，换来的是每个 remote 都能按自己的 tracking 关系更新
                for branch in &other_branches {
                    for remote in &ctx.target_remotes {
                        if ctx.git_ctx.has_remote_branch(remote, &branch.name) {
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
                                msg: format!("跳过 {} (远程无此分支)", branch.name),
                            });
                        }
                    }
                }

                // 收尾切回进入时的分支：branch_phase 里 checkout 过一串分支
                branch_phase.add(GitOperation::Checkout {
                    ref_name: current_branch.clone(),
                    working_dir: repo_path.to_path_buf(),
                });

                if !branch_phase.is_empty() {
                    plan.add_phase(branch_phase);
                }
            }
        }

        let mut pull_phase = Phase::new("拉取当前分支");
        for remote in &ctx.target_remotes {
            if ctx.git_ctx.has_remote_branch(remote, current_branch) {
                pull_phase.add(GitOperation::Pull {
                    remote: remote.clone(),
                    branch: current_branch.clone(),
                    working_dir: repo_path.to_path_buf(),
                });
            } else {
                pull_phase.add_message(DisplayMessage::Skip {
                    msg: format!("跳过拉取 {}/{} (远程无此分支)", remote, current_branch),
                });
            }
        }

        // 兜底再拉一次：不带 remote/branch，完全按 git 自己的 upstream 配置走，
        // 这样即使上面的 has_remote_branch 判断为假（例如 tracking 关系未保存进快照）也能更新当前分支
        pull_phase.add(GitOperation::PullDefault {
            working_dir: repo_path.to_path_buf(),
        });

        if !pull_phase.is_empty() {
            plan.add_phase(pull_phase);
        }

        let mut tag_phase = Phase::new("拉取标签");
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
            let mut push_phase = Phase::new("推送");
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
        plan::run_plan_with_deadline(plan, deadline)
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
            jobs: 1,
            timeout,
        }
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
}
