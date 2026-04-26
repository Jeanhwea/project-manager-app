use super::{
    ConfigEditor, DependencyRef, VersionEditError, VersionLocation, VersionPosition,
    preserve_line_endings,
};
use std::path::Path;

pub struct PackageJsonEditor {
    pub in_npm_dir: bool,
}

impl PackageJsonEditor {
    fn find_version_position(
        content: &str,
        value: &serde_json::Value,
    ) -> Option<VersionPosition> {
        let obj = value.as_object()?;
        if !obj.contains_key("version") {
            return None;
        }

        let version_pattern = regex::Regex::new(r#""version"\s*:\s*"[^"]*""#).ok()?;
        if let Some(m) = version_pattern.find(content) {
            let start = m.start();
            let end = m.end();
            let line = content[..start].chars().filter(|&c| c == '\n').count() + 1;
            return Some(VersionPosition { start, end, line });
        }

        None
    }

    fn find_dependency_refs(content: &str, value: &serde_json::Value) -> Vec<DependencyRef> {
        let mut refs = Vec::new();
        let obj = match value.as_object() {
            Some(o) => o,
            None => return refs,
        };

        let dep_sections = ["dependencies", "devDependencies", "optionalDependencies"];

        for section in dep_sections {
            if let Some(deps) = obj.get(section).and_then(|d| d.as_object()) {
                for key in deps.keys() {
                    if key.starts_with("@jeansoft/pma") {
                        let pattern = regex::Regex::new(&format!(
                            r#""{}"\s*:\s*"[^"]*""#,
                            regex::escape(key)
                        ))
                        .ok();

                        if let Some(re) = pattern
                            && let Some(m) = re.find(content)
                        {
                            let start = m.start();
                            let end = m.end();
                            let line =
                                content[..start].chars().filter(|&c| c == '\n').count() + 1;
                            refs.push(DependencyRef {
                                name_pattern: key.clone(),
                                position: VersionPosition { start, end, line },
                            });
                        }
                    }
                }
            }
        }

        refs
    }
}

impl ConfigEditor for PackageJsonEditor {
    fn name(&self) -> &'static str {
        "package_json"
    }

    fn file_patterns(&self) -> &[&str] {
        &["package.json", "tauri.conf.json"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "package.json" || n == "tauri.conf.json")
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation, VersionEditError> {
        let value: serde_json::Value =
            serde_json::from_str(content).map_err(|e| VersionEditError::ParseError {
                file: "package.json".to_string(),
                reason: e.to_string(),
            })?;

        let project_version = Self::find_version_position(content, &value);

        if project_version.is_none() {
            return Err(VersionEditError::VersionNotFound {
                file: "package.json".to_string(),
                hint: "package.json 未找到顶层 version 字段。".to_string(),
            });
        }

        let dependency_refs = if self.in_npm_dir {
            Self::find_dependency_refs(content, &value)
        } else {
            Vec::new()
        };

        Ok(VersionLocation {
            project_version,
            parent_version: None,
            is_workspace_root: false,
            dependency_refs,
        })
    }

    fn edit(
        &self,
        content: &str,
        location: &VersionLocation,
        new_version: &str,
    ) -> Result<String, VersionEditError> {
        let mut result = content.to_string();

        if location.project_version.is_some() {
            let version_pattern =
                regex::Regex::new(r#""version"\s*:\s*"[^"]*""#).map_err(|_| {
                    VersionEditError::ParseError {
                        file: "package.json".to_string(),
                        reason: "Failed to create version pattern".to_string(),
                    }
                })?;

            let new_version_str = format!(r#""version": "{}""#, new_version);
            result = version_pattern
                .replace(&result, &new_version_str)
                .to_string();
        }

        for dep_ref in &location.dependency_refs {
            let pattern = regex::Regex::new(&format!(
                r#""{}"\s*:\s*"[^"]*""#,
                regex::escape(&dep_ref.name_pattern)
            ))
            .map_err(|_| VersionEditError::ParseError {
                file: "package.json".to_string(),
                reason: "Failed to create dependency pattern".to_string(),
            })?;

            let new_dep_str = format!(r#""{}": "{}""#, dep_ref.name_pattern, new_version);
            result = pattern.replace(&result, &new_dep_str).to_string();
        }

        Ok(preserve_line_endings(content, result))
    }

    fn validate(&self, _original: &str, edited: &str) -> Result<(), VersionEditError> {
        if serde_json::from_str::<serde_json::Value>(edited).is_err() {
            return Err(VersionEditError::FormatPreservationError {
                file: "package.json".to_string(),
            });
        }
        Ok(())
    }
}
