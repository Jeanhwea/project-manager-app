use crate::domain::git::command::GitCommandRunner;
use crate::domain::git::repository::RepoWalker;
use crate::utils::output::Output;
use anyhow::Result;
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
}

#[derive(Debug, clap::Args)]
pub struct BranchListArgs {
    #[arg(long, short, default_value = "3")]
    pub max_depth: Option<usize>,
    #[arg(default_value = ".")]
    pub path: String,
}

#[derive(Debug, clap::Args)]
pub struct BranchCleanArgs {
    #[arg(long, short, default_value = "3")]
    pub max_depth: Option<usize>,
    #[arg(long, short)]
    pub pattern: Option<String>,
    #[arg(long, default_value = "false")]
    pub remote: bool,
    #[arg(long, default_value = "false")]
    pub dry_run: bool,
    #[arg(default_value = ".")]
    pub path: String,
}

#[derive(Debug, clap::Args)]
pub struct BranchSwitchArgs {
    #[arg(help = "Branch name to switch to")]
    pub branch: String,
    #[arg(long, short, default_value = "3")]
    pub max_depth: Option<usize>,
    #[arg(default_value = ".")]
    pub path: String,
}

#[derive(Debug, clap::Args)]
pub struct BranchRenameArgs {
    #[arg(help = "Old branch name")]
    pub old_name: String,
    #[arg(help = "New branch name")]
    pub new_name: String,
    #[arg(long, short, default_value = "3")]
    pub max_depth: Option<usize>,
    #[arg(default_value = ".")]
    pub path: String,
}

pub fn run(args: BranchArgs) -> Result<()> {
    match args {
        BranchArgs::List(args) => execute_list(args),
        BranchArgs::Clean(args) => execute_clean(args),
        BranchArgs::Switch(args) => execute_switch(args),
        BranchArgs::Rename(args) => execute_rename(args),
    }
}

fn execute_list(args: BranchListArgs) -> Result<()> {
    let search_path = crate::utils::path::canonicalize_path(&args.path)?;
    let walker = RepoWalker::new(&search_path, args.max_depth.unwrap_or(3))?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    let runner = GitCommandRunner::new();
    let total = walker.total();

    for (index, repo_info) in walker.repositories().iter().enumerate() {
        let repo_path = &repo_info.path;
        let branch_output = match runner.execute(&["branch", "--list"], Some(repo_path)) {
            Ok(o) => o,
            Err(_) => continue,
        };

        if branch_output.trim().is_empty() {
            continue;
        }

        Output::repo_header(index + 1, total, repo_path);

        for line in branch_output.lines() {
            let is_current = line.starts_with("* ");
            let branch_name = line.trim_start_matches("* ").trim();

            if is_current {
                Output::item("当前", branch_name);
            } else {
                Output::message(&format!("  {}", branch_name));
            }
        }
    }

    Ok(())
}

fn execute_clean(args: BranchCleanArgs) -> Result<()> {
    let search_path = crate::utils::path::canonicalize_path(&args.path)?;
    let walker = RepoWalker::new(&search_path, args.max_depth.unwrap_or(3))?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    let runner = GitCommandRunner::new();
    let total = walker.total();

    for (index, repo_info) in walker.repositories().iter().enumerate() {
        let repo_path = &repo_info.path;

        let current_branch = match runner.get_current_branch(repo_path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let branch_output = match runner.execute(&["branch", "--list"], Some(repo_path)) {
            Ok(o) => o,
            Err(_) => continue,
        };

        let branches: Vec<&str> = branch_output
            .lines()
            .map(|line| line.trim_start_matches("* ").trim())
            .filter(|name| *name != current_branch)
            .filter(|name| {
                if let Some(ref pattern) = args.pattern {
                    match_pattern(name, pattern)
                } else {
                    is_merged_branch(name, repo_path, &runner)
                }
            })
            .collect();

        if branches.is_empty() {
            continue;
        }

        Output::repo_header(index + 1, total, repo_path);

        for branch in &branches {
            if args.dry_run {
                Output::skip(&format!("git branch -d {}", branch));
            } else {
                match runner.execute_with_success(&["branch", "-d", branch], Some(repo_path)) {
                    Ok(()) => Output::success(&format!("已删除分支: {}", branch)),
                    Err(e) => Output::error(&format!("删除分支 {} 失败: {}", branch, e)),
                }
            }

            if args.remote {
                if args.dry_run {
                    Output::skip(&format!("git push origin --delete {}", branch));
                } else {
                    match runner.execute_with_success(
                        &["push", "origin", "--delete", branch],
                        Some(repo_path),
                    ) {
                        Ok(()) => Output::success(&format!("已删除远程分支: {}", branch)),
                        Err(e) => Output::error(&format!("删除远程分支 {} 失败: {}", branch, e)),
                    }
                }
            }
        }
    }

    Ok(())
}

fn execute_switch(args: BranchSwitchArgs) -> Result<()> {
    let search_path = crate::utils::path::canonicalize_path(&args.path)?;
    let walker = RepoWalker::new(&search_path, args.max_depth.unwrap_or(3))?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    let runner = GitCommandRunner::new();

    for repo_info in walker.repositories() {
        let repo_path = &repo_info.path;

        let has_branch = runner
            .execute(&["rev-parse", "--verify", &args.branch], Some(repo_path))
            .is_ok();

        if !has_branch {
            Output::skip(&format!(
                "{}: 分支 {} 不存在",
                repo_path.display(),
                args.branch
            ));
            continue;
        }

        match runner.execute_streaming(&["checkout", &args.branch], repo_path) {
            Ok(()) => Output::success(&format!(
                "{}: 已切换到 {}",
                repo_path.display(),
                args.branch
            )),
            Err(e) => Output::error(&format!(
                "{}: 切换到 {} 失败: {}",
                repo_path.display(),
                args.branch,
                e
            )),
        }
    }

    Ok(())
}

fn execute_rename(args: BranchRenameArgs) -> Result<()> {
    let search_path = crate::utils::path::canonicalize_path(&args.path)?;
    let walker = RepoWalker::new(&search_path, args.max_depth.unwrap_or(3))?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    let runner = GitCommandRunner::new();

    for repo_info in walker.repositories() {
        let repo_path = &repo_info.path;

        let has_branch = runner
            .execute(&["rev-parse", "--verify", &args.old_name], Some(repo_path))
            .is_ok();

        if !has_branch {
            Output::skip(&format!(
                "{}: 分支 {} 不存在",
                repo_path.display(),
                args.old_name
            ));
            continue;
        }

        match runner
            .execute_streaming(&["branch", "-m", &args.old_name, &args.new_name], repo_path)
        {
            Ok(()) => Output::success(&format!(
                "{}: {} -> {}",
                repo_path.display(),
                args.old_name,
                args.new_name
            )),
            Err(e) => Output::error(&format!("{}: 重命名失败: {}", repo_path.display(), e)),
        }
    }

    Ok(())
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

fn is_merged_branch(name: &str, repo_path: &Path, runner: &GitCommandRunner) -> bool {
    runner
        .execute(&["branch", "--merged", "master"], Some(repo_path))
        .map(|output| {
            output
                .lines()
                .any(|line| line.trim_start_matches("* ").trim() == name)
        })
        .unwrap_or(false)
}
