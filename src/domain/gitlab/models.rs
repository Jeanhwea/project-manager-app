use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub path: String,
    pub path_with_namespace: String,
    #[serde(rename = "ssh_url_to_repo")]
    pub ssh_url: Option<String>,
    #[serde(rename = "http_url_to_repo")]
    pub http_url: Option<String>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Group {
    pub id: u64,
    pub name: String,
    pub full_path: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub username: String,
    pub name: String,
}
