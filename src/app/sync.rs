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
    let mut repos = Vec::new();
    
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                
                if path.is_dir() {
                    if path.ends_with(".git") {
                        // Found a git repository, add its parent directory
                        if let Some(parent) = path.parent() {
                            repos.push(parent.to_path_buf());
                        }
                    } else {
                        // Recursively search in subdirectories
                        repos.extend(find_git_repositories(&path));
                    }
                } else if path.is_file() && path.file_name().unwrap_or_default() == ".git" {
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
