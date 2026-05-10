use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition,
    find_version_value_in_quotes, replace_at_position,
};
use std::path::Path;

pub struct PomXmlEditor;

impl PomXmlEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern = regex::Regex::new(r#"<version>([^<]+)</version>"#).ok()?;
        find_version_value_in_quotes(content, &pattern)
    }
}

impl FileEditor for PomXmlEditor {
    fn file_patterns(&self) -> &[&str] {
        &["pom.xml"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "pom.xml")
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation> {
        let project_version = Self::find_version_position(content);

        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(
                "pom.xml does not have version field".to_string(),
            ));
        }

        Ok(VersionLocation {
            project_version,
            is_workspace_root: false,
        })
    }

    fn edit(
        &self,
        content: &str,
        location: &VersionLocation,
        new_version: &str,
    ) -> Result<String> {
        if let Some(ref pos) = location.project_version {
            Ok(replace_at_position(content, pos, new_version))
        } else {
            Err(EditorError::VersionNotFound(
                "pom.xml does not have version field".to_string(),
            ))
        }
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
