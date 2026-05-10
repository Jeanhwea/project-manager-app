use crate::commands::{RepoPathArgs, init_repo_walker};
use crate::domain::git::executor::GitContext;
use crate::utils::output::Output;

#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    #[command(flatten)]
    pub repo_path: RepoPathArgs,
}

pub fn run(args: StatusArgs) -> anyhow::Result<()> {
    let Some(walker) = init_repo_walker(&args.repo_path)? else {
        return Ok(());
    };

    let total = walker.total();

    for (index, repo_info) in walker.repositories().iter().enumerate() {
        let repo_path = &repo_info.path;
        let Ok(ctx) = GitContext::collect(repo_path) else {
            continue;
        };

        Output::repo_header(index + 1, total, repo_path);

        Output::item("分支", &ctx.current_branch);

        if ctx.has_uncommitted_changes {
            Output::warning("有未提交的变更");
        } else {
            Output::success("工作区干净");
        }

        if !ctx.remotes.is_empty() {
            for remote in &ctx.remotes {
                Output::detail(&remote.name, &remote.url);
            }
        }

        if !ctx.tags.is_empty() {
            let latest_tag = &ctx.tags[0].name;
            Output::item("最新标签", latest_tag);
        }
    }

    Ok(())
}
