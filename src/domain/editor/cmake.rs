//! CMakeLists.txt 编辑器。
//! 版本号定位策略：在 `project(...)` 调用内匹配 `VERSION` 后的三段式版本号，
//! 用 `[^)]*?` 把匹配限制在同一个 project() 括号内，避免命中其他命令里的数字。

use super::{FileEditor, VersionPosition};

/// 处理 CMakeLists.txt 的 project() VERSION 字段。
pub struct CMakeListsEditor;

impl FileEditor for CMakeListsEditor {
    fn name(&self) -> &str {
        "CMakeLists.txt"
    }

    fn file_patterns(&self) -> &[&str] {
        &["CMakeLists.txt"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        let version_pattern =
            regex::Regex::new(r#"project\s*\([^)]*?VERSION\s+([0-9]+\.[0-9]+\.[0-9]+)"#).ok()?;
        if let Some(caps) = version_pattern.captures(content)
            && let Some(version_match) = caps.get(1)
        {
            return Some(VersionPosition {
                start: version_match.start(),
                end: version_match.end(),
            });
        }
        None
    }
}
