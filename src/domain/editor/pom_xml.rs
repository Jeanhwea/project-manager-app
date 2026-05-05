use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition, preserve_line_endings,
};
use std::path::Path;

pub struct PomXmlEditor;

impl PomXmlEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let version_pattern = regex::Regex::new(r#"<version>([^<]+)</version>"#).ok()?;

        if let Some(m) = version_pattern.find(content) {
            let start = m.start();
            let end = m.end();
            return Some(VersionPosition { start, end });
        }

        None
    }
}

impl FileEditor for PomXmlEditor {
    fn name(&self) -> &'static str {
        "pom_xml"
    }

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
            let mut result = String::new();
            result.push_str(&content[..pos.start]);
            result.push_str(&format!("<version>{}</version>", new_version));
            result.push_str(&content[pos.end..]);
            Ok(preserve_line_endings(content, result))
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
