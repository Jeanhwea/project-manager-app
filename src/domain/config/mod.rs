//! 全局配置：`~/.pma` 目录下的 config.toml 与 gitlab.toml。
//! 为仓库发现、remote 命名、sync 等各领域逻辑提供用户级默认值。

pub mod manager;
pub mod schema;

pub use manager::ConfigManager;
