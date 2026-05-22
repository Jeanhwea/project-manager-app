use super::error::{EditorError, Result};
use super::position::{VersionLocation, VersionPosition, replace_at_position};
use std::path::Path;

pub trait FileEditor: Send + Sync {
    fn name(&self) -> &str;
    fn file_patterns(&self) -> &[&str];
    fn find_version(&self, content: &str) -> Option<VersionPosition>;

    fn candidate_files(&self) -> Vec<&str> {
        self.file_patterns().to_vec()
    }

    fn matches_file(&self, path: &Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        self.file_patterns().iter().any(|pattern| {
            if pattern.contains("{parent}") {
                let parent_dir = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let replaced = pattern.replace("{parent}", parent_dir);
                file_name == replaced || path.ends_with(&replaced)
            } else if pattern.contains("{}") {
                if let Some(parent) = path.parent() {
                    let parent_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let replaced = pattern.replace("{}", parent_name);
                    file_name == replaced || path.ends_with(&replaced)
                } else {
                    false
                }
            } else {
                file_name == *pattern || path.ends_with(pattern)
            }
        })
    }

    fn parse(&self, content: &str) -> Result<VersionLocation> {
        let project_version = self.find_version(content);
        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(format!(
                "{} does not have version field",
                self.name()
            )));
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
            Err(EditorError::VersionNotFound(format!(
                "{} does not have version field",
                self.name()
            )))
        }
    }

    fn validate(&self, _original: &str, _edited: &str) -> Result<()> {
        Ok(())
    }
}

pub fn write_with_backup(path: &str, content: &str) -> Result<()> {
    let tmp_path = format!("{}.tmp", path);
    std::fs::write(&tmp_path, content).map_err(EditorError::WriteError)?;
    std::fs::rename(&tmp_path, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        EditorError::WriteError(e)
    })
}
