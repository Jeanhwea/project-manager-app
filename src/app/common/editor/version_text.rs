use super::{preserve_line_endings, ConfigEditor, VersionEditError, VersionLocation, VersionPosition};
use std::path::Path;

pub struct VersionTextEditor;

impl ConfigEditor for VersionTextEditor {
    fn name(&self) -> &'static str {
        "version_text"
    }

    fn file_patterns(&self) -> &[&str] {
        &["version", "version.txt"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "version" || n == "version.txt")
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation, VersionEditError> {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(VersionEditError::VersionNotFound {
                file: "version text file".to_string(),
                hint: "版本文件为空。".to_string(),
            });
        }

        let start = content.find(trimmed).unwrap_or(0);
        let end = start + trimmed.len();
        let line = 1;

        Ok(VersionLocation {
            project_version: Some(VersionPosition { start, end, line }),
            parent_version: None,
            is_workspace_root: false,
            dependency_refs: Vec::new(),
        })
    }

    fn edit(
        &self,
        content: &str,
        _location: &VersionLocation,
        new_version: &str,
    ) -> Result<String, VersionEditError> {
        Ok(preserve_line_endings(content, new_version.to_string()))
    }

    fn validate(&self, _original: &str, _edited: &str) -> Result<(), VersionEditError> {
        Ok(())
    }
}
