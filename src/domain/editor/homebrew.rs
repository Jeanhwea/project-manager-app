//! Homebrew formula（*.rb）编辑器。
//! 版本号定位策略：匹配 Ruby 里 `version "x.y.z"` 形式的字面量，取第一次出现的位置。

use super::{EditorError, FileEditor, Result, VersionPosition, extract_version_position};
use std::path::Path;

/// 处理 Homebrew formula 的 version 字段。
pub struct HomebrewFormulaEditor;

impl HomebrewFormulaEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern = regex::Regex::new(r#"version\s+"([^"]+)""#).ok()?;
        extract_version_position(content, &pattern)
    }
}

impl FileEditor for HomebrewFormulaEditor {
    fn name(&self) -> &str {
        "Homebrew formula"
    }

    fn file_patterns(&self) -> &[&str] {
        &["{parent}.rb"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        Self::find_version_position(content)
    }

    // formula 文件名随包名变化（{parent}.rb）无法枚举，这里直接按 .rb 扩展名匹配，
    // 是否真的含 version 字段交给 parse / find_version 判断
    fn matches_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "rb")
            .unwrap_or(false)
    }

    fn validate(&self, _original: &str, edited: &str) -> Result<()> {
        // 空版本号说明替换位置不对或原文被破坏，交给上层报错而不是写出坏文件
        if edited.contains("version \"\"") {
            return Err(EditorError::FormatPreservationError(
                "Homebrew formula version field is empty".to_string(),
            ));
        }

        Ok(())
    }
}
