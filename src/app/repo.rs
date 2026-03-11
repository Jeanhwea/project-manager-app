use std::fs;
use std::path::Path;

#[derive(PartialEq)]
pub enum RepoType {
    Regular,
    Submodule,
}

pub struct RepoInfo {
    pub path: std::path::PathBuf,
    pub repo_type: RepoType,
}

pub fn find_git_repositories(root_dir: &Path, max_depth: Option<usize>) -> Vec<RepoInfo> {
    find_git_repositories_with_depth(root_dir, max_depth.unwrap_or(3))
        .into_iter()
        .filter(|repo| repo.repo_type == RepoType::Regular)
        .collect()
}

fn find_git_repositories_with_depth(root_dir: &Path, max_depth: usize) -> Vec<RepoInfo> {
    let mut repos = Vec::new();

    if max_depth == 0 {
        return repos;
    }

    if let Ok(entries) = fs::read_dir(root_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_str().unwrap_or("");

            if file_name_str == ".venv" {
                continue;
            }

            if path.is_dir() {
                if file_name_str == ".git" {
                    if let Some(parent) = path.parent() {
                        repos.push(RepoInfo {
                            path: parent.to_path_buf(),
                            repo_type: RepoType::Regular,
                        });
                    }
                } else {
                    repos.extend(find_git_repositories_with_depth(&path, max_depth - 1));
                }
            } else if file_name_str == ".git" {
                if let Some(parent) = path.parent() {
                    repos.push(RepoInfo {
                        path: parent.to_path_buf(),
                        repo_type: RepoType::Submodule,
                    });
                }
            }
        }
    }

    repos
}
