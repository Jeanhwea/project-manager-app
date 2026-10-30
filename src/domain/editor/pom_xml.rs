//! pom.xml 编辑器。
//! 版本号定位策略：匹配第一个 `<version>...</version>` 标签对，
//! 单模块 pom 中它就是项目版本；多模块或带 <parent> 的 pom 需人工确认。

use super::{EditorError, FileEditor, Result, VersionPosition, extract_version_position};

/// 处理 pom.xml 的 <version> 标签。
pub struct PomXmlEditor;

impl PomXmlEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern = regex::Regex::new(r#"<version>([^<]+)</version>"#).ok()?;
        extract_version_position(content, &pattern)
    }
}

impl FileEditor for PomXmlEditor {
    fn name(&self) -> &str {
        "pom.xml"
    }

    fn file_patterns(&self) -> &[&str] {
        &["pom.xml"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        Self::find_version_position(content)
    }

    fn validate(&self, _original: &str, edited: &str) -> Result<()> {
        if !edited.contains("<version>") || !edited.contains("</version>") {
            return Err(EditorError::FormatPreservationError(
                "pom.xml format validation failed".to_string(),
            ));
        }

        Ok(())
    }
}
