use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition, preserve_line_endings,
};
use std::path::Path;

pub struct PythonVersionEditor;

impl PythonVersionEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let version_pattern = regex::Regex::new(r#"__version__\s*=\s*["']([^"']+)["']"#).ok()?;

        if let Some(m) = version_pattern.find(content) {
            let start = m.start();
            let end = m.end();
            let line = content[..start].chars().filter(|&c| c == '\n').count() + 1;
            return Some(VersionPosition { start, end, line });
        }

        None
    }
}

impl FileEditor for PythonVersionEditor {
    fn name(&self) -> &'static str {
        "project_py"
    }

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
            parent_version: None,
            is_workspace_root: false,
            dependency_refs: Vec::new(),
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
            result.push_str(&format!("__version__ = \"{}\"", new_version));
            result.push_str(&content[pos.end..]);
            Ok(preserve_line_endings(content, result))
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
