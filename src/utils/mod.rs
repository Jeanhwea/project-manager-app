//! Utilities module
//!
//! This module contains general utility functions organized by domain.

pub mod file;
pub mod git;
pub mod path;

// Re-export commonly used utilities
pub use self::path::canonicalize_path;
pub use self::path::format_path;

use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

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
