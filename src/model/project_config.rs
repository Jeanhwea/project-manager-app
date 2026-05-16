use serde::{Deserialize, Serialize};

pub const PROJECT_CONFIG_FILE: &str = ".pma.json";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub files: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_empty() {
        let cfg = ProjectConfig::default();
        assert!(cfg.files.is_empty());
    }

    #[test]
    fn test_serialize_round_trip() {
        let cfg = ProjectConfig {
            files: vec!["Cargo.toml".to_string(), "package.json".to_string()],
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: ProjectConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.files, cfg.files);
    }

    #[test]
    fn test_deserialize_missing_files_field() {
        let parsed: ProjectConfig = serde_json::from_str("{}").unwrap();
        assert!(parsed.files.is_empty());
    }
}
