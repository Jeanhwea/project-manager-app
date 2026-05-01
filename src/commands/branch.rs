use super::{Command, CommandResult};
use crate::domain::git::command::GitCommandRunner;
use crate::domain::git::repository::RepoWalker;
use colored::Colorize;
use std::path::Path;

/// Branch command arguments
#[derive(Debug)]
pub enum BranchArgs {
    /// List branches across all repositories
    List(ListArgs),
    /// Clean merged branches across all repositories
    Clean(CleanArgs),
    /// Switch to a branch across all repositories
    Switch(SwitchArgs),
    /// Rename a branch across all repositories
    Rename(RenameArgs),
}

/// List branches arguments
#[derive(Debug)]
pub struct ListArgs {
    /// Maximum depth to search for repositories
    pub max_depth: Option<usize>,
    /// Path to the directory to search for repositories
    pub path: String,
}

/// Clean branches arguments
#[derive(Debug)]
pub struct CleanArgs {
    /// Maximum depth to search for repositories
    pub max_depth: Option<usize>,
    /// Also delete remote merged branches
    pub remote: bool,
    /// Path to the directory to search for repositories
    pub path: String,
    /// Dry run: show what would be changed without making any modifications
    pub dry_run: bool,
}

/// Switch branch arguments
#[derive(Debug)]
pub struct SwitchArgs {
    /// Branch name to switch to
    pub branch: String,
    /// Create the branch if it does not exist
    pub create: bool,
    /// Maximum depth to search for repositories
    pub max_depth: Option<usize>,
    /// Path to the directory to search for repositories
    pub path: String,
    /// Dry run: show what would be changed without making any modifications
    pub dry_run: bool,
}

/// Rename branch arguments
#[derive(Debug)]
pub struct RenameArgs {
    /// Old branch name
    pub old_name: String,
    /// New branch name
    pub new_name: String,
    /// Maximum depth to search for repositories
    pub max_depth: Option<usize>,
    /// Path to the directory to search for repositories
    pub path: String,
    /// Dry run: show what would be changed without making any modifications
    pub dry_run: bool,
}

/// Branch command
pub struct BranchCommand;

impl Command for BranchCommand {
    type Args = BranchArgs;

    fn execute(args: Self::Args) -> CommandResult {
        match args {
            BranchArgs::List(list_args) => execute_list(list_args),
            BranchArgs::Clean(clean_args) => execute_clean(clean_args),
            BranchArgs::Switch(switch_args) => execute_switch(switch_args),
            BranchArgs::Rename(rename_args) => execute_rename(rename_args),
        }
    }
}

fn execute_list(args: ListArgs) -> CommandResult {
    let walker =
        RepoWalker::new(Path::new(&args.path), args.max_depth.unwrap_or(3)).map_err(|e| {
            super::CommandError::ExecutionFailed(format!("Failed to find repositories: {}", e))
        })?;

    if walker.is_empty() {
        return Ok(());
    }

    walker
        .walk(|path, _index, _total| {
            list_branches(path);
            Ok(())
        })
        .map_err(|e| {
            super::CommandError::ExecutionFailed(format!("Failed to walk repositories: {}", e))
        })?;

    Ok(())
}

fn execute_clean(args: CleanArgs) -> CommandResult {
    let walker =
        RepoWalker::new(Path::new(&args.path), args.max_depth.unwrap_or(3)).map_err(|e| {
            super::CommandError::ExecutionFailed(format!("Failed to find repositories: {}", e))
        })?;

    if walker.is_empty() {
        return Ok(());
    }

    let runner = GitCommandRunner::new();

    walker
        .walk(|path, _index, _total| {
            clean_merged_branches(&runner, path, args.remote, args.dry_run)?;
            Ok(())
        })
        .map_err(|e| {
            super::CommandError::ExecutionFailed(format!("Failed to clean branches: {}", e))
        })?;

    Ok(())
}

fn execute_switch(args: SwitchArgs) -> CommandResult {
    let walker =
        RepoWalker::new(Path::new(&args.path), args.max_depth.unwrap_or(3)).map_err(|e| {
            super::CommandError::ExecutionFailed(format!("Failed to find repositories: {}", e))
        })?;

    if walker.is_empty() {
        return Ok(());
    }

    let runner = GitCommandRunner::new();

    walker
        .walk(|path, _index, _total| {
            switch_branch(&runner, path, &args.branch, args.create, args.dry_run)?;
            Ok(())
        })
        .map_err(|e| {
            super::CommandError::ExecutionFailed(format!("Failed to switch branches: {}", e))
        })?;

    Ok(())
}

fn execute_rename(args: RenameArgs) -> CommandResult {
    let walker =
        RepoWalker::new(Path::new(&args.path), args.max_depth.unwrap_or(3)).map_err(|e| {
            super::CommandError::ExecutionFailed(format!("Failed to find repositories: {}", e))
        })?;

    if walker.is_empty() {
        return Ok(());
    }

    let runner = GitCommandRunner::new();

    walker
        .walk(|path, _index, _total| {
            rename_branch(&runner, path, &args.old_name, &args.new_name, args.dry_run)?;
            Ok(())
        })
        .map_err(|e| {
            super::CommandError::ExecutionFailed(format!("Failed to rename branches: {}", e))
        })?;

    Ok(())
}

fn list_branches(repo_path: &Path) {
    let runner = GitCommandRunner::new();

    // Get current branch
    let current = runner.execute_in_dir(&["branch", "--show-current"], repo_path).unwrap_or_default();

    // Get local branches
    let local_branches = match runner.execute_in_dir(&["branch", "--list"], repo_path) {
        Ok(output) => output
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|line| {
                if let Some(stripped) = line.strip_prefix('*') {
                    stripped.trim().to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<String>>(),
        Err(_) => Vec::new(),
    };

    if !local_branches.is_empty() {
        println!("  本地分支:");
        for branch in &local_branches {
            if Some(branch.as_str()) == Some(current.as_str()) {
                println!("    {} {}", "*".green(), branch.yellow());
            } else {
                println!("     {}", branch);
            }
        }
    }

    // Get remote branches
    let remote_branches = match runner.execute_in_dir(&["branch", "-r", "--list"], repo_path) {
        Ok(output) => output
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect::<Vec<String>>(),
        Err(_) => Vec::new(),
    };

    if !remote_branches.is_empty() {
        println!("  远程分支:");
        for branch in &remote_branches {
            println!("    {}", branch.dimmed());
        }
    }
}

fn switch_branch(
    runner: &GitCommandRunner,
    repo_path: &Path,
    branch_name: &str,
    create: bool,
    dry_run: bool,
) -> Result<(), crate::domain::git::GitError> {
    // Get current branch
    let current = runner.execute_in_dir(&["branch", "--show-current"], repo_path).unwrap_or_default();

    if current.trim() == branch_name {
        println!("  {} 已在分支 {} 上", "跳过".dimmed(), branch_name.yellow());
        return Ok(());
    }

    if create {
        // Check if branch exists
        let local_branches = match runner.execute_in_dir(&["branch", "--list"], repo_path) {
            Ok(output) => output
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(|line| {
                    if let Some(stripped) = line.strip_prefix('*') {
                        stripped.trim().to_string()
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<String>>(),
            Err(_) => Vec::new(),
        };

        let branch_exists = local_branches.iter().any(|b| b == branch_name);

        if branch_exists {
            println!(
                "  {} 分支 {} 已存在，直接切换",
                "提示".yellow(),
                branch_name.yellow()
            );
            if !dry_run {
                runner.execute_with_success_in_dir(&["checkout", branch_name], repo_path)?;
            }
        } else {
            if !dry_run {
                runner
                    .execute_with_success_in_dir(&["checkout", "-b", branch_name], repo_path)?;
                println!(
                    "  {} 创建并切换到分支 {}",
                    "完成".green(),
                    branch_name.yellow()
                );
            } else {
                println!(
                    "  {} 创建并切换到分支 {}",
                    "模拟".cyan(),
                    branch_name.yellow()
                );
            }
        }
    } else {
        // Check if branch exists
        let local_branches = match runner.execute_in_dir(&["branch", "--list"], repo_path) {
            Ok(output) => output
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(|line| {
                    if let Some(stripped) = line.strip_prefix('*') {
                        stripped.trim().to_string()
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<String>>(),
            Err(_) => Vec::new(),
        };

        let branch_exists = local_branches.iter().any(|b| b == branch_name);

        if !branch_exists {
            println!(
                "  {} 分支 {} 不存在 (使用 --create 创建新分支)",
                "跳过".red(),
                branch_name.red()
            );
            return Ok(());
        }

        if !dry_run {
            runner.execute_with_success_in_dir(&["checkout", branch_name], repo_path)?;
            println!("  {} 切换到分支 {}", "完成".green(), branch_name.yellow());
        } else {
            println!("  {} 切换到分支 {}", "模拟".cyan(), branch_name.yellow());
        }
    }

    Ok(())
}

fn rename_branch(
    runner: &GitCommandRunner,
    repo_path: &Path,
    old_name: &str,
    new_name: &str,
    dry_run: bool,
) -> Result<(), crate::domain::git::GitError> {
    // Get local branches
    let local_branches = match runner.execute_in_dir(&["branch", "--list"], repo_path) {
        Ok(output) => output
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|line| {
                if let Some(stripped) = line.strip_prefix('*') {
                    stripped.trim().to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<String>>(),
        Err(_) => Vec::new(),
    };

    if !local_branches.iter().any(|b| b == old_name) {
        println!("  {} 分支 {} 不存在", "跳过".dimmed(), old_name.red());
        return Ok(());
    }

    if local_branches.iter().any(|b| b == new_name) {
        println!("  {} 分支 {} 已存在", "跳过".red(), new_name.red());
        return Ok(());
    }

    // Get current branch
    let current = runner.execute_in_dir(&["branch", "--show-current"], repo_path).unwrap_or_default();
    let is_current = current.trim() == old_name;

    if !dry_run {
        runner.execute_with_success_in_dir(&["branch", "-m", old_name, new_name], repo_path)?;

        if is_current {
            println!(
                "  {} 当前分支 {} -> {}",
                "重命名".green(),
                old_name.red(),
                new_name.yellow()
            );
        } else {
            println!(
                "  {} 分支 {} -> {}",
                "重命名".green(),
                old_name.red(),
                new_name.yellow()
            );
        }
    } else {
        if is_current {
            println!(
                "  {} 当前分支 {} -> {}",
                "模拟重命名".cyan(),
                old_name.red(),
                new_name.yellow()
            );
        } else {
            println!(
                "  {} 分支 {} -> {}",
                "模拟重命名".cyan(),
                old_name.red(),
                new_name.yellow()
            );
        }
    }

    Ok(())
}

fn clean_merged_branches(
    runner: &GitCommandRunner,
    repo_path: &Path,
    remote: bool,
    dry_run: bool,
) -> Result<(), crate::domain::git::GitError> {
    // Get current branch
    let current = match runner.execute_in_dir(&["branch", "--show-current"], repo_path) {
        Ok(output) => output.trim().to_string(),
        Err(_) => "master".to_string(),
    };

    // Get merged branches
    let merged_branches =
        match runner.execute_in_dir(&["branch", "--merged", &current], repo_path) {
            Ok(output) => output
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(|line| {
                    if let Some(stripped) = line.strip_prefix('*') {
                        stripped.trim().to_string()
                    } else {
                        line.to_string()
                    }
                })
                .filter(|branch| branch != &current && !branch.starts_with("remotes/"))
                .collect::<Vec<String>>(),
            Err(_) => Vec::new(),
        };

    if merged_branches.is_empty() {
        println!("  {}", "无已合并分支".green());
        return Ok(());
    }

    for branch in &merged_branches {
        if dry_run {
            println!("  {} 删除本地分支 {}", "模拟".cyan(), branch.red());
        } else {
            match runner.execute_with_success_in_dir(&["branch", "-d", branch], repo_path) {
                Ok(_) => println!("  {} 本地分支 {}", "已删除".green(), branch.red()),
                Err(e) => println!("  {} 本地分支 {} - {}", "删除失败".red(), branch.red(), e),
            }
        }
    }

    if remote {
        // Get remote merged branches
        let remote_merged =
            match runner.execute_in_dir(&["branch", "-r", "--merged", &current], repo_path) {
                Ok(output) => output
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .filter(|line| !line.contains("origin/HEAD"))
                    .map(|line| {
                        let parts: Vec<&str> = line.split('/').collect();
                        if parts.len() > 1 {
                            parts[1..].join("/")
                        } else {
                            line.to_string()
                        }
                    })
                    .collect::<Vec<String>>(),
                Err(_) => Vec::new(),
            };

        if remote_merged.is_empty() {
            println!("  {}", "无已合并的远程分支".green());
        } else {
            for branch in &remote_merged {
                if dry_run {
                    println!("  {} 删除远程分支 {}", "模拟".cyan(), branch.red());
                } else {
                    match runner.execute_with_success_in_dir(
                        &["push", "origin", "--delete", branch],
                        repo_path,
                    ) {
                        Ok(_) => println!("  {} 远程分支 {}", "已删除".green(), branch.red()),
                        Err(e) => {
                            println!("  {} 远程分支 {} - {}", "删除失败".red(), branch.red(), e)
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
