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

    /// 通过组路径获取组信息
    /// 先尝试搜索，如果找不到再尝试直接获取
    pub fn get_by_path(&self, group_path: &str) -> Result<GitLabGroup> {
        eprintln!("[DEBUG] get_by_path - group_path: {}", group_path);

        // 方法1: 通过搜索 API 查找
        // 使用路径的第一部分作为搜索词（例如 "ntfw/fe" -> "ntfw"）
        let search_term = group_path.split('/').next().unwrap_or(group_path);
        eprintln!("[DEBUG] get_by_path - search_term: {}", search_term);

        match self.search(search_term) {
            Ok(groups) => {
                eprintln!(
                    "[DEBUG] get_by_path - found {} groups via search",
                    groups.len()
                );
                if let Some(group) = groups.iter().find(|g| g.full_path == group_path) {
                    eprintln!(
                        "[DEBUG] get_by_path - found matching group: {} (id={})",
                        group.full_path, group.id
                    );
                    return Ok(group.clone());
                }
            }
            Err(e) => {
                eprintln!("[DEBUG] get_by_path - search error: {}", e);
            }
        }

        // 方法2: 直接通过 ID 或编码路径获取
        // 尝试使用组的 ID（如果 group_path 是数字）
        if let Ok(id) = group_path.parse::<u64>() {
            let path = format!("groups/{}", id);
            eprintln!("[DEBUG] get_by_path - trying by id: {}", path);
            if let Ok(group) = self.client.get::<GitLabGroup>(&path) {
                return Ok(group);
            }
        }

        // 方法3: 尝试直接使用路径（不编码）
        let path = format!("groups/{}", group_path);
        eprintln!("[DEBUG] get_by_path - trying without encoding: {}", path);
        if let Ok(group) = self.client.get::<GitLabGroup>(&path) {
            return Ok(group);
        }

        // 方法4: 尝试 URL 编码路径
        let encoded_path = url_encode_path(group_path);
        let path = format!("groups/{}", encoded_path);
        eprintln!("[DEBUG] get_by_path - trying with encoding: {}", path);
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
