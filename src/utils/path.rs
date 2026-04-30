//! Path manipulation utilities

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
}