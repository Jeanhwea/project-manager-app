//! Utilities module
//!
//! This module contains general utility functions organized by domain.

pub mod file;
pub mod git;
pub mod path;

// Re-export commonly used utilities
pub use self::path::canonicalize_path;
pub use self::path::format_path;

// Re-export Git utilities
pub use self::git::git_command;
pub use self::git::is_git_repo;
pub use self::git::get_current_branch;
pub use self::git::get_remote_urls;
pub use self::git::has_uncommitted_changes;
pub use self::git::get_repo_root;
pub use self::git::is_git_available;
pub use self::git::get_git_version;
pub use self::git::get_git_user_name;
pub use self::git::get_git_user_email;
pub use self::git::get_head_commit;
pub use self::git::get_head_commit_short;
pub use self::git::get_head_commit_message;
pub use self::git::get_head_tag;
pub use self::git::get_head_or_nearest_tag;
pub use self::git::is_detached_head;
pub use self::git::get_origin_url;
pub use self::git::get_upstream_url;
pub use self::git::get_status_summary;
pub use self::git::get_log;
pub use self::git::get_staged_diff;
pub use self::git::get_unstaged_diff;
pub use self::git::has_staged_changes;
pub use self::git::has_unstaged_changes;
pub use self::git::get_branch_list;
pub use self::git::get_remote_list;
pub use self::git::is_repo_clean;
pub use self::git::get_last_commit_date;
pub use self::git::get_last_commit_author;
pub use self::git::get_last_commit_author_email;
pub use self::git::get_file_count;
pub use self::git::get_repo_size;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_format_path() {
        let path = Path::new(r"\\?\C:\Users\test");
        let formatted = format_path(path);
        assert_eq!(formatted, r"C:\Users\test");
    }

    #[test]
    fn test_format_path_normal() {
        let path = Path::new("/home/user/project");
        let formatted = format_path(path);
        assert_eq!(formatted, "/home/user/project");
    }
}
