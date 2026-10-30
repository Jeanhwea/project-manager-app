//! 配置文件的路径解析、加载与缓存，以及旧版 `~/.pma.toml` 的一次性迁移。

use super::schema::{AppConfig, GitlabConfig};
use std::io;
use std::path::PathBuf;
use std::sync::OnceLock;

/// 读取并解析一个 TOML 配置；文件不存在、读不出或解析失败都返回 `T::default()`。
// 配置文件是可选输入，任何问题都只警告不中断，否则一处写错会让所有子命令不可用
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

/// pma 全局配置的访问入口，所有方法都是无状态的静态方法
pub struct ConfigManager;

impl ConfigManager {
    /// 配置根目录 `~/.pma`；取不到 HOME 时退回当前目录
    pub fn base_dir() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".pma")
    }

    /// 主配置 `~/.pma/config.toml`
    pub fn config_path() -> PathBuf {
        Self::base_dir().join("config.toml")
    }

    /// GitLab 服务器凭据 `~/.pma/gitlab.toml`，与主配置分开存放以避免 token 泄漏到普通配置里
    pub fn gitlab_path() -> PathBuf {
        Self::base_dir().join("gitlab.toml")
    }

    /// 迁移前的旧配置路径 `~/.pma.toml`
    pub fn legacy_config_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".pma.toml")
    }

    /// 确保配置目录存在，写配置文件前调用
    pub fn ensure_dir() -> io::Result<()> {
        let dir = Self::base_dir();
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }
        Ok(())
    }

    // 缓存在 OnceLock 里：同一次运行会被大量仓库反复访问，不应每次都读盘；
    // 代价是运行中手工改配置文件要等下次运行才生效
    /// 加载主配置；进程内只加载一次，后续调用直接返回缓存副本
    pub fn load_config() -> AppConfig {
        static CONFIG: OnceLock<AppConfig> = OnceLock::new();
        CONFIG
            .get_or_init(|| {
                Self::migrate_legacy_config();
                load_toml_config::<AppConfig>(&Self::config_path(), "")
            })
            .clone()
    }

    /// 加载 GitLab 服务器列表，同样只加载一次
    pub fn load_gitlab() -> GitlabConfig {
        static GITLAB_CONFIG: OnceLock<GitlabConfig> = OnceLock::new();
        GITLAB_CONFIG
            .get_or_init(|| load_toml_config::<GitlabConfig>(&Self::gitlab_path(), " GitLab"))
            .clone()
    }

    /// 把 `~/.pma.toml` 搬到 `~/.pma/config.toml`
    fn migrate_legacy_config() {
        let legacy_path = Self::legacy_config_path();
        let new_path = Self::config_path();

        // 已有新配置就不动旧文件，避免覆盖用户已经改过的配置
        if !legacy_path.exists() || new_path.exists() {
            return;
        }

        // 任一步失败就整体放弃（保留旧文件），下次运行会再试一次
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
