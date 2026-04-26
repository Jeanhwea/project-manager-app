use super::{ConfigEditor, VersionEditError, VersionLocation, VersionPosition};
use std::path::Path;

pub struct HomebrewFormulaEditor;

impl HomebrewFormulaEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let version_pattern = regex::Regex::new(r#"version\s+"[^"]*""#).ok()?;
        if let Some(m) = version_pattern.find(content) {
            let match_str = m.as_str();
            let start = m.start();

            if let Some(quote_pos) = match_str.find('"') {
                let version_start = start + quote_pos + 1;
                let version_end = match_str.rfind('"')?;
                let end = start + version_end;
                let line = content[..version_start]
                    .chars()
                    .filter(|&c| c == '\n')
                    .count()
                    + 1;
                return Some(VersionPosition {
                    start: version_start,
                    end,
                    line,
                });
            }
        }
        None
    }
}

impl ConfigEditor for HomebrewFormulaEditor {
    fn name(&self) -> &'static str {
        "homebrew"
    }

    fn file_patterns(&self) -> &[&str] {
        &["Formula/pma.rb"]
    }

    fn matches_file(&self, path: &Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let parent = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());
        file_name == "pma.rb" && parent == Some("Formula")
    }

    fn parse(&self, content: &str) -> Result<VersionLocation, VersionEditError> {
        let project_version = Self::find_version_position(content);

        if project_version.is_none() {
            return Err(VersionEditError::VersionNotFound {
                file: "Homebrew formula".to_string(),
                hint: "未找到 version 声明。".to_string(),
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
            Ok(result)
        } else {
            Err(VersionEditError::VersionNotFound {
                file: "Homebrew formula".to_string(),
                hint: "未找到 version 声明。".to_string(),
            })
        }
    }

    fn validate(&self, original: &str, edited: &str) -> Result<(), VersionEditError> {
        let original_has_crlf = original.contains("\r\n");
        let edited_has_crlf = edited.contains("\r\n");
        if original_has_crlf != edited_has_crlf && original_has_crlf {
            return Err(VersionEditError::FormatPreservationError {
                file: "Homebrew formula".to_string(),
            });
        }
        Ok(())
    }
}
