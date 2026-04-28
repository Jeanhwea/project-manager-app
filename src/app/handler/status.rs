use crate::app::common::git;
use crate::app::common::runner::CommandRunner;
use crate::utils;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum StatusFilter {
    Dirty,
    Clean,
    Ahead,
    Behind,
}

impl StatusFilter {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "dirty" => Some(StatusFilter::Dirty),
            "clean" => Some(StatusFilter::Clean),
            "ahead" => Some(StatusFilter::Ahead),
            "behind" => Some(StatusFilter::Behind),
            _ => None,
        }
    }
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

pub fn execute(path: &str, max_depth: Option<usize>, short: bool, filter: Option<StatusFilter>) -> Result<()> {
    let root_dir = Path::new(path);

    if !root_dir.exists() {
        anyhow::bail!("目录不存在: {}", path);
    }

    let git_repos = git::find_git_repositories_or_current(root_dir, max_depth);

    if git_repos.is_empty() {
        println!("未找到git仓库");
        return Ok(());
    }

    let total = git_repos.len();
    let mut stats = StatusStats::default();

    for (index, repo) in git_repos.iter().enumerate() {
        let repo_path = repo
            .path
            .canonicalize()
            .unwrap_or_else(|_| repo.path.clone());

        if repo.repo_type == git::RepoType::Submodule {
            stats.submodules += 1;
            continue;
        }

        let status = collect_repo_status(&repo_path);

        if !matches_filter(&status, &filter) {
            stats.skipped += 1;
            continue;
        }

        let progress = format!("({}/{})", index + 1, total);
        println!(
            "{}>> {}",
            progress.white().bold(),
            utils::format_path(&repo_path).cyan().underline(),
        );

        if short {
            print_short_status(&status);
        } else {
            print_full_status(&repo_path, &status);
        }

        stats.update(&status);
    }

    print_summary(&stats, total);

    Ok(())
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
    let branch = get_current_branch(repo_path);
    let dirty = !is_workdir_clean(repo_path);
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

fn print_summary(stats: &StatusStats, total: usize) {
    println!();
    println!("{}", "── 汇总 ──".green().bold());
    println!(
        "  仓库: {} (显示 {}, 跳过 {}, 子模块 {})",
        total.to_string().white().bold(),
        stats.total_shown.to_string().cyan(),
        stats.skipped.to_string().dimmed(),
        stats.submodules.to_string().dimmed(),
    );
    println!(
        "  状态: {} {} {} {}",
        "✔".green(),
        format!("{} 干净", stats.clean_count).green(),
        "✗".red(),
        format!("{} 脏", stats.dirty_count).red(),
    );
    if stats.ahead_count > 0 || stats.behind_count > 0 {
        println!(
            "  同步: {} 领先, {} 落后",
            stats.ahead_count.to_string().yellow(),
            stats.behind_count.to_string().yellow(),
        );
    }
    if stats.total_staged > 0 || stats.total_unstaged > 0 || stats.total_untracked > 0 {
        println!(
            "  文件: {} 已暂存, {} 未暂存, {} 未跟踪",
            stats.total_staged.to_string().yellow(),
            stats.total_unstaged.to_string().yellow(),
            stats.total_untracked.to_string().yellow(),
        );
    }
}

fn print_short_status(status: &RepoStatus) {
    let branch_display = status.branch.as_deref().unwrap_or("HEAD");
    let status_icon = if status.dirty { "✗".red() } else { "✔".green() };

    let mut extra = Vec::new();
    if status.ahead > 0 {
        extra.push(format!("↑{}", status.ahead));
    }
    if status.behind > 0 {
        extra.push(format!("↓{}", status.behind));
    }

    let extra_str = if extra.is_empty() {
        String::new()
    } else {
        format!(" {}", extra.join(" ").yellow())
    };

    println!(
        "  {} {} {}{}",
        status_icon,
        branch_display.yellow(),
        if status.dirty {
            "(dirty)".red().to_string()
        } else {
            "(clean)".green().to_string()
        },
        extra_str,
    );
}

fn print_full_status(repo_path: &Path, status: &RepoStatus) {
    let branch_display = status.branch.as_deref().unwrap_or("HEAD (detached)");
    let status_label = if status.dirty {
        "脏工作目录".red().to_string()
    } else {
        "干净工作目录".green().to_string()
    };

    println!("  分支: {}", branch_display.yellow());
    println!("  状态: {}", status_label);

    if status.dirty {
        print_dirty_details_from_counts(status);
    }

    print_ahead_behind_from_status(status);

    print_latest_tag(repo_path);

    let remotes = git::get_remote_info(repo_path);
    if !remotes.is_empty() {
        println!("  远程:");
        for (name, url) in &remotes {
            println!("    {} {}", name.green(), url.dimmed());
        }
    }
}

fn print_dirty_details_from_counts(status: &RepoStatus) {
    let mut parts = Vec::new();
    if status.staged > 0 {
        parts.push(format!("{} 已暂存", status.staged.to_string().yellow()));
    }
    if status.unstaged > 0 {
        parts.push(format!("{} 未暂存", status.unstaged.to_string().yellow()));
    }
    if status.untracked > 0 {
        parts.push(format!("{} 未跟踪", status.untracked.to_string().yellow()));
    }

    if !parts.is_empty() {
        println!("  详情: {}", parts.join(", "));
    }
}

fn print_ahead_behind_from_status(status: &RepoStatus) {
    if status.ahead == 0 && status.behind == 0 {
        println!("  同步: {}", "与远程一致".green());
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
        println!("  同步: {}", msg.yellow());
    }
}

fn get_dirty_counts(repo_path: &Path) -> (usize, usize, usize) {
    let output = match CommandRunner::run_quiet_in_dir("git", &["status", "--porcelain"], repo_path)
    {
        Ok(o) => o,
        Err(_) => return (0, 0, 0),
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return (0, 0, 0),
    };

    let mut staged = 0usize;
    let mut unstaged = 0usize;
    let mut untracked = 0usize;

    for line in stdout.lines() {
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

fn get_ahead_behind(repo_path: &Path) -> (usize, usize) {
    let branch = match get_current_branch(repo_path) {
        Some(b) => b,
        None => return (0, 0),
    };

    let upstream = format!("{}@{{upstream}}...HEAD", branch);
    let output = match CommandRunner::run_quiet_in_dir(
        "git",
        &["rev-list", "--count", "--left-right", &upstream],
        repo_path,
    ) {
        Ok(o) => o,
        Err(_) => return (0, 0),
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return (0, 0),
    };

    let trimmed = stdout.trim();
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

fn print_latest_tag(repo_path: &Path) {
    let output = match CommandRunner::run_quiet_in_dir(
        "git",
        &["tag", "-l", "v*", "--sort=-version:refname"],
        repo_path,
    ) {
        Ok(o) => o,
        Err(_) => return,
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return,
    };

    if let Some(tag) = stdout.lines().next() {
        let tag = tag.trim();
        if !tag.is_empty() {
            println!("  标签: {}", tag.cyan());
        }
    }
}

fn get_current_branch(repo_path: &Path) -> Option<String> {
    let output =
        CommandRunner::run_quiet_in_dir("git", &["branch", "--show-current"], repo_path).ok()?;

    let branch = String::from_utf8(output.stdout).ok()?;
    let branch = branch.trim();

    if branch.is_empty() {
        None
    } else {
        Some(branch.to_string())
    }
}

fn is_workdir_clean(repo_path: &Path) -> bool {
    CommandRunner::run_quiet_in_dir("git", &["status", "--porcelain"], repo_path)
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .is_some_and(|s| s.is_empty())
}
