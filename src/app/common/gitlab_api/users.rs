use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GitLabUser {
    pub id: u64,
    pub username: String,
    pub name: String,
    pub state: String,
    pub email: Option<String>,
    pub is_admin: Option<bool>,
}
