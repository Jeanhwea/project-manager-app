use super::{GitLabConfig, GitLabError, Result};
use crate::domain::gitlab::models::{Group, Project, User};
use serde::de::DeserializeOwned;
use std::time::Duration;

pub struct GitLabClient {
    config: GitLabConfig,
    client: ureq::Agent,
    base_url: String,
}

impl GitLabClient {
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

    pub fn with_url_and_token(base_url: &str, token: &str) -> Self {
        let config = GitLabConfig {
            server: Some(base_url.trim_end_matches('/').to_string()),
            token: Some(token.to_string()),
        };
        Self::new(config)
    }

    fn token(&self) -> Result<&str> {
        self.config
            .token
            .as_deref()
            .ok_or_else(|| GitLabError::AuthenticationError("No token configured".to_string()))
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}/api/v4/{}", self.base_url, path);
        let token = self.token()?;

        let response = self
            .client
            .get(&url)
            .set("PRIVATE-TOKEN", token)
            .set("User-Agent", "pma-gitlab")
            .call()
            .map_err(|e| GitLabError::NetworkError(Box::new(e)))?;

        let status = response.status();
        if status == 429 {
            return Err(GitLabError::RateLimited);
        }

        if status != 200 {
            let body = response.into_string().map_err(|e| {
                GitLabError::InvalidResponse(format!("Failed to read body: {}", e))
            })?;
            return Err(GitLabError::ApiError(format!(
                "API returned error ({}): {}",
                status, body
            )));
        }

        response
            .into_json()
            .map_err(|e| GitLabError::InvalidResponse(format!("Failed to parse JSON: {}", e)))
    }

    fn get_with_query<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T> {
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
            .map_err(|e| GitLabError::NetworkError(Box::new(e)))?;

        let status = response.status();
        if status == 429 {
            return Err(GitLabError::RateLimited);
        }

        if status != 200 {
            let body = response.into_string().map_err(|e| {
                GitLabError::InvalidResponse(format!("Failed to read body: {}", e))
            })?;
            return Err(GitLabError::ApiError(format!(
                "API returned error ({}): {}",
                status, body
            )));
        }

        response
            .into_json()
            .map_err(|e| GitLabError::InvalidResponse(format!("Failed to parse JSON: {}", e)))
    }

    fn get_paged<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Vec<T>> {
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

    pub fn get_group_projects(
        &self,
        group_id: u64,
        include_subgroups: bool,
        include_archived: bool,
    ) -> Result<Vec<Project>> {
        let path = format!("groups/{}/projects", group_id);
        let query = vec![
            (
                "include_subgroups",
                if include_subgroups { "true" } else { "false" },
            ),
            ("archived", if include_archived { "true" } else { "false" }),
            ("order_by", "path"),
            ("sort", "asc"),
        ];

        self.get_paged(&path, &query)
    }

    pub fn get_groups(&self) -> Result<Vec<Group>> {
        let path = "groups";
        let query = vec![("order_by", "path"), ("sort", "asc")];
        self.get_paged(path, &query)
    }

    pub fn get_current_user(&self) -> Result<User> {
        let path = "user";
        self.get(path)
    }
}
