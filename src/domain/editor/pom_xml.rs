use super::{EditorError, FileEditor, Result, VersionPosition, find_version_value_in_quotes};

pub struct PomXmlEditor;

impl PomXmlEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern = regex::Regex::new(r#"<version>([^<]+)</version>"#).ok()?;
        find_version_value_in_quotes(content, &pattern)
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
