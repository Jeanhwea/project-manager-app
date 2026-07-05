use crate::model::project_config::{PROJECT_CONFIG_FILE, ProjectConfig};
use std::path::{Path, PathBuf};

pub fn config_path(work_dir: &Path) -> PathBuf {
    work_dir.join(PROJECT_CONFIG_FILE)
}

pub fn load(work_dir: &Path) -> Option<ProjectConfig> {
    let path = config_path(work_dir);
    if !path.exists() {
        return None;
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<ProjectConfig>(&content) {
            Ok(config) => Some(config),
            Err(e) => {
                eprintln!("警告: 解析 {} 失败: {}", path.display(), e);
                None
            }
        },
        Err(e) => {
            eprintln!("警告: 无法读取 {}: {}", path.display(), e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_returns_none_when_file_missing() {
        let dir = tempdir().unwrap();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn test_load_parses_files_list() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join(PROJECT_CONFIG_FILE),
            r#"{"files": ["Cargo.toml", "src-tauri/tauri.conf.json"]}"#,
        )
        .unwrap();

        let cfg = load(dir.path()).expect("config should load");
        assert_eq!(
            cfg.files,
            vec![
                "Cargo.toml".to_string(),
                "src-tauri/tauri.conf.json".to_string()
            ]
        );
    }

    #[test]
    fn test_load_empty_files_field_is_ok() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join(PROJECT_CONFIG_FILE), r#"{}"#).unwrap();
        let cfg = load(dir.path()).expect("config should load");
        assert!(cfg.files.is_empty());
    }

    #[test]
    fn test_load_invalid_json_returns_none() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join(PROJECT_CONFIG_FILE), "{ not json").unwrap();
        assert!(load(dir.path()).is_none());
    }
}
