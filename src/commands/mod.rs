pub mod branch;
pub mod config;
pub mod doctor;
pub mod fork;
pub mod gitlab;
pub mod release;
pub mod selfman;
pub mod snap;
pub mod status;
pub mod sync;

use crate::domain::git::repository::RepoWalker;
use crate::utils::output::Output;
use anyhow::Result;

#[derive(Debug, clap::Args)]
pub struct RepoPathArgs {
    #[arg(
        long,
        short,
        default_value = "3",
        help = "Maximum depth to search for repositories"
    )]
    pub max_depth: Option<usize>,
    #[arg(default_value = ".", help = "Path to search for repositories")]
    pub path: String,
}

pub fn init_repo_walker(args: &RepoPathArgs) -> Result<Option<RepoWalker>> {
    let search_path = crate::utils::path::canonicalize_path(&args.path)?;
    let walker = RepoWalker::new(&search_path, args.max_depth.unwrap_or(3))?;
    if walker.is_empty() {
        Output::not_found("未找到 Git 仓库");
        return Ok(None);
    }
    Ok(Some(walker))
}
