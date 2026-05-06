use std::sync::RwLock;

use super::manager::ConfigDir;
use super::schema::{AppConfig, GitLabConfig};

#[allow(dead_code)]
pub struct ConfigCache {
    config: RwLock<Option<AppConfig>>,
    gitlab_config: RwLock<Option<GitLabConfig>>,
}

// Test-only counter to verify lazy loading behavior
#[cfg(test)]
static CONFIG_LOAD_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

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

        #[cfg(test)]
        CONFIG_LOAD_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

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
    use std::sync::atomic::Ordering;

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

    #[test]
    fn test_lazy_loading_only_loads_once() {
        // Reset the counter before this test
        CONFIG_LOAD_COUNT.store(0, Ordering::SeqCst);

        let cache = ConfigCache::new();

        // First call should trigger load
        let _config1 = cache.get();
        let loads_after_first = CONFIG_LOAD_COUNT.load(Ordering::SeqCst);
        assert_eq!(
            loads_after_first, 1,
            "Config should be loaded once on first access"
        );

        // Second call should use cache, not load again
        let _config2 = cache.get();
        let loads_after_second = CONFIG_LOAD_COUNT.load(Ordering::SeqCst);
        assert_eq!(
            loads_after_second, 1,
            "Config should not be loaded again on second access"
        );

        // Third call should still use cache
        let _config3 = cache.get();
        let loads_after_third = CONFIG_LOAD_COUNT.load(Ordering::SeqCst);
        assert_eq!(
            loads_after_third, 1,
            "Config should still use cache on subsequent accesses"
        );
    }

    #[test]
    fn test_refresh_clears_cache_and_reloads() {
        // Reset the counter before this test
        CONFIG_LOAD_COUNT.store(0, Ordering::SeqCst);

        let cache = ConfigCache::new();

        // First load
        let _config1 = cache.get();
        let loads_after_first = CONFIG_LOAD_COUNT.load(Ordering::SeqCst);
        assert_eq!(loads_after_first, 1);

        // Refresh should clear cache
        cache.refresh();

        // Next access should reload
        let _config2 = cache.get();
        let loads_after_refresh = CONFIG_LOAD_COUNT.load(Ordering::SeqCst);
        assert_eq!(
            loads_after_refresh, 2,
            "Config should be reloaded after refresh"
        );
    }
}
