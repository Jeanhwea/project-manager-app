use super::{GitLabError, Result};
use crate::domain::config::manager::ConfigManager;
use crate::domain::gitlab::models::{Group, Project, User};
use serde::de::DeserializeOwned;

pub struct GitLabClient {
    client: ureq::Agent,
    base_url: String,
    token: String,
}

impl GitLabClient {
    pub fn new() -> Result<Self> {
        let gitlab_config = ConfigManager::load_gitlab();
        let server = gitlab_config.servers.first().ok_or_else(|| {
            GitLabError::AuthenticationError("GitLab server not configured".to_string())
        })?;
        let base_url = normalize_server_url(&server.url);
        let token = server.token.clone();

        let client = ureq::agent();
        Ok(Self {
            client,
            base_url,
            token,
        })
    }

    pub fn with_url_and_token(url: &str, token: &str) -> Self {
        let base_url = normalize_server_url(url);
        let client = ureq::agent();
        Self {
            client,
            base_url,
            token: token.to_string(),
        }
    }

    fn send_request<T: DeserializeOwned>(&self, request: ureq::Request) -> Result<T> {
        let response = request
            .set("PRIVATE-TOKEN", &self.token)
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

    pub fn get_groups(&self) -> Result<Vec<Group>> {
        let url = format!("{}/api/v4/groups?per_page=100", self.base_url);
        let request = self.client.get(&url);
        self.send_request(request)
    }

    pub fn get_group_projects(
        &self,
        group_id: u64,
        include_subgroups: bool,
        archived: bool,
    ) -> Result<Vec<Project>> {
        let include = if include_subgroups { "true" } else { "false" };
        let archived_flag = if archived { "true" } else { "false" };
        let url = format!(
            "{}/api/v4/groups/{}/projects?include_subgroups={}&per_page=100&archived={}",
            self.base_url, group_id, include, archived_flag
        );
        let request = self.client.get(&url);
        self.send_request(request)
    }
}

fn normalize_server_url(server: &str) -> String {
    if server.starts_with("http") {
        server.trim_end_matches('/').to_string()
    } else {
        format!(
            "https://{}",
            server
                .trim_start_matches("https://")
                .trim_start_matches("http://")
        )
    }
}

impl Default for GitLabClient {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            client: ureq::agent(),
            base_url: String::new(),
            token: String::new(),
        })
    }
}
