use super::{GitLabError, Result};
use crate::domain::config::manager::ConfigDir;
use crate::domain::gitlab::models::{Group, Project, User};
use serde::de::DeserializeOwned;

pub struct GitLabClient {
    client: ureq::Agent,
    base_url: String,
}

impl GitLabClient {
    pub fn new() -> Result<Self> {
        let gitlab_config = ConfigDir::load_gitlab();
        let server = gitlab_config.server.ok_or_else(|| {
            GitLabError::AuthenticationError("GitLab server not configured".to_string())
        })?;
        let base_url = if server.starts_with("http") {
            server.trim_end_matches('/').to_string()
        } else {
            format!("https://{}", server.trim_start_matches("https://").trim_start_matches("http://"))
        };

        let client = ureq::agent();
        Ok(Self { client, base_url })
    }

    pub fn with_config(server: String, _token: String) -> Self {
        let base_url = if server.starts_with("http") {
            server.trim_end_matches('/').to_string()
        } else {
            format!("https://{}", server.trim_start_matches("https://").trim_start_matches("http://"))
        };

        let client = ureq::agent();
        Self { client, base_url }
    }

    fn token(&self) -> Result<&str> {
        let gitlab_config = ConfigDir::load_gitlab();
        gitlab_config.token.as_deref().ok_or_else(|| {
            GitLabError::AuthenticationError("GitLab token not configured".to_string())
        })
    }

    fn send_request<T: DeserializeOwned>(&self, request: ureq::Request) -> Result<T> {
        let token = self.token()?;
        let response = request
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

    pub fn get_current_user(&self) -> Result<User> {
        let url = format!("{}/api/v4/user", self.base_url);
        let request = self.client.get(&url);
        self.send_request(request)
    }

    pub fn list_projects(&self, group_id: &str) -> Result<Vec<Project>> {
        let url = format!(
            "{}/api/v4/groups/{}/projects?include_subgroups=true&per_page=100",
            self.base_url, group_id
        );
        let request = self.client.get(&url);
        self.send_request(request)
    }

    pub fn list_projects_with_query(
        &self,
        group_id: &str,
        query: &[(&str, &str)],
    ) -> Result<Vec<Project>> {
        let query_str: String = query
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        let url = format!(
            "{}/api/v4/groups/{}/projects?include_subgroups=true&per_page=100&{}",
            self.base_url, group_id, query_str
        );
        let request = self.client.get(&url);
        self.send_request(request)
    }

    pub fn list_groups(&self) -> Result<Vec<Group>> {
        let url = format!("{}/api/v4/groups?per_page=100", self.base_url);
        let request = self.client.get(&url);
        self.send_request(request)
    }

    pub fn list_groups_with_query(&self, query: &[(&str, &str)]) -> Result<Vec<Group>> {
        let query_str: String = query
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        let url = format!(
            "{}/api/v4/groups?per_page=100&{}",
            self.base_url, query_str
        );
        let request = self.client.get(&url);
        self.send_request(request)
    }
}

impl Default for GitLabClient {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            client: ureq::agent(),
            base_url: String::new(),
        })
    }
}
