use crate::app::common::gitlab_api::client::GitLabClient;
use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct GitLabProject {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub path_with_namespace: String,
    pub ssh_url_to_repo: Option<String>,
    pub http_url_to_repo: Option<String>,
    pub web_url: Option<String>,
    pub description: Option<String>,
    pub archived: Option<bool>,
    pub visibility: Option<String>,
}

pub struct ProjectsQuery<'a> {
    client: &'a GitLabClient,
    group_id: Option<u64>,
    include_subgroups: bool,
    include_archived: bool,
}

impl<'a> ProjectsQuery<'a> {
    pub fn new(client: &'a GitLabClient) -> Self {
        Self {
            client,
            group_id: None,
            include_subgroups: true,
            include_archived: false,
        }
    }

    pub fn group(mut self, group_id: u64) -> Self {
        self.group_id = Some(group_id);
        self
    }

    pub fn include_subgroups(mut self, include: bool) -> Self {
        self.include_subgroups = include;
        self
    }

    pub fn include_archived(mut self, include: bool) -> Self {
        self.include_archived = include;
        self
    }

    pub fn list(&self) -> Result<Vec<GitLabProject>> {
        let group_id = self.group_id.context("必须指定 group_id")?;
        let path = format!("groups/{}/projects", group_id);

        let mut query: Vec<(&str, &str)> = Vec::new();
        query.push((
            "include_subgroups",
            if self.include_subgroups {
                "true"
            } else {
                "false"
            },
        ));
        query.push((
            "archived",
            if self.include_archived {
                "true"
            } else {
                "false"
            },
        ));
        query.push(("order_by", "path"));
        query.push(("sort", "asc"));

        self.client.get_paged(&path, &query)
    }
}
