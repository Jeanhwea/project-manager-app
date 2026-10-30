//! pyproject.toml 编辑器。
//! 版本号定位策略：按 TOML section 切分，支持 PEP 621 的 `[project]` 与 Poetry 的
//! `[tool.poetry]` 两种布局，优先 `[project]`，两者都没有 version 时报错。

use super::{EditorError, FileEditor, Result, VersionPosition};

/// 处理 pyproject.toml 的项目版本号，兼容 PEP 621 与 Poetry 布局。
pub struct PyprojectEditor;

impl PyprojectEditor {
    fn find_version_in_section(content: &str, section_header: &str) -> Option<VersionPosition> {
        let section_start = content.find(section_header)?;
        // section 边界为下一个 "\n["；没有后续 section 时以文件末尾兜底，
        // 这样不会匹配到后面 section（例如 [tool.x]）里的 version
        let section_end = content[section_start..]
            .find("\n[")
            .map(|p| section_start + p)
            .unwrap_or(content.len());

        let section_content = &content[section_start..section_end];

        let pattern = regex::Regex::new(r#"version\s*=\s*"([^"]*)""#).ok()?;
        let caps = pattern.captures(section_content)?;
        let version_match = caps.get(1)?;

        Some(VersionPosition {
            start: section_start + version_match.start(),
            end: section_start + version_match.end(),
        })
    }
}

impl FileEditor for PyprojectEditor {
    fn name(&self) -> &str {
        "pyproject.toml"
    }

    fn file_patterns(&self) -> &[&str] {
        &["pyproject.toml"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        Self::find_version_in_section(content, "[project]")
            .or_else(|| Self::find_version_in_section(content, "[tool.poetry]"))
    }

    fn parse(&self, content: &str) -> Result<super::VersionLocation> {
        let doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
            EditorError::ParseError(format!("Failed to parse pyproject.toml: {}", e))
        })?;

        // 先用 toml_edit 确认 section 存在，再回到原文按位置替换，保证不改格式
        if doc.contains_key("project") {
            let project_version = Self::find_version_in_section(content, "[project]");
            if project_version.is_some() {
                return Ok(super::VersionLocation {
                    project_version,
                    is_workspace_root: false,
                });
            }
        }

        if doc.contains_key("tool")
            && let Some(tool) = doc.get("tool")
            && let Some(tool_table) = tool.as_table_like()
            && tool_table.contains_key("poetry")
        {
            let project_version = Self::find_version_in_section(content, "[tool.poetry]");
            if project_version.is_some() {
                return Ok(super::VersionLocation {
                    project_version,
                    is_workspace_root: false,
                });
            }
        }

        Err(EditorError::VersionNotFound(
            "pyproject.toml does not have version field in [project] or [tool.poetry] section"
                .to_string(),
        ))
    }

    fn validate(&self, _original: &str, edited: &str) -> Result<()> {
        if edited.parse::<toml_edit::DocumentMut>().is_err() {
            return Err(EditorError::FormatPreservationError(
                "pyproject.toml format validation failed".to_string(),
            ));
        }
        Ok(())
    }
}
