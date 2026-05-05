use std::sync::RwLock;

use super::manager::ConfigDir;
use super::schema::{AppConfig, GitLabConfig};

#[allow(dead_code)]
pub struct ConfigCache {
    config: RwLock<Option<AppConfig>>,
    gitlab_config: RwLock<Option<GitLabConfig>>,
}

#[allow(dead_code)]
impl ConfigCache {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(None),
            gitlab_config: RwLock::new(None),
        }
    }

    pub fn get(&self) -> AppConfig {
        {
            let guard = self.config.read().unwrap();
            if let Some(ref config) = *guard {
                return config.clone();
            }
        }

        let config = ConfigDir::load_config();
        let mut guard = self.config.write().unwrap();
        *guard = Some(config.clone());
        config
    }

    pub fn refresh(&self) {
        {
            let mut guard = self.config.write().unwrap();
            *guard = None;
        }
        {
            let mut guard = self.gitlab_config.write().unwrap();
            *guard = None;
        }
    }

    pub fn get_gitlab(&self) -> GitLabConfig {
        {
            let guard = self.gitlab_config.read().unwrap();
            if let Some(ref config) = *guard {
                return config.clone();
            }
        }

        let config = ConfigDir::load_gitlab();
        let mut guard = self.gitlab_config.write().unwrap();
        *guard = Some(config.clone());
        config
    }
}

impl Default for ConfigCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_returns_same_values() {
        let cache = ConfigCache::new();
        let config1 = cache.get();
        let config2 = cache.get();

        assert_eq!(config1.repository.max_depth, config2.repository.max_depth);
        assert_eq!(
            config1.repository.skip_dirs.len(),
            config2.repository.skip_dirs.len()
        );
    }

    #[test]
    fn test_gitlab_cache_returns_same_values() {
        let cache = ConfigCache::new();
        let config1 = cache.get_gitlab();
        let config2 = cache.get_gitlab();

        assert_eq!(config1.servers.len(), config2.servers.len());
    }

    #[test]
    fn test_default_creates_new_instance() {
        let cache = ConfigCache::default();
        let config = cache.get();

        assert_eq!(config.repository.max_depth, 3);
        assert!(!config.repository.skip_dirs.is_empty());
    }
}
