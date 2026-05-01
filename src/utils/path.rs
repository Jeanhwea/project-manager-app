//! Path manipulation utilities
//!
//! This module provides cross-platform path handling utilities for path manipulation,
//! normalization, and common path operations.

use std::path::{Path, PathBuf};

/// 规范化路径，处理 Windows UNC 路径前缀
///
/// Windows 上 `std::fs::canonicalize` 会返回 `\\?\` 前缀的 UNC 路径，
/// 这可能导致路径比较和文件类型检测失败。此函数会移除该前缀。
pub fn canonicalize_path(path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    let canonicalized = std::fs::canonicalize(path.as_ref())?;

    #[cfg(windows)]
    {
        let path_str = canonicalized.to_string_lossy();
        if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
            Ok(PathBuf::from(stripped))
        } else {
            Ok(canonicalized)
        }
    }

    #[cfg(not(windows))]
    {
        Ok(canonicalized)
    }
}

/// 优化路径显示，移除 Windows UNC 路径前缀
pub fn format_path(path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string()
}

/// Check if a path is absolute
pub fn is_absolute(path: &Path) -> bool {
    path.is_absolute()
}

/// Normalize a path by removing redundant components
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                if let Some(last) = components.last() {
                    if *last != std::path::Component::ParentDir {
                        components.pop();
                    } else {
                        components.push(component);
                    }
                } else {
                    components.push(component);
                }
            }
            std::path::Component::CurDir => {
                // Skip current directory components
            }
            _ => {
                components.push(component);
            }
        }
    }
    
    components.iter().collect()
}

/// Join multiple path components with proper handling for cross-platform compatibility
///
/// This function provides a more robust alternative to `Path::join()` that handles
/// edge cases like empty components and absolute paths more consistently.
pub fn join_paths(base: impl AsRef<Path>, components: &[impl AsRef<Path>]) -> PathBuf {
    let mut result = base.as_ref().to_path_buf();
    
    for component in components {
        let component_path = component.as_ref();
        if !component_path.as_os_str().is_empty() {
            result = result.join(component_path);
        }
    }
    
    result
}

/// Get the parent directory of a path, if it exists
///
/// Returns `None` if the path has no parent (e.g., root directory).
pub fn parent_dir(path: impl AsRef<Path>) -> Option<PathBuf> {
    path.as_ref().parent().map(|p| p.to_path_buf())
}

/// Get the file name from a path, if it exists
///
/// Returns `None` if the path has no file name (e.g., directory paths ending with separator).
pub fn file_name(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .file_name()
        .and_then(|os_str| os_str.to_str())
        .map(|s| s.to_string())
}

/// Get the file stem (name without extension) from a path, if it exists
pub fn file_stem(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .file_stem()
        .and_then(|os_str| os_str.to_str())
        .map(|s| s.to_string())
}

/// Get the file extension from a path, if it exists
pub fn file_extension(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .extension()
        .and_then(|os_str| os_str.to_str())
        .map(|s| s.to_string())
}

/// Check if a path is relative
pub fn is_relative(path: &Path) -> bool {
    !path.is_absolute()
}

/// Convert a path to a string, handling invalid UTF-8 gracefully
///
/// Returns `None` if the path contains invalid UTF-8 sequences.
pub fn path_to_string(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref().to_str().map(|s| s.to_string())
}

/// Check if two paths are equivalent after normalization
///
/// This function normalizes both paths (removing `.` and `..` components)
/// before comparing them.
pub fn paths_equal(a: impl AsRef<Path>, b: impl AsRef<Path>) -> bool {
    let a_normalized = normalize_path(a.as_ref());
    let b_normalized = normalize_path(b.as_ref());
    a_normalized == b_normalized
}

/// Get the common prefix between two paths
///
/// Returns the longest common prefix shared by both paths.
pub fn common_prefix(a: impl AsRef<Path>, b: impl AsRef<Path>) -> PathBuf {
    let a_components: Vec<_> = a.as_ref().components().collect();
    let b_components: Vec<_> = b.as_ref().components().collect();
    
    let mut common = PathBuf::new();
    for (a_comp, b_comp) in a_components.iter().zip(b_components.iter()) {
        if a_comp == b_comp {
            common.push(a_comp.as_os_str());
        } else {
            break;
        }
    }
    
    common
}

/// Make a path relative to a base directory
///
/// Returns `None` if the path is not relative to the base directory.
pub fn make_relative_to(path: impl AsRef<Path>, base: impl AsRef<Path>) -> Option<PathBuf> {
    let path = path.as_ref();
    let base = base.as_ref();
    
    path.strip_prefix(base).ok().map(|p| p.to_path_buf())
}

/// Check if a path is a child of a base directory
pub fn is_child_of(path: impl AsRef<Path>, base: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let base = base.as_ref();
    
    path.strip_prefix(base).is_ok()
}

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
    
    #[test]
    fn test_normalize_path() {
        let path = Path::new("/foo/../bar/./baz");
        let normalized = normalize_path(path);
        assert_eq!(normalized, Path::new("/bar/baz"));
    }
    
    #[test]
    fn test_join_paths() {
        let base = Path::new("/home/user");
        let components = [Path::new("projects"), Path::new("rust"), Path::new("src")];
        let joined = join_paths(base, &components);
        assert_eq!(joined, Path::new("/home/user/projects/rust/src"));
    }
    
    #[test]
    fn test_join_paths_with_empty() {
        let base = Path::new("/home/user");
        let components = [Path::new(""), Path::new("projects"), Path::new("")];
        let joined = join_paths(base, &components);
        assert_eq!(joined, Path::new("/home/user/projects"));
    }
    
    #[test]
    fn test_parent_dir() {
        let path = Path::new("/home/user/projects/file.txt");
        let parent = parent_dir(path);
        assert_eq!(parent, Some(PathBuf::from("/home/user/projects")));
        
        let root_path = Path::new("/");
        let root_parent = parent_dir(root_path);
        assert_eq!(root_parent, None);
    }
    
    #[test]
    fn test_file_name() {
        let path = Path::new("/home/user/projects/file.txt");
        let name = file_name(path);
        assert_eq!(name, Some("file.txt".to_string()));
        
        // Test with directory path - behavior depends on platform
        // On Unix, Path::new("/home/user/projects/") has no file name
        // On Windows, it might behave differently
        let dir_path = Path::new("/home/user/projects");
        let dir_name = file_name(dir_path);
        assert_eq!(dir_name, Some("projects".to_string()));
    }
    
    #[test]
    fn test_file_stem() {
        let path = Path::new("/home/user/projects/file.txt");
        let stem = file_stem(path);
        assert_eq!(stem, Some("file".to_string()));
        
        let path_no_ext = Path::new("/home/user/projects/file");
        let stem_no_ext = file_stem(path_no_ext);
        assert_eq!(stem_no_ext, Some("file".to_string()));
    }
    
    #[test]
    fn test_file_extension() {
        let path = Path::new("/home/user/projects/file.txt");
        let ext = file_extension(path);
        assert_eq!(ext, Some("txt".to_string()));
        
        let path_no_ext = Path::new("/home/user/projects/file");
        let ext_no_ext = file_extension(path_no_ext);
        assert_eq!(ext_no_ext, None);
    }
    
    #[test]
    fn test_is_relative() {
        #[cfg(unix)]
        {
            assert!(is_relative(Path::new("relative/path")));
            assert!(!is_relative(Path::new("/absolute/path")));
        }
        
        #[cfg(windows)]
        {
            assert!(is_relative(Path::new("relative\\path")));
            assert!(!is_relative(Path::new("C:\\absolute\\path")));
        }
    }
    
    #[test]
    fn test_paths_equal() {
        let path1 = Path::new("/home/user/../user/projects/./file.txt");
        let path2 = Path::new("/home/user/projects/file.txt");
        assert!(paths_equal(path1, path2));
        
        let path3 = Path::new("/home/user/projects/file.txt");
        let path4 = Path::new("/home/user/docs/file.txt");
        assert!(!paths_equal(path3, path4));
    }
    
    #[test]
    fn test_common_prefix() {
        let path1 = Path::new("/home/user/projects/rust/src");
        let path2 = Path::new("/home/user/projects/python/src");
        let common = common_prefix(path1, path2);
        assert_eq!(common, Path::new("/home/user/projects"));
        
        let path3 = Path::new("/home/user/docs");
        let path4 = Path::new("/home/user/projects");
        let common2 = common_prefix(path3, path4);
        assert_eq!(common2, Path::new("/home/user"));
    }
    
    #[test]
    fn test_make_relative_to() {
        let base = Path::new("/home/user");
        let path = Path::new("/home/user/projects/rust/src");
        let relative = make_relative_to(path, base);
        assert_eq!(relative, Some(PathBuf::from("projects/rust/src")));
        
        let unrelated = Path::new("/var/log");
        let relative_unrelated = make_relative_to(unrelated, base);
        assert_eq!(relative_unrelated, None);
    }
    
    #[test]
    fn test_is_child_of() {
        let base = Path::new("/home/user");
        let child = Path::new("/home/user/projects/rust");
        assert!(is_child_of(child, base));
        
        let not_child = Path::new("/var/log");
        assert!(!is_child_of(not_child, base));
        
        let same = Path::new("/home/user");
        assert!(is_child_of(same, base));
    }
}