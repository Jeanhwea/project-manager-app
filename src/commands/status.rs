use crate::domain::git::command::GitCommandRunner;
use crate::domain::git::repository::RepoWalker;
use crate::utils::output::Output;
use anyhow::Result;

#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    #[arg(
        long,
        short,
        default_value = "3",
        help = "Maximum depth to search for repositories"
    )]
    pub max_depth: Option<usize>,
    #[arg(default_value = ".", help = "Path to search")]
    pub path: String,
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Show detailed change list per repository"
    )]
    pub verbose: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Fetch from remote before checking sync status"
    )]
    pub fetch: bool,
}

pub fn run(args: StatusArgs) -> Result<()> {
    let search_path = crate::utils::path::canonicalize_path(&args.path)?;
    let walker = RepoWalker::new(&search_path, args.max_depth.unwrap_or(3))?;

    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(());
    }

    let runner = GitCommandRunner::new();
    let total = walker.total();

    let mut ahead_count = 0;
    let mut behind_count = 0;
    let mut dirty_count = 0;
    let mut clean_count = 0;

    for (index, repo_info) in walker.repositories().iter().enumerate() {
        let repo_path = &repo_info.path;
        Output::repo_header(index + 1, total, repo_path);

        let branch = runner.get_current_branch(repo_path).unwrap_or_default();
        Output::item("分支", &branch);

        let is_dirty = runner.has_uncommitted_changes(repo_path).unwrap_or(true);
        if is_dirty {
            dirty_count += 1;
            Output::warning("有未提交的更改");
        } else {
            clean_count += 1;
            Output::success("工作目录干净");
        }

        if args.fetch
            && let Ok(remotes) = runner.get_remote_list(repo_path)
            && remotes.iter().any(|r| r == "origin")
        {
            let _ = runner.execute(&["fetch", "origin"], Some(repo_path));
        }

        if let Ok(remotes) = runner.get_remote_list(repo_path) {
            for remote in &remotes {
                if let Ok(output) = runner.execute(
                    &[
                        "rev-list",
                        "--left-right",
                        &format!("{}...{}", remote, branch),
                    ],
                    Some(repo_path),
                ) {
                    let ahead = output.lines().filter(|l| l.starts_with('<')).count();
                    let behind = output.lines().filter(|l| l.starts_with('>')).count();

                    if ahead > 0 || behind > 0 {
                        ahead_count += ahead;
                        behind_count += behind;
                        Output::item("同步", &format!("{} 领先, {} 落后", ahead, behind));
                    }
                }
            }
        }

        if args.verbose
            && let Ok(output) = runner.execute(&["status", "--short"], Some(repo_path))
        {
            for line in output.lines() {
                Output::detail("变更", line);
            }
        }
    }

    Output::header("状态汇总");
    Output::item("仓库总数", &total.to_string());
    Output::item("干净", &clean_count.to_string());
    Output::item("有更改", &dirty_count.to_string());
    if ahead_count > 0 || behind_count > 0 {
        Output::item(
            "同步",
            &format!("{} 领先, {} 落后", ahead_count, behind_count),
        );
    }

    Ok(())
}
