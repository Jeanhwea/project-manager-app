use anyhow::{Context, Result};
use std::path::Path;

/// 优化路径显示，移除 Windows UNC 路径前缀
pub fn format_path(path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches("\\\\?\\")
        .to_string()
}

/// 将字符串转换为 kebab-case 格式
pub fn to_kebab_case(s: &str) -> String {
    s.chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if i > 0 && c.is_uppercase() {
                vec!['-', c.to_ascii_lowercase()]
            } else {
                vec![c.to_ascii_lowercase()]
            }
        })
        .collect::<String>()
}

pub fn get_current_dir() -> Result<String> {
    let current_dir = std::env::current_dir().context("无法获取当前目录")?;
    let canonical = current_dir
        .canonicalize()
        .context("无法规范化当前目录路径")?;
    let file_name = canonical.file_name().context("无法获取当前目录名称")?;
    Ok(file_name.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_current_dir() {
        let dir_name = get_current_dir().expect("应该能获取当前目录");
        assert!(!dir_name.is_empty(), "Directory name should not be empty");
    }

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
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("RedToolApp"), "red-tool-app");
        assert_eq!(to_kebab_case("MyProject"), "my-project");
        assert_eq!(to_kebab_case("Simple"), "simple");
        assert_eq!(to_kebab_case("ABC"), "a-b-c");
    }
}
