use crate::app::common::git;
use crate::app::common::runner::CommandRunner;
use crate::utils;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

pub fn execute(path: &str, max_depth: Option<usize>, short: bool) -> Result<()> {
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

    for (index, repo) in git_repos.iter().enumerate() {
        let repo_path = repo
            .path
            .canonicalize()
            .unwrap_or_else(|_| repo.path.clone());

        let progress = format!("({}/{})", index + 1, total);
        println!(
            "{}>> {}",
            progress.white().bold(),
            utils::format_path(&repo_path).cyan().underline(),
        );

        if repo.repo_type == git::RepoType::Submodule {
            println!("  {}", "(submodule, 跳过)".dimmed());
            continue;
        }

        if short {
            print_short_status(&repo_path);
        } else {
            print_full_status(&repo_path);
        }
    }

    Ok(())
}

fn print_short_status(repo_path: &Path) {
    let branch = get_current_branch(repo_path);
    let dirty = !is_workdir_clean(repo_path);

    let branch_display = branch.unwrap_or_else(|| "HEAD".to_string());
    let status_icon = if dirty { "✗".red() } else { "✔".green() };

    println!(
        "  {} {} {}",
        status_icon,
        branch_display.yellow(),
        if dirty {
            "(dirty)".red().to_string()
        } else {
            "(clean)".green().to_string()
        },
    );
}

fn print_full_status(repo_path: &Path) {
    let branch = get_current_branch(repo_path);
    let dirty = !is_workdir_clean(repo_path);

    let branch_display = branch.as_deref().unwrap_or("HEAD (detached)");
    let status_label = if dirty {
        "脏工作目录".red().to_string()
    } else {
        "干净工作目录".green().to_string()
    };

    println!("  分支: {}", branch_display.yellow());
    println!("  状态: {}", status_label);

    if dirty {
        print_dirty_details(repo_path);
    }

    print_ahead_behind(repo_path);

    print_latest_tag(repo_path);

    let remotes = git::get_remote_info(repo_path);
    if !remotes.is_empty() {
        println!("  远程:");
        for (name, url) in &remotes {
            println!("    {} {}", name.green(), url.dimmed());
        }
    }
}

fn print_dirty_details(repo_path: &Path) {
    let output =
        match CommandRunner::run_quiet_in_dir("git", &["status", "--porcelain"], repo_path) {
            Ok(o) => o,
            Err(_) => return,
        };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return,
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

    let mut parts = Vec::new();
    if staged > 0 {
        parts.push(format!("{} 已暂存", staged.to_string().yellow()));
    }
    if unstaged > 0 {
        parts.push(format!("{} 未暂存", unstaged.to_string().yellow()));
    }
    if untracked > 0 {
        parts.push(format!("{} 未跟踪", untracked.to_string().yellow()));
    }

    if !parts.is_empty() {
        println!("  详情: {}", parts.join(", "));
    }
}

fn print_ahead_behind(repo_path: &Path) {
    let branch = match get_current_branch(repo_path) {
        Some(b) => b,
        None => return,
    };

    let upstream = format!("{}@{{upstream}}...HEAD", branch);
    let output = match CommandRunner::run_quiet_in_dir(
        "git",
        &["rev-list", "--count", "--left-right", &upstream],
        repo_path,
    ) {
        Ok(o) => o,
        Err(_) => return,
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return,
    };

    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return;
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() != 2 {
        return;
    }

    let ahead: usize = match parts[0].parse() {
        Ok(n) => n,
        Err(_) => return,
    };
    let behind: usize = match parts[1].parse() {
        Ok(n) => n,
        Err(_) => return,
    };

    if ahead == 0 && behind == 0 {
        println!("  同步: {}", "与远程一致".green());
    } else {
        let mut msg = String::new();
        if ahead > 0 {
            msg.push_str(&format!("↑{} 领先", ahead));
        }
        if behind > 0 {
            if !msg.is_empty() {
                msg.push(' ');
            }
            msg.push_str(&format!("↓{} 落后", behind));
        }
        println!("  同步: {}", msg.yellow());
    }
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
