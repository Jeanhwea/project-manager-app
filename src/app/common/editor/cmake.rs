use super::{
    ConfigEditor, VersionEditError, VersionLocation, VersionPosition, preserve_line_endings,
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
            let line = content[..start].chars().filter(|&c| c == '\n').count() + 1;
            return Some(VersionPosition { start, end, line });
        }
        None
    }
}

impl ConfigEditor for CMakeListsEditor {
    fn name(&self) -> &'static str {
        "cmake"
    }

    fn file_patterns(&self) -> &[&str] {
        &["CMakeLists.txt"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "CMakeLists.txt")
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation, VersionEditError> {
        let project_version = Self::find_version_position(content);

        if project_version.is_none() {
            return Err(VersionEditError::VersionNotFound {
                file: "CMakeLists.txt".to_string(),
                hint: "未找到 project(VERSION ...) 声明。".to_string(),
            });
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
    ) -> Result<String, VersionEditError> {
        if let Some(ref pos) = location.project_version {
            let mut result = String::new();
            result.push_str(&content[..pos.start]);
            result.push_str(new_version);
            result.push_str(&content[pos.end..]);
            Ok(preserve_line_endings(content, result))
        } else {
            Err(VersionEditError::VersionNotFound {
                file: "CMakeLists.txt".to_string(),
                hint: "未找到 project(VERSION ...) 声明。".to_string(),
            })
        }
    }

    fn validate(&self, _original: &str, _edited: &str) -> Result<(), VersionEditError> {
        Ok(())
    }
}
