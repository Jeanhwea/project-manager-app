use super::runner::CommandRunner;
use std::fs;
use std::path::Path;

pub fn execute(path: &str) {
    println!("开始同步所有git仓库...");

    let sync_dir = std::path::Path::new(path);
    let git_repos = find_git_repositories(sync_dir);

    if git_repos.is_empty() {
        println!("未找到git仓库");
        return;
    }

    for repo in &git_repos {
        println!("- {}", repo.display());
    }

    for repo in git_repos {
        println!("同步仓库: {}", repo.display());

        CommandRunner::run_with_success_in_dir("git", &["pull"], repo.to_str().unwrap());
    }
}

fn find_git_repositories(dir: &Path) -> Vec<std::path::PathBuf> {
    find_git_repositories_with_depth(dir, 5) 
}

fn find_git_repositories_with_depth(dir: &Path, max_depth: usize) -> Vec<std::path::PathBuf> {
    let mut repos = Vec::new();

    if max_depth == 0 {
        return repos;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                let file_name = entry.file_name();
                let file_name_str = file_name.to_str().unwrap_or("");

                // Skip .venv directories
                if file_name_str == ".venv" {
                    continue;
                }

                if path.is_dir() {
                    if file_name_str == ".git" {
                        // Found a git repository, add its parent directory
                        if let Some(parent) = path.parent() {
                            repos.push(parent.to_path_buf());
                        }
                    } else {
                        // Recursively search in subdirectories with reduced depth
                        repos.extend(find_git_repositories_with_depth(&path, max_depth - 1));
                    }
                } else if file_name_str == ".git" {
                    // Found a git submodule (has .git file instead of directory)
                    if let Some(parent) = path.parent() {
                        repos.push(parent.to_path_buf());
                    }
                }
            }
        }
    }

    repos
}
