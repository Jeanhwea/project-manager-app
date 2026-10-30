//! Python 版本文件编辑器（__init__.py / version.py / __version__.py）。
//! 版本号定位策略：匹配 `__version__ = "..."` 赋值语句，取第一次出现的位置。

use super::{EditorError, FileEditor, Result, VersionPosition, extract_version_position};

/// 处理 Python 源码里的 __version__ 赋值。
pub struct PythonVersionEditor;

impl PythonVersionEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        // 单双引号都要覆盖：两种写法在 Python 项目里都很常见
        let pattern = regex::Regex::new(r#"__version__\s*=\s*["']([^"']+)["']"#).ok()?;
        extract_version_position(content, &pattern)
    }
}

impl FileEditor for PythonVersionEditor {
    fn name(&self) -> &str {
        "Python version file"
    }

    fn file_patterns(&self) -> &[&str] {
        &["__init__.py", "version.py", "__version__.py"]
    }

    fn candidate_files(&self) -> Vec<&str> {
        // {} 会被展开成任意子目录名，用于定位放在包目录下的 __version__.py
        vec![
            "__init__.py",
            "version.py",
            "__version__.py",
            "{}/__version__.py",
        ]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        Self::find_version_position(content)
    }

    fn validate(&self, _original: &str, edited: &str) -> Result<()> {
        if edited.contains("__version__ = \"\"") {
            return Err(EditorError::FormatPreservationError(
                "Python version field is empty".to_string(),
            ));
        }

        Ok(())
    }
}
