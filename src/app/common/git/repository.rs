use crate::app::common::config;
use crate::utils;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(PartialEq)]
pub enum RepoType {
    Regular,
    Submodule,
}

pub struct RepoInfo {
    pub path: PathBuf,
    pub repo_type: RepoType,
}

pub fn find_git_repositories(root_dir: &Path, max_depth: Option<usize>) -> Vec<RepoInfo> {
    let cfg = config::load();
    find_git_repositories_with_depth(root_dir, max_depth.unwrap_or(cfg.repository.max_depth))
}

pub fn find_git_repositories_or_current(
    root_dir: &Path,
    max_depth: Option<usize>,
) -> Vec<RepoInfo> {
    let repos = find_git_repositories(root_dir, max_depth);
    if !repos.is_empty() {
        return repos;
    }

    if let Some(top_level_dir) = super::command::get_top_level_dir() {
        return vec![RepoInfo {
            path: top_level_dir,
            repo_type: RepoType::Regular,
        }];
    }

    Vec::new()
}

fn find_git_repositories_with_depth(root_dir: &Path, max_depth: usize) -> Vec<RepoInfo> {
    let cfg = config::load();
    let skip_dirs = &cfg.repository.skip_dirs;

    let mut repos = Vec::new();

    if max_depth == 0 {
        return repos;
    }

    let git_path = root_dir.join(".git");
    if git_path.exists() {
        let repo_type = if git_path.is_dir() {
            RepoType::Regular
        } else {
            RepoType::Submodule
        };
        repos.push(RepoInfo {
            path: root_dir.to_path_buf(),
            repo_type,
        });
        return repos;
    }

    let Ok(entries) = fs::read_dir(root_dir) else {
        return repos;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_str().unwrap_or("");

        if skip_dirs.iter().any(|d| d == file_name_str) {
            continue;
        }

        if path.is_dir() {
            repos.extend(find_git_repositories_with_depth(&path, max_depth - 1));
        }
    }

    repos
}

pub fn for_each_repo<F>(
    path: &str,
    max_depth: Option<usize>,
    mut callback: F,
) -> anyhow::Result<()>
where
    F: FnMut(&Path) -> anyhow::Result<()>,
{
    let root_dir = Path::new(path);

    if !root_dir.exists() {
        anyhow::bail!("目录不存在: {}", path);
    }

    let git_repos = find_git_repositories(root_dir, max_depth);

    if git_repos.is_empty() {
        println!("未找到git仓库");
        return Ok(());
    }

    let total = git_repos.len();

    for (index, repo) in git_repos.iter().enumerate() {
        let repo_path =
            utils::canonicalize_path(&repo.path).unwrap_or_else(|_| repo.path.clone());

        let progress = format!("({}/{})", index + 1, total);
        println!(
            "{}>> {}",
            progress.white().bold(),
            utils::format_path(&repo_path).cyan().underline(),
        );

        if repo.repo_type == RepoType::Submodule {
            continue;
        }

        callback(&repo_path)?;
    }

    Ok(())
}

pub struct RepoWalker {
    repos: Vec<RepoInfo>,
}

#[allow(dead_code)]
pub struct WalkEntry<'a> {
    pub path: &'a Path,
    pub index: usize,
    pub total: usize,
}

impl RepoWalker {
    pub fn new(path: &str, max_depth: Option<usize>) -> anyhow::Result<Self> {
        let root_dir = Path::new(path);
        if !root_dir.exists() {
            anyhow::bail!("目录不存在: {}", path);
        }

        let repos = find_git_repositories_or_current(root_dir, max_depth);

        if repos.is_empty() {
            println!("未找到git仓库");
        }

        Ok(Self { repos })
    }

    pub fn is_empty(&self) -> bool {
        self.repos.is_empty()
    }

    pub fn total(&self) -> usize {
        self.repos.len()
    }

    pub fn walk<F>(&self, mut callback: F) -> anyhow::Result<()>
    where
        F: FnMut(WalkEntry<'_>) -> anyhow::Result<()>,
    {
        let total = self.repos.len();

        for (index, repo) in self.repos.iter().enumerate() {
            let repo_path =
                utils::canonicalize_path(&repo.path).unwrap_or_else(|_| repo.path.clone());

            let progress = format!("({}/{})", index + 1, total);
            println!(
                "{}>> {}",
                progress.white().bold(),
                utils::format_path(&repo_path).cyan().underline(),
            );

            if repo.repo_type == RepoType::Submodule {
                println!("  {}", "(submodule, 跳过)".dimmed());
                continue;
            }

            callback(WalkEntry {
                path: &repo_path,
                index,
                total,
            })?;
        }

        Ok(())
    }
}
