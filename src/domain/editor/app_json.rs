use super::{EditorError, FileEditor, Result, VersionPosition, extract_version_position};

pub struct AppJsonEditor;

impl AppJsonEditor {
    fn find_version_position(content: &str) -> Option<VersionPosition> {
        let pattern =
            regex::Regex::new(r#""expo"\s*:\s*\{[\s\S]*?"version"\s*:\s*"([^"]*)""#).ok()?;
        extract_version_position(content, &pattern)
    }
}

impl FileEditor for AppJsonEditor {
    fn name(&self) -> &str {
        "app.json"
    }

    fn file_patterns(&self) -> &[&str] {
        &["app.json"]
    }

    fn find_version(&self, content: &str) -> Option<VersionPosition> {
        Self::find_version_position(content)
    }

    fn parse(&self, content: &str) -> Result<super::VersionLocation> {
        let value: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| EditorError::ParseError(format!("Failed to parse app.json: {}", e)))?;

        if value.get("expo").is_none() {
            return Err(EditorError::VersionNotFound(
                "app.json is not an Expo config (missing expo field)".to_string(),
            ));
        }

        let project_version = self.find_version(content);

        if project_version.is_none() {
            return Err(EditorError::VersionNotFound(
                "app.json does not have expo.version field".to_string(),
            ));
        }

        Ok(super::VersionLocation {
            project_version,
            is_workspace_root: false,
        })
    }

    fn validate(&self, original: &str, edited: &str) -> Result<()> {
        if serde_json::from_str::<serde_json::Value>(edited).is_err() {
            return Err(EditorError::FormatPreservationError(
                "app.json format validation failed".to_string(),
            ));
        }

        let original_len = original.len();
        let edited_len = edited.len();
        if edited_len.abs_diff(original_len) > original_len / 2 {
            return Err(EditorError::FormatPreservationError(
                "app.json changed too much".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_json_editor_basic() {
        let content = r#"{
  "expo": {
    "name": "my-room-things",
    "slug": "my-room-things",
    "version": "1.0.0",
    "orientation": "portrait",
    "icon": "./assets/images/icon.png",
    "scheme": "myroomthings",
    "userInterfaceStyle": "automatic"
  }
}"#;

        let editor = AppJsonEditor;
        let location = editor.parse(content).unwrap();
        assert!(location.project_version.is_some());

        let edited = editor.edit(content, &location, "1.1.0").unwrap();
        assert!(edited.contains("\"version\": \"1.1.0\""));
        assert!(!edited.contains("\"version\": \"1.0.0\""));
        assert!(edited.contains("\"name\": \"my-room-things\""));
        editor.validate(content, &edited).unwrap();
    }

    #[test]
    fn test_app_json_editor_ignores_outer_version() {
        let content = r#"{
  "version": "9.9.9",
  "expo": {
    "name": "app",
    "version": "1.0.0"
  }
}"#;

        let editor = AppJsonEditor;
        let location = editor.parse(content).unwrap();
        let edited = editor.edit(content, &location, "2.0.0").unwrap();
        assert!(edited.contains("\"version\": \"9.9.9\""));
        assert!(edited.contains("\"version\": \"2.0.0\""));
        assert!(!edited.contains("\"version\": \"1.0.0\""));
    }

    #[test]
    fn test_app_json_editor_no_expo() {
        let content = r#"{
  "name": "app"
}"#;
        let editor = AppJsonEditor;
        assert!(editor.parse(content).is_err());
    }

    #[test]
    fn test_app_json_editor_invalid_json() {
        let content = "{ not json";
        let editor = AppJsonEditor;
        assert!(editor.parse(content).is_err());
    }
}
