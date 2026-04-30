//! GitLab data models module
//!
//! This module defines GitLab API data structures.

use serde::{Deserialize, Serialize};

/// GitLab project representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,
    pub name: String,
    pub path_with_namespace: String,
    pub ssh_url: String,
    pub http_url: String,
    pub archived: bool,
}

/// GitLab group representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: u64,
    pub name: String,
    pub full_path: String,
}

/// GitLab user representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub name: String,
}