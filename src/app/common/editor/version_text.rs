use super::{ConfigEditor, VersionEditError, VersionLocation, VersionPosition};

pub struct VersionTextEditor;

impl ConfigEditor for VersionTextEditor {
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
        _content: &str,
        _location: &VersionLocation,
        new_version: &str,
    ) -> Result<String, VersionEditError> {
        Ok(new_version.to_string())
    }

    fn validate(&self, _original: &str, _edited: &str) -> Result<(), VersionEditError> {
        Ok(())
    }
}
