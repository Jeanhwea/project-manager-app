use super::{FileEditor, VersionPosition};

pub struct CMakeListsEditor;

impl FileEditor for CMakeListsEditor {
    fn name(&self) -> &str {
        "CMakeLists.txt"
    }

    fn file_patterns(&self) -> &[&str] {
        &["CMakeLists.txt"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        let version_pattern =
            regex::Regex::new(r#"project\s*\([^)]*?VERSION\s+([0-9]+\.[0-9]+\.[0-9]+)"#).ok()?;
        if let Some(caps) = version_pattern.captures(content)
            && let Some(version_match) = caps.get(1)
        {
            return Some(VersionPosition {
                start: version_match.start(),
                end: version_match.end(),
            });
        }
        None
    }
}
