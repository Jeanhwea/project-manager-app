//! Configuration manager module
//!
//! This module implements configuration loading and saving.

use super::{AppConfig, ConfigError, ConfigManager, ConfigSource, Result};

/// Configuration manager implementation
pub struct FileConfigManager;

impl ConfigManager for FileConfigManager {
    fn load() -> Result<AppConfig> {
        // Implementation will be added in Task 7.1
        todo!("Configuration loading not yet implemented")
    }
    
    fn save(&self, _config: &AppConfig) -> Result<()> {
        // Implementation will be added in Task 7.1
        todo!("Configuration saving not yet implemented")
    }
    
    fn get_source(&self) -> ConfigSource {
        ConfigSource::File
    }
}