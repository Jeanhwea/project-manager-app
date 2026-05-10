use crate::domain::git::command::GitCommandRunner;
use crate::utils::output::Output;
use anyhow::Result;
use std::path::Path;

#[derive(Debug, clap::Subcommand)]
pub enum BranchArgs {
    /// List branches across all repositories
    #[command(visible_alias = "ls")]
    List(BranchListArgs),
    /// Clean up merged branches across all repositories
    #[command(visible_alias = "cl")]
    Clean(BranchCleanArgs),
    /// Switch to a branch across all repositories
    #[command(visible_alias = "sw")]
    Switch(BranchSwitchArgs),
    /// Rename a branch across all repositories
    #[command(visible_alias = "rn")]
    Rename(BranchRenameArgs),
}

#[derive(Debug, clap::Args)]
pub struct BranchListArgs {
    /// Maximum depth to search for repositories
    #[arg(long, short, default_value = "3")]
    pub max_depth: Option<usize>,
    /// Path to search for repositories
    #[arg(default_value = ".")]
    pub path: String,
}

#[derive(Debug, clap::Args)]
pub struct BranchCleanArgs {
    /// Maximum depth to search for repositories
    #[arg(long, short, default_value = "3")]
    pub max_depth: Option<usize>,
    /// Branch name pattern to clean (supports wildcards)
    #[arg(long, short)]
    pub pattern: Option<String>,
    /// Also delete remote branches
    #[arg(long, default_value = "false")]
    pub remote: bool,
    /// Dry run: show what would be changed without making any modifications
    #[arg(long, default_value = "false")]
    pub dry_run: bool,
    /// Path to search for repositories
    #[arg(default_value = ".")]
    pub path: String,
}

#[derive(Debug, clap::Args)]
pub struct BranchSwitchArgs {
    /// Branch name to switch to
    #[arg(help = "Branch name to switch to")]
    pub branch: String,
    /// Maximum depth to search for repositories
    #[arg(long, short, default_value = "3")]
    pub max_depth: Option<usize>,
    /// Path to search for repositories
    #[arg(default_value = ".")]
    pub path: String,
}

#[derive(Debug, clap::Args)]
pub struct BranchRenameArgs {
    /// Old branch name
    #[arg(help = "Old branch name")]
    pub old_name: String,
    /// New branch name
    #[arg(help = "New branch name")]
    pub new_name: String,
    /// Maximum depth to search for repositories
    #[arg(long, short, default_value = "3")]
    pub max_depth: Option<usize>,
    /// Path to search for repositories
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
    let walker = crate::domain::git::repository::RepoWalker::new(
        &search_path,
        args.max_depth.unwrap_or(3),
    )?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    let runner = GitCommandRunner::new();

    for repo_info in walker.repositories() {
        let repo_path = &repo_info.path;
        let branch_output = match runner.execute_in_dir(&["branch", "--list"], repo_path) {
            Ok(o) => o,
            Err(_) => continue,
        };

        if branch_output.trim().is_empty() {
            continue;
        }

        Output::repo_header(0, 0, repo_path);

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
    let walker = crate::domain::git::repository::RepoWalker::new(
        &search_path,
        args.max_depth.unwrap_or(3),
    )?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    let runner = AppContext::git_runner();

    for repo_info in walker.repositories() {
        let repo_path = &repo_info.path;

        let current_branch = match runner.get_current_branch(repo_path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let branch_output = match runner.execute_in_dir(&["branch", "--list"], repo_path) {
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

        Output::repo_header(0, 0, repo_path);

        for branch in &branches {
            if args.dry_run {
                Output::skip(&format!("git branch -d {}", branch));
            } else {
                match runner.execute_with_success_in_dir(&["branch", "-d", branch], repo_path) {
                    Ok(()) => Output::success(&format!("已删除分支: {}", branch)),
                    Err(e) => Output::error(&format!("删除分支 {} 失败: {}", branch, e)),
                }
            }

            if args.remote {
                if args.dry_run {
                    Output::skip(&format!("git push origin --delete {}", branch));
                } else {
                    match runner.execute_with_success_in_dir(
                        &["push", "origin", "--delete", branch],
                        repo_path,
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
    let walker = crate::domain::git::repository::RepoWalker::new(
        &search_path,
        args.max_depth.unwrap_or(3),
    )?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    let runner = AppContext::git_runner();

    for repo_info in walker.repositories() {
        let repo_path = &repo_info.path;

        let has_branch = runner
            .execute_in_dir(&["rev-parse", "--verify", &args.branch], repo_path)
            .is_ok();

        if !has_branch {
            Output::skip(&format!(
                "{}: 分支 {} 不存在",
                repo_path.display(),
                args.branch
            ));
            continue;
        }

        match runner.execute_streaming_in_dir(&["checkout", &args.branch], repo_path) {
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
    let walker = crate::domain::git::repository::RepoWalker::new(
        &search_path,
        args.max_depth.unwrap_or(3),
    )?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    let runner = AppContext::git_runner();

    for repo_info in walker.repositories() {
        let repo_path = &repo_info.path;

        let has_branch = runner
            .execute_in_dir(&["rev-parse", "--verify", &args.old_name], repo_path)
            .is_ok();

        if !has_branch {
            Output::skip(&format!(
                "{}: 分支 {} 不存在",
                repo_path.display(),
                args.old_name
            ));
            continue;
        }

        match runner.execute_streaming_in_dir(
            &["branch", "-m", &args.old_name, &args.new_name],
            repo_path,
        ) {
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
        .execute_in_dir(&["branch", "--merged", "master"], repo_path)
        .map(|output| {
            output
                .lines()
                .any(|line| line.trim_start_matches("* ").trim() == name)
        })
        .unwrap_or(false)
}
