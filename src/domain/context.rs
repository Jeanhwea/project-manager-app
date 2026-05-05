use std::sync::OnceLock;

use super::config::ConfigCache;
use super::config::schema::AppConfig;
use super::git::command::GitCommandRunner;

pub struct AppContext {
    git_runner: OnceLock<GitCommandRunner>,
    config_cache: OnceLock<ConfigCache>,
}

impl AppContext {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<AppContext> = OnceLock::new();
        INSTANCE.get_or_init(Self::new)
    }

    pub fn git_runner(&self) -> &GitCommandRunner {
        self.git_runner.get_or_init(GitCommandRunner::new)
    }

    pub fn config(&self) -> AppConfig {
        self.config_cache.get_or_init(ConfigCache::new).get()
    }

    pub fn refresh_config(&self) {
        if let Some(cache) = self.config_cache.get() {
            cache.refresh();
        }
    }

    fn new() -> Self {
        Self {
            git_runner: OnceLock::new(),
            config_cache: OnceLock::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_returns_same_instance() {
        let ctx1 = AppContext::global();
        let ctx2 = AppContext::global();
        assert!(std::ptr::eq(ctx1, ctx2));
    }

    #[test]
    fn test_git_runner_returns_same_instance() {
        let ctx = AppContext::global();
        let runner1 = ctx.git_runner();
        let runner2 = ctx.git_runner();
        assert!(std::ptr::eq(runner1, runner2));
    }

    #[test]
    fn test_config_returns_valid_config() {
        let ctx = AppContext::global();
        let config = ctx.config();
        assert_eq!(config.repository.max_depth, 3);
        assert!(!config.repository.skip_dirs.is_empty());
    }
}
