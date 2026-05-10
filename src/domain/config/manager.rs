use super::schema::{AppConfig, GitLabConfig};
use std::io;
use std::path::PathBuf;
use std::sync::OnceLock;

fn load_toml_config<T: Default + serde::de::DeserializeOwned>(
    path: &std::path::Path,
    label: &str,
) -> T {
    if !path.exists() {
        return T::default();
    }
    match std::fs::read_to_string(path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                eprintln!(
                    "警告: {}配置文件解析失败 ({}): {}",
                    label,
                    path.display(),
                    e
                );
                T::default()
            }
        },
        Err(e) => {
            eprintln!(
                "警告: 无法读取{}配置文件 ({}): {}",
                label,
                path.display(),
                e
            );
            T::default()
        }
    }
}

pub struct ConfigDir;

impl ConfigDir {
    pub fn base_dir() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".pma")
    }

    pub fn config_path() -> PathBuf {
        Self::base_dir().join("config.toml")
    }

    pub fn gitlab_path() -> PathBuf {
        Self::base_dir().join("gitlab.toml")
    }

    pub fn legacy_config_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".pma.toml")
    }

    pub fn ensure_dir() -> io::Result<()> {
        let dir = Self::base_dir();
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }
        Ok(())
    }

    pub fn load_config() -> AppConfig {
        static CONFIG: OnceLock<AppConfig> = OnceLock::new();
        CONFIG
            .get_or_init(|| {
                Self::migrate_legacy_config();
                load_toml_config::<AppConfig>(&Self::config_path(), "")
            })
            .clone()
    }

    pub fn load_gitlab() -> GitLabConfig {
        load_toml_config::<GitLabConfig>(&Self::gitlab_path(), " GitLab")
    }

    pub fn save_gitlab(config: &GitLabConfig) -> io::Result<()> {
        Self::ensure_dir()?;
        let content = toml::to_string_pretty(config)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        std::fs::write(Self::gitlab_path(), content)
    }

    fn migrate_legacy_config() {
        let legacy_path = Self::legacy_config_path();
        let new_path = Self::config_path();

        if !legacy_path.exists() || new_path.exists() {
            return;
        }

        if let Ok(content) = std::fs::read_to_string(&legacy_path)
            && let Ok(config) = toml::from_str::<AppConfig>(&content)
            && Self::ensure_dir().is_ok()
            && let Ok(new_content) = toml::to_string_pretty(&config)
            && std::fs::write(&new_path, new_content).is_ok()
        {
            let _ = std::fs::remove_file(&legacy_path);
            eprintln!("已将旧配置文件迁移到 {}", new_path.display());
        }
    }
}
