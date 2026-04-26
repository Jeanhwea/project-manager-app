use super::{preserve_line_endings, ConfigEditor, VersionEditError, VersionLocation, VersionPosition};
use std::path::Path;

pub struct PythonVersionEditor;

impl PythonVersionEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let version_pattern = regex::Regex::new(r#"__version__\s*=\s*"[^"]*""#).ok()?;
        if let Some(m) = version_pattern.find(content) {
            let start = m.start();
            let end = m.end();
            let line = content[..start].chars().filter(|&c| c == '\n').count() + 1;
            return Some(VersionPosition { start, end, line });
        }
        None
    }
}

impl ConfigEditor for PythonVersionEditor {
    fn name(&self) -> &'static str {
        "python_version"
    }

    fn file_patterns(&self) -> &[&str] {
        &["__version__.py"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "__version__.py" || n.ends_with(".py"))
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<VersionLocation, VersionEditError> {
        let project_version = Self::find_version_position(content);

        if project_version.is_none() {
            return Err(VersionEditError::VersionNotFound {
                file: "Python version file".to_string(),
                hint: "未找到 __version__ 变量定义。".to_string(),
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
            result.push_str(&format!(r#"__version__ = "{}""#, new_version));
            result.push_str(&content[pos.end..]);
            Ok(preserve_line_endings(content, result))
        } else {
            Err(VersionEditError::VersionNotFound {
                file: "Python version file".to_string(),
                hint: "未找到 __version__ 变量定义。".to_string(),
            })
        }
    }

    fn validate(&self, _original: &str, _edited: &str) -> Result<(), VersionEditError> {
        Ok(())
    }
}
