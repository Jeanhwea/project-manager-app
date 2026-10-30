//! 纯文本版本文件编辑器（version.txt、名为 version 的文件）。
//! 版本号定位策略：直接匹配文件中第一次出现的 `x.y.z`，无需关心任何语法结构。

use super::{EditorError, FileEditor, Result, VersionPosition};

/// 处理只含一个版本号的纯文本文件。
pub struct VersionTextEditor;

impl FileEditor for VersionTextEditor {
    fn name(&self) -> &str {
        "version text file"
    }

    fn file_patterns(&self) -> &[&str] {
        &["version.txt", "version"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        let version_pattern = regex::Regex::new(r#"\d+\.\d+\.\d+"#).ok()?;

        if let Some(m) = version_pattern.find(content) {
            return Some(VersionPosition {
                start: m.start(),
                end: m.end(),
            });
        }

        None
    }

    fn validate(&self, original: &str, edited: &str) -> Result<()> {
        // 原文为空时 original.len() / 2 为 0，长度检查会误判任何改动，
        // 因此原文与改写结果都为空时直接放行
        if original.is_empty() && edited.is_empty() {
            return Ok(());
        }

        // 纯文本替换只应带来版本号长度的变化，长度突变说明定位错了
        if edited.len().abs_diff(original.len()) > original.len() / 2 {
            return Err(EditorError::FormatPreservationError(
                "Text file changed too much".to_string(),
            ));
        }

        Ok(())
    }
}
