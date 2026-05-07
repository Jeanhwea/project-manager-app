use std::sync::{Arc, OnceLock};

use super::config::ConfigCache;
use super::config::schema::AppConfig;
use super::git::command::GitCommandRunner;
use super::runner::{CommandRunner, DefaultCommandRunner};

pub struct AppContext {
    git_runner: OnceLock<GitCommandRunner>,
    command_runner: OnceLock<Arc<dyn CommandRunner>>,
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

    pub fn command_runner(&self) -> Arc<dyn CommandRunner> {
        self.command_runner
            .get_or_init(|| Arc::new(DefaultCommandRunner))
            .clone()
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
            command_runner: OnceLock::new(),
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

    #[test]
    fn test_multiple_calls_return_same_git_runner() {
        let ctx = AppContext::global();

        let runners: Vec<&GitCommandRunner> = (0..10).map(|_| ctx.git_runner()).collect();

        let first = runners[0];
        for runner in &runners[1..] {
            assert!(
                std::ptr::eq(first, *runner),
                "GitCommandRunner instances should be identical"
            );
        }
    }

    #[test]
    fn test_singleton_persists_across_multiple_global_calls() {
        let ctx1 = AppContext::global();
        let runner1 = ctx1.git_runner();

        let ctx2 = AppContext::global();
        let runner2 = ctx2.git_runner();

        assert!(std::ptr::eq(ctx1, ctx2));
        assert!(std::ptr::eq(runner1, runner2));
    }

    #[test]
    fn test_command_runner_returns_valid_instance() {
        let ctx = AppContext::global();
        let runner = ctx.command_runner();
        assert!(Arc::strong_count(&runner) >= 1);
    }

    #[test]
    fn test_command_runner_returns_same_instance() {
        let ctx = AppContext::global();
        let runner1 = ctx.command_runner();
        let runner2 = ctx.command_runner();
        assert!(Arc::ptr_eq(&runner1, &runner2));
    }

    #[test]
    fn test_multiple_calls_return_same_command_runner() {
        let ctx = AppContext::global();

        let runners: Vec<Arc<dyn CommandRunner>> =
            (0..10).map(|_| ctx.command_runner()).collect();

        let first = &runners[0];
        for runner in &runners[1..] {
            assert!(
                Arc::ptr_eq(first, runner),
                "CommandRunner instances should be identical"
            );
        }
    }
}
