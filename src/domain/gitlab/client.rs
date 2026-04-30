//! GitLab API client module
//!
//! This module implements the GitLab API client.

use super::{GitLabConfig, GitLabError, Result};
use crate::domain::gitlab::models::{Group, Project, User};
use serde::de::DeserializeOwned;
use std::time::Duration;

/// GitLab API client
pub struct GitLabClient {
    config: GitLabConfig,
    client: ureq::Agent,
    base_url: String,
}

impl GitLabClient {
    /// Create a new GitLab client with configuration
    pub fn new(config: GitLabConfig) -> Self {
        let base_url = config
            .server
            .as_ref()
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| "https://gitlab.com".to_string());

        let client = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(30))
            .build();

        Self {
            config,
            client,
            base_url,
        }
    }

    /// Create a new GitLab client with custom base URL and token
    pub fn with_url_and_token(base_url: &str, token: &str) -> Self {
        let config = GitLabConfig {
            server: Some(base_url.trim_end_matches('/').to_string()),
            token: Some(token.to_string()),
            default_protocol: super::CloneProtocol::Https,
        };
        Self::new(config)
    }

    /// Get the authentication token
    fn token(&self) -> Result<&str> {
        self.config
            .token
            .as_deref()
            .ok_or_else(|| GitLabError::AuthenticationError("No token configured".to_string()))
    }

    /// Make a GET request to the GitLab API
    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}/api/v4/{}", self.base_url, path);
        let token = self.token()?;

        let response = self
            .client
            .get(&url)
            .set("PRIVATE-TOKEN", token)
            .set("User-Agent", "pma-gitlab")
            .call()
            .map_err(GitLabError::NetworkError)?;

        let status = response.status();
        if status != 200 {
            let body = response
                .into_string()
                .map_err(|e| GitLabError::InvalidResponse(format!("Failed to read body: {}", e)))?;
            return Err(GitLabError::ApiError(format!(
                "API returned error ({}): {}",
                status, body
            )));
        }

        response
            .into_json()
            .map_err(|e| GitLabError::InvalidResponse(format!("Failed to parse JSON: {}", e)))
    }

    /// Make a GET request with query parameters
    fn get_with_query<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T> {
        let query_str: String = query
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!("{}/api/v4/{}?{}", self.base_url, path, query_str);
        let token = self.token()?;

        let response = self
            .client
            .get(&url)
            .set("PRIVATE-TOKEN", token)
            .set("User-Agent", "pma-gitlab")
            .call()
            .map_err(GitLabError::NetworkError)?;

        let status = response.status();
        if status != 200 {
            let body = response
                .into_string()
                .map_err(|e| GitLabError::InvalidResponse(format!("Failed to read body: {}", e)))?;
            return Err(GitLabError::ApiError(format!(
                "API returned error ({}): {}",
                status, body
            )));
        }

        response
            .into_json()
            .map_err(|e| GitLabError::InvalidResponse(format!("Failed to parse JSON: {}", e)))
    }

    /// Get paginated results
    fn get_paged<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<Vec<T>> {
        let mut all_items = Vec::new();
        let mut page = 1u32;
        let per_page = 100u32;

        loop {
            let page_str = page.to_string();
            let per_page_str = per_page.to_string();

            let mut query_with_page: Vec<(&str, &str)> = query.to_vec();
            query_with_page.push(("page", &page_str));
            query_with_page.push(("per_page", &per_page_str));

            let items: Vec<T> = self.get_with_query(path, &query_with_page)?;
            let count = items.len();
            all_items.extend(items);

            if count < per_page as usize {
                break;
            }
            page += 1;
        }

        Ok(all_items)
    }

    // Project API methods

    /// Get projects for a specific group
    pub fn get_group_projects(
        &self,
        group_id: u64,
        include_subgroups: bool,
        include_archived: bool,
    ) -> Result<Vec<Project>> {
        let path = format!("groups/{}/projects", group_id);
        let mut query = vec![
            (
                "include_subgroups",
                if include_subgroups { "true" } else { "false" },
            ),
            (
                "archived",
                if include_archived { "true" } else { "false" },
            ),
            ("order_by", "path"),
            ("sort", "asc"),
        ];

        self.get_paged(&path, &query)
    }

    /// Get all projects accessible to the authenticated user
    pub fn get_projects(&self, owned: bool) -> Result<Vec<Project>> {
        let path = "projects";
        let query = if owned {
            vec![("owned", "true"), ("order_by", "path"), ("sort", "asc")]
        } else {
            vec![("order_by", "path"), ("sort", "asc")]
        };

        self.get_paged(path, &query)
    }

    /// Get a specific project by ID
    pub fn get_project(&self, project_id: u64) -> Result<Project> {
        let path = format!("projects/{}", project_id);
        self.get(&path)
    }

    // Group API methods

    /// Get all groups accessible to the authenticated user
    pub fn get_groups(&self) -> Result<Vec<Group>> {
        let path = "groups";
        let query = vec![("order_by", "path"), ("sort", "asc")];
        self.get_paged(path, &query)
    }

    /// Get a specific group by ID
    pub fn get_group(&self, group_id: u64) -> Result<Group> {
        let path = format!("groups/{}", group_id);
        self.get(&path)
    }

    // User API methods

    /// Get the current authenticated user
    pub fn get_current_user(&self) -> Result<User> {
        let path = "user";
        self.get(path)
    }

    /// Test API connectivity
    pub fn test_connection(&self) -> Result<()> {
        // Try to get current user as a connectivity test
        self.get_current_user()?;
        Ok(())
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Check if authentication is configured
    pub fn is_authenticated(&self) -> bool {
        self.config.token.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::gitlab::CloneProtocol;

    #[test]
    fn test_client_creation() {
        let config = GitLabConfig {
            server: Some("https://gitlab.com".to_string()),
            token: Some("test-token".to_string()),
            default_protocol: CloneProtocol::Https,
        };
        
        let client = GitLabClient::new(config);
        assert_eq!(client.base_url(), "https://gitlab.com");
        assert!(client.is_authenticated());
    }

    #[test]
    fn test_client_with_url_and_token() {
        let client = GitLabClient::with_url_and_token("https://gitlab.example.com", "test-token");
        assert_eq!(client.base_url(), "https://gitlab.example.com");
        assert!(client.is_authenticated());
    }

    #[test]
    fn test_client_without_token() {
        let config = GitLabConfig {
            server: Some("https://gitlab.com".to_string()),
            token: None,
            default_protocol: CloneProtocol::Https,
        };
        
        let client = GitLabClient::new(config);
        assert!(!client.is_authenticated());
        
        // Should fail when trying to make a request without token
        let result = client.token();
        assert!(result.is_err());
    }

    #[test]
    fn test_default_server() {
        let config = GitLabConfig {
            server: None,
            token: Some("test-token".to_string()),
            default_protocol: CloneProtocol::Https,
        };
        
        let client = GitLabClient::new(config);
        assert_eq!(client.base_url(), "https://gitlab.com");
    }

    #[test]
    fn test_server_url_normalization() {
        let config = GitLabConfig {
            server: Some("https://gitlab.com/".to_string()), // Trailing slash
            token: Some("test-token".to_string()),
            default_protocol: CloneProtocol::Https,
        };
        
        let client = GitLabClient::new(config);
        assert_eq!(client.base_url(), "https://gitlab.com");
    }
}