use std::sync::OnceLock;

use super::config::ConfigDir;
use super::config::schema::AppConfig;
use super::git::command::GitCommandRunner;

pub struct AppContext;

impl AppContext {
    pub fn git_runner() -> GitCommandRunner {
        GitCommandRunner::new()
    }

    pub fn config() -> AppConfig {
        static CONFIG: OnceLock<AppConfig> = OnceLock::new();
        CONFIG.get_or_init(ConfigDir::load_config).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_returns_valid_config() {
        let config = AppContext::config();
        assert_eq!(config.repository.max_depth, 3);
        assert!(!config.repository.skip_dirs.is_empty());
    }
}
