use crate::app::common::gitlab_api::client::GitLabClient;
use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct GitLabGroup {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub full_path: String,
    pub description: Option<String>,
    pub visibility: Option<String>,
    pub web_url: Option<String>,
}

pub struct GroupQuery<'a> {
    client: &'a GitLabClient,
}

impl<'a> GroupQuery<'a> {
    pub fn new(client: &'a GitLabClient) -> Self {
        Self { client }
    }

    pub fn search(&self, search: &str) -> Result<Vec<GitLabGroup>> {
        let query = &[("search", search)];
        self.client.get_paged("groups", query)
    }

    pub fn get_by_path(&self, group_path: &str) -> Result<GitLabGroup> {
        let search_term = group_path.split('/').next().unwrap_or(group_path);

        if let Ok(groups) = self.search(search_term)
            && let Some(group) = groups.iter().find(|g| g.full_path == group_path)
        {
            return Ok(group.clone());
        }

        if let Ok(id) = group_path.parse::<u64>() {
            let path = format!("groups/{}", id);
            if let Ok(group) = self.client.get::<GitLabGroup>(&path) {
                return Ok(group);
            }
        }

        let path = format!("groups/{}", group_path);
        if let Ok(group) = self.client.get::<GitLabGroup>(&path) {
            return Ok(group);
        }

        let encoded_path = url_encode_path(group_path);
        let path = format!("groups/{}", encoded_path);
        self.client
            .get::<GitLabGroup>(&path)
            .with_context(|| format!("无法找到组: {}", group_path))
    }
}

fn url_encode_path(s: &str) -> String {
    s.replace('%', "%25")
        .replace('/', "%2F")
        .replace(' ', "%20")
        .replace('#', "%23")
}
