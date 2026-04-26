use super::{ConfigEditor, VersionEditError, VersionLocation, VersionPosition};
use std::path::Path;

pub struct CargoTomlEditor;

impl CargoTomlEditor {
    fn find_version_position(
        content: &str,
        doc: &toml_edit::DocumentMut,
    ) -> Option<VersionPosition> {
        let package = doc.get("package")?.as_table_like()?;

        if !package.contains_key("version") {
            return None;
        }

        let version_pattern = regex::Regex::new(r#"version\s*=\s*"[^"]*""#).ok()?;

        let package_start = content.find("[package]")?;
        let package_end = content[package_start..]
            .find("\n[")
            .map(|p| package_start + p)
            .unwrap_or(content.len());

        let package_section = &content[package_start..package_end];

        if let Some(m) = version_pattern.find(package_section) {
            let start = package_start + m.start();
            let end = package_start + m.end();
            let line = content[..start].chars().filter(|&c| c == '\n').count() + 1;
            return Some(VersionPosition { start, end, line });
        }

        None
    }
}

impl ConfigEditor for CargoTomlEditor {
    fn name(&self) -> &'static str {
        "cargo_toml"
    }

    fn file_patterns(&self) -> &[&str] {
        &["Cargo.toml"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "Cargo.toml")
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation, VersionEditError> {
        let doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
            VersionEditError::ParseError {
                file: "Cargo.toml".to_string(),
                reason: e.to_string(),
            }
        })?;

        let has_package = doc.contains_key("package");
        let has_workspace = doc.contains_key("workspace");

        if !has_package && has_workspace {
            return Ok(VersionLocation {
                project_version: None,
                parent_version: None,
                is_workspace_root: true,
                dependency_refs: Vec::new(),
            });
        }

        if !has_package {
            return Err(VersionEditError::VersionNotFound {
                file: "Cargo.toml".to_string(),
                hint: "Cargo.toml 未找到 [package] section。".to_string(),
            });
        }

        let project_version = Self::find_version_position(content, &doc);

        if project_version.is_none() {
            return Err(VersionEditError::VersionNotFound {
                file: "Cargo.toml".to_string(),
                hint: "Cargo.toml [package] section 中未找到 version 字段。".to_string(),
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
        if location.is_workspace_root {
            return Err(VersionEditError::VersionNotFound {
                file: "Cargo.toml".to_string(),
                hint: "Cargo.toml 是 workspace 根文件，无项目版本。请指定具体的 member package。"
                    .to_string(),
            });
        }

        let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
            VersionEditError::ParseError {
                file: "Cargo.toml".to_string(),
                reason: e.to_string(),
            }
        })?;

        if let Some(package) = doc.get_mut("package")
            && let Some(table) = package.as_table_like_mut()
        {
            table.insert("version", toml_edit::value(new_version));
        }

        Ok(doc.to_string())
    }

    fn validate(&self, original: &str, edited: &str) -> Result<(), VersionEditError> {
        if edited.parse::<toml_edit::DocumentMut>().is_err() {
            return Err(VersionEditError::FormatPreservationError {
                file: "Cargo.toml".to_string(),
            });
        }

        let original_has_crlf = original.contains("\r\n");
        let edited_has_crlf = edited.contains("\r\n");
        if original_has_crlf != edited_has_crlf && original_has_crlf {
            return Err(VersionEditError::FormatPreservationError {
                file: "Cargo.toml".to_string(),
            });
        }

        Ok(())
    }
}
