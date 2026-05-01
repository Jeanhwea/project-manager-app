use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub path_with_namespace: String,
    #[serde(rename = "ssh_url_to_repo")]
    pub ssh_url: Option<String>,
    #[serde(rename = "http_url_to_repo")]
    pub http_url: Option<String>,
    pub web_url: Option<String>,
    pub description: Option<String>,
    pub archived: Option<bool>,
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: u64,
    pub name: String,
    pub full_path: String,
    pub web_url: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub name: String,
    pub email: Option<String>,
    pub web_url: Option<String>,
}
