use super::{ConfigEditor, VersionEditError, VersionLocation, VersionPosition};
use std::path::Path;

pub struct PyprojectEditor;

impl PyprojectEditor {
    fn find_version_in_section(
        content: &str,
        doc: &toml_edit::DocumentMut,
        section_path: &[&str],
    ) -> Option<VersionPosition> {
        let mut current: Option<&toml_edit::Item> = None;
        for key in section_path {
            current = match current {
                None => doc.get(key),
                Some(item) => item.get(key),
            };
        }

        let table = current?.as_table_like()?;
        if !table.contains_key("version") {
            return None;
        }

        let section_header = if section_path.len() == 1 {
            format!("[{}]", section_path[0])
        } else {
            format!("[{}]", section_path.join("."))
        };

        let section_start = content.find(&section_header)?;
        let section_end = content[section_start..]
            .find("\n[")
            .map(|p| section_start + p)
            .unwrap_or(content.len());

        let section_content = &content[section_start..section_end];

        let version_pattern = regex::Regex::new(r#"version\s*=\s*"[^"]*""#).ok()?;
        if let Some(m) = version_pattern.find(section_content) {
            let start = section_start + m.start();
            let end = section_start + m.end();
            let line = content[..start].chars().filter(|&c| c == '\n').count() + 1;
            return Some(VersionPosition { start, end, line });
        }

        None
    }
}

impl ConfigEditor for PyprojectEditor {
    fn name(&self) -> &'static str {
        "pyproject"
    }

    fn file_patterns(&self) -> &[&str] {
        &["pyproject.toml"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "pyproject.toml")
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation, VersionEditError> {
        let doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
            VersionEditError::ParseError {
                file: "pyproject.toml".to_string(),
                reason: e.to_string(),
            }
        })?;

        if doc.contains_key("project") {
            let project_version = Self::find_version_in_section(content, &doc, &["project"]);
            if project_version.is_some() {
                return Ok(VersionLocation {
                    project_version,
                    parent_version: None,
                    is_workspace_root: false,
                    dependency_refs: Vec::new(),
                });
            }
        }

        if doc.contains_key("tool")
            && let Some(tool) = doc.get("tool")
            && let Some(tool_table) = tool.as_table_like()
            && tool_table.contains_key("poetry")
        {
            let project_version =
                Self::find_version_in_section(content, &doc, &["tool", "poetry"]);
            if project_version.is_some() {
                return Ok(VersionLocation {
                    project_version,
                    parent_version: None,
                    is_workspace_root: false,
                    dependency_refs: Vec::new(),
                });
            }
        }

        Err(VersionEditError::VersionNotFound {
            file: "pyproject.toml".to_string(),
            hint: "pyproject.toml 未找到版本字段。请确保文件包含 [project] 或 [tool.poetry] section。".to_string(),
        })
    }

    fn edit(
        &self,
        content: &str,
        _location: &VersionLocation,
        new_version: &str,
    ) -> Result<String, VersionEditError> {
        let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
            VersionEditError::ParseError {
                file: "pyproject.toml".to_string(),
                reason: e.to_string(),
            }
        })?;

        if doc.contains_key("project")
            && let Some(project) = doc.get_mut("project")
            && let Some(table) = project.as_table_like_mut()
            && table.contains_key("version")
        {
            table.insert("version", toml_edit::value(new_version));
            return Ok(doc.to_string());
        }

        if doc.contains_key("tool")
            && let Some(tool) = doc.get_mut("tool")
            && let Some(tool_table) = tool.as_table_like_mut()
            && tool_table.contains_key("poetry")
            && let Some(poetry) = tool_table.get_mut("poetry")
            && let Some(poetry_table) = poetry.as_table_like_mut()
            && poetry_table.contains_key("version")
        {
            poetry_table.insert("version", toml_edit::value(new_version));
            return Ok(doc.to_string());
        }

        Err(VersionEditError::VersionNotFound {
            file: "pyproject.toml".to_string(),
            hint: "pyproject.toml 未找到版本字段。".to_string(),
        })
    }

    fn validate(&self, original: &str, edited: &str) -> Result<(), VersionEditError> {
        if edited.parse::<toml_edit::DocumentMut>().is_err() {
            return Err(VersionEditError::FormatPreservationError {
                file: "pyproject.toml".to_string(),
            });
        }

        let original_has_crlf = original.contains("\r\n");
        let edited_has_crlf = edited.contains("\r\n");
        if original_has_crlf != edited_has_crlf && original_has_crlf {
            return Err(VersionEditError::FormatPreservationError {
                file: "pyproject.toml".to_string(),
            });
        }

        Ok(())
    }
}
