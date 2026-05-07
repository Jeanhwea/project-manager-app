use super::{Command, CommandResult};
use crate::domain::context::AppContext;
use crate::domain::git::repository::{RepoWalker, find_git_repository_upwards};
use crate::utils::output::{ItemColor, Output, SummaryBuilder};
use std::path::{Path, PathBuf};

#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    /// Maximum depth to search for repositories
    #[arg(
        long,
        short,
        default_value = "3",
        help = "Maximum depth to search for repositories"
    )]
    pub max_depth: Option<usize>,
    /// Show short status (branch + clean/dirty only)
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Show short status (branch + clean/dirty only)"
    )]
    pub short: bool,
    /// Filter repositories by status
    #[arg(
        long,
        short,
        value_enum,
        help = "Filter repositories by status: dirty, clean, ahead, behind"
    )]
    pub filter: Option<StatusFilter>,
    /// Path to the directory to search for repositories, defaults to current directory
    #[arg(
        default_value = ".",
        help = "Path to the directory to search for repositories, defaults to current directory"
    )]
    pub path: String,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum StatusFilter {
    Dirty,
    Clean,
    Ahead,
    Behind,
}

struct RepoStatus {
    branch: Option<String>,
    dirty: bool,
    ahead: usize,
    behind: usize,
    staged: usize,
    unstaged: usize,
    untracked: usize,
}

#[derive(Default)]
struct StatusStats {
    total_shown: usize,
    clean_count: usize,
    dirty_count: usize,
    ahead_count: usize,
    behind_count: usize,
    submodules: usize,
    skipped: usize,
    total_staged: usize,
    total_unstaged: usize,
    total_untracked: usize,
}

impl StatusStats {
    fn update(&mut self, status: &RepoStatus) {
        self.total_shown += 1;
        if status.dirty {
            self.dirty_count += 1;
        } else {
            self.clean_count += 1;
        }
        if status.ahead > 0 {
            self.ahead_count += 1;
        }
        if status.behind > 0 {
            self.behind_count += 1;
        }
        self.total_staged += status.staged;
        self.total_unstaged += status.unstaged;
        self.total_untracked += status.untracked;
    }
}

pub struct StatusCommand;

impl Command for StatusCommand {
    type Args = StatusArgs;

    fn execute(args: Self::Args) -> CommandResult {
        let search_path = if args.path.is_empty() || args.path == "." {
            std::env::current_dir().map_err(|e| {
                super::CommandError::ExecutionFailed(format!("获取当前目录失败: {}", e))
            })?
        } else {
            PathBuf::from(&args.path)
        };

        let effective_path =
            find_git_repository_upwards(&search_path).unwrap_or_else(|| search_path.clone());

        let walker =
            RepoWalker::new(&effective_path, args.max_depth.unwrap_or(3)).map_err(|e| {
                super::CommandError::ExecutionFailed(format!("创建仓库遍历器失败: {}", e))
            })?;

        if walker.is_empty() {
            Output::not_found("未找到git仓库");
            return Ok(());
        }

        let total = walker.total();
        let mut stats = StatusStats::default();

        for (index, repo_info) in walker.repositories().iter().enumerate() {
            let repo_path = &repo_info.path;

            if repo_info.repo_type == crate::domain::git::repository::RepoType::Submodule {
                stats.submodules += 1;
                continue;
            }

            let status = collect_repo_status(repo_path);

            if !matches_filter(&status, &args.filter) {
                stats.skipped += 1;
                continue;
            }

            Output::repo_header(index + 1, total, repo_path);

            if args.short {
                print_short_status(&status);
            } else {
                print_full_status(repo_path, &status);
            }

            stats.update(&status);
        }

        print_summary(&stats, total);

        Ok(())
    }
}

fn matches_filter(status: &RepoStatus, filter: &Option<StatusFilter>) -> bool {
    match filter {
        None => true,
        Some(StatusFilter::Dirty) => status.dirty,
        Some(StatusFilter::Clean) => !status.dirty,
        Some(StatusFilter::Ahead) => status.ahead > 0,
        Some(StatusFilter::Behind) => status.behind > 0,
    }
}

fn collect_repo_status(repo_path: &Path) -> RepoStatus {
    let runner = AppContext::global().git_runner();
    let branch = runner.get_current_branch(repo_path).ok();
    let dirty = runner.has_uncommitted_changes(repo_path).unwrap_or(false);
    let (ahead, behind) = get_ahead_behind(repo_path);
    let (staged, unstaged, untracked) = get_dirty_counts(repo_path);

    RepoStatus {
        branch,
        dirty,
        ahead,
        behind,
        staged,
        unstaged,
        untracked,
    }
}

fn get_ahead_behind(repo_path: &Path) -> (usize, usize) {
    let runner = AppContext::global().git_runner();
    let branch = match runner.get_current_branch(repo_path) {
        Ok(b) => b,
        Err(_) => return (0, 0),
    };

    let upstream = format!("{}@{{upstream}}...HEAD", branch);
    let output = match runner.execute_in_dir(
        &["rev-list", "--count", "--left-right", &upstream],
        repo_path,
    ) {
        Ok(o) => o,
        Err(_) => return (0, 0),
    };

    let trimmed = output.trim();
    if trimmed.is_empty() {
        return (0, 0);
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() != 2 {
        return (0, 0);
    }

    let ahead: usize = parts[0].parse().unwrap_or(0);
    let behind: usize = parts[1].parse().unwrap_or(0);

    (ahead, behind)
}

fn get_dirty_counts(repo_path: &Path) -> (usize, usize, usize) {
    let runner = AppContext::global().git_runner();
    let output = match runner.execute_in_dir(&["status", "--porcelain"], repo_path) {
        Ok(o) => o,
        Err(_) => return (0, 0, 0),
    };

    let mut staged = 0usize;
    let mut unstaged = 0usize;
    let mut untracked = 0usize;

    for line in output.lines() {
        if line.len() < 3 {
            continue;
        }
        let x = line.as_bytes()[0];
        let y = line.as_bytes()[1];

        if x != b' ' && x != b'?' {
            staged += 1;
        }
        if y != b' ' && y != b'?' {
            unstaged += 1;
        }
        if x == b'?' && y == b'?' {
            untracked += 1;
        }
    }

    (staged, unstaged, untracked)
}

fn print_summary(stats: &StatusStats, total: usize) {
    Output::header("汇总");

    let repo_value = format!(
        "{} (显示 {}, 跳过 {}, 子模块 {})",
        total, stats.total_shown, stats.skipped, stats.submodules
    );
    Output::item("仓库", &repo_value);

    let status_value = format!("{} 干净, {} 脏", stats.clean_count, stats.dirty_count);
    Output::item_colored("状态", &status_value, ItemColor::Yellow);

    if stats.ahead_count > 0 || stats.behind_count > 0 {
        SummaryBuilder::new()
            .add(
                "同步",
                format!("{} 领先, {} 落后", stats.ahead_count, stats.behind_count),
            )
            .print();
    }

    if stats.total_staged > 0 || stats.total_unstaged > 0 || stats.total_untracked > 0 {
        SummaryBuilder::new()
            .add(
                "文件",
                format!(
                    "{} 已暂存, {} 未暂存, {} 未跟踪",
                    stats.total_staged, stats.total_unstaged, stats.total_untracked
                ),
            )
            .print();
    }
}

fn print_short_status(status: &RepoStatus) {
    let branch_display = status.branch.as_deref().unwrap_or("HEAD");

    if status.dirty {
        Output::error(&format!("{} (dirty)", branch_display));
    } else {
        Output::success(&format!("{} (clean)", branch_display));
    }

    let mut extra = Vec::new();
    if status.ahead > 0 {
        extra.push(format!("↑{}", status.ahead));
    }
    if status.behind > 0 {
        extra.push(format!("↓{}", status.behind));
    }

    if !extra.is_empty() {
        Output::message(&extra.join(" "));
    }
}

fn print_full_status(repo_path: &Path, status: &RepoStatus) {
    let runner = AppContext::global().git_runner();
    let branch_display = status.branch.as_deref().unwrap_or("HEAD (detached)");

    Output::item("分支", branch_display);

    if status.dirty {
        Output::item_colored("状态", "脏工作目录", ItemColor::Red);
        print_dirty_details_from_counts(status);
    } else {
        Output::item_colored("状态", "干净工作目录", ItemColor::Green);
    }

    print_ahead_behind_from_status(status);

    print_latest_tag(repo_path);

    let remotes = runner.get_remote_list(repo_path).unwrap_or_default();
    if !remotes.is_empty() {
        Output::message("远程:");
        for remote in &remotes {
            let url = runner
                .execute_in_dir(&["remote", "get-url", remote], repo_path)
                .unwrap_or_default();
            Output::detail(remote, &url);
        }
    }
}

fn print_dirty_details_from_counts(status: &RepoStatus) {
    let mut parts = Vec::new();
    if status.staged > 0 {
        parts.push(format!("{} 已暂存", status.staged));
    }
    if status.unstaged > 0 {
        parts.push(format!("{} 未暂存", status.unstaged));
    }
    if status.untracked > 0 {
        parts.push(format!("{} 未跟踪", status.untracked));
    }

    if !parts.is_empty() {
        Output::item("详情", &parts.join(", "));
    }
}

fn print_ahead_behind_from_status(status: &RepoStatus) {
    if status.ahead == 0 && status.behind == 0 {
        Output::item_colored("同步", "与远程一致", ItemColor::Green);
    } else {
        let mut msg = String::new();
        if status.ahead > 0 {
            msg.push_str(&format!("↑{} 领先", status.ahead));
        }
        if status.behind > 0 {
            if !msg.is_empty() {
                msg.push(' ');
            }
            msg.push_str(&format!("↓{} 落后", status.behind));
        }
        Output::item_colored("同步", &msg, ItemColor::Yellow);
    }
}

fn print_latest_tag(repo_path: &Path) {
    let runner = AppContext::global().git_runner();
    let output =
        match runner.execute_in_dir(&["tag", "-l", "v*", "--sort=-version:refname"], repo_path) {
            Ok(o) => o,
            Err(_) => return,
        };

    if let Some(tag) = output.lines().next() {
        let tag = tag.trim();
        if !tag.is_empty() {
            Output::item_colored("标签", tag, ItemColor::Cyan);
        }
    }
}
