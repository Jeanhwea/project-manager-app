//! package.json 编辑器。
//! 版本号定位策略：匹配第一次出现的 `"version": "..."`。
//! 改写走纯文本替换而非 serde_json 重新序列化，以此保留原始的缩进、换行与键顺序。

use super::{EditorError, FileEditor, Result, VersionPosition, extract_version_position};

/// 处理 package.json 的顶层 version 字段。
pub struct PackageJsonEditor;

impl PackageJsonEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern = regex::Regex::new(r#""version"\s*:\s*"([^"]*)""#).ok()?;
        extract_version_position(content, &pattern)
    }
}

impl FileEditor for PackageJsonEditor {
    fn name(&self) -> &str {
        "package.json"
    }

    fn file_patterns(&self) -> &[&str] {
        &["package.json"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        Self::find_version_position(content)
    }

    fn parse(&self, content: &str) -> Result<super::VersionLocation> {
        // 只做 JSON 合法性校验，不参与改写：一旦重新序列化就会丢失缩进与键顺序
        let _: serde_json::Value = serde_json::from_str(content).map_err(|e| {
            EditorError::ParseError(format!("Failed to parse package.json: {}", e))
        })?;

        let project_version = self.find_version(content);

        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(
                "package.json does not have version field".to_string(),
            ));
        }

        Ok(super::VersionLocation {
            project_version,
            is_workspace_root: false,
        })
    }

    fn validate(&self, original: &str, edited: &str) -> Result<()> {
        if serde_json::from_str::<serde_json::Value>(edited).is_err() {
            return Err(EditorError::FormatPreservationError(
                "package.json format validation failed".to_string(),
            ));
        }

        // 纯文本替换只应带来版本号长度的变化，长度突变说明定位错了
        let original_len = original.len();
        let edited_len = edited.len();
        if edited_len.abs_diff(original_len) > original_len / 2 {
            return Err(EditorError::FormatPreservationError(
                "package.json changed too much".to_string(),
            ));
        }

        Ok(())
    }
}
