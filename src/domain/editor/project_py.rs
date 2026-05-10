use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition,
    find_version_value_in_quotes, replace_at_position,
};
use std::path::Path;

pub struct PythonVersionEditor;

impl PythonVersionEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern = regex::Regex::new(r#"__version__\s*=\s*["']([^"']+)["']"#).ok()?;
        find_version_value_in_quotes(content, &pattern)
    }
}

impl FileEditor for PythonVersionEditor {
    fn file_patterns(&self) -> &[&str] {
        &["__init__.py", "version.py", "__version__.py"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "__init__.py" || n == "version.py" || n == "__version__.py")
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation> {
        let project_version = Self::find_version_position(content);

        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(
                "Python file does not have __version__ field".to_string(),
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
                "Python file does not have __version__ field".to_string(),
            ))
        }
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
