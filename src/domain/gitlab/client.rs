//! GitLab API client module
//!
//! This module implements the GitLab API client.

use super::{GitLabConfig, GitLabError, Result};

/// GitLab API client
pub struct GitLabClient {
    config: GitLabConfig,
}

impl GitLabClient {
    /// Create a new GitLab client
    pub fn new(config: GitLabConfig) -> Self {
        Self { config }
    }
    
    /// Make API request
    pub fn request(&self, endpoint: &str) -> Result<String> {
        // Implementation will be added in Task 5.1
        todo!("API client not yet implemented")
    }
}