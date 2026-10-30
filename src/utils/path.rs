//! 路径规整：统一 canonicalize 后的 Windows verbatim 前缀处理，以及面向展示的路径格式化。

use std::path::{Path, PathBuf};

/// 取绝对路径，并在 Windows 上去掉 canonicalize 自带的 `\\?\` 前缀
// `\\?\` 是 Windows 的 verbatim 路径标记，很多外部工具和展示场景都不认它
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

/// 给用户看的路径字符串，去掉 `\\?\` 前缀，不要求路径真实存在
pub fn format_path(path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string()
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
}
