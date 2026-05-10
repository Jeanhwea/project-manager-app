use super::{
    EditorError, FileEditor, Result, VersionLocation, VersionPosition, replace_at_position,
};
use std::path::Path;

pub struct CMakeListsEditor;

impl CMakeListsEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let version_pattern =
            regex::Regex::new(r#"project\s*\([^)]*?VERSION\s+([0-9]+\.[0-9]+\.[0-9]+)"#).ok()?;
        if let Some(caps) = version_pattern.captures(content)
            && let Some(version_match) = caps.get(1)
        {
            let start = version_match.start();
            let end = version_match.end();
            return Some(VersionPosition { start, end });
        }
        None
    }
}

impl FileEditor for CMakeListsEditor {
    fn file_patterns(&self) -> &[&str] {
        &["CMakeLists.txt"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "CMakeLists.txt")
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation> {
        let project_version = Self::find_version_position(content);

        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(
                "CMakeLists.txt does not have project(VERSION ...) declaration".to_string(),
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
                "CMakeLists.txt does not have project(VERSION ...) declaration".to_string(),
            ))
        }
    }

    fn validate(&self, _original: &str, _edited: &str) -> Result<()> {
        Ok(())
    }
}
