use anyhow::{Context, Result};
use serde::de::DeserializeOwned;

pub struct GitLabClient {
    base_url: String,
    token: String,
}

impl GitLabClient {
    pub fn new(base_url: &str, token: &str) -> Self {
        let base_url = base_url.trim_end_matches('/');
        Self {
            base_url: base_url.to_string(),
            token: token.to_string(),
        }
    }

    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}/api/v4/{}", self.base_url, path);

        let resp = ureq::get(&url)
            .set("PRIVATE-TOKEN", &self.token)
            .set("User-Agent", "pma-gitlab")
            .call()
            .with_context(|| format!("请求失败: {}", url))?;

        let status = resp.status();
        if status != 200 {
            let body: String = resp.into_string().unwrap_or_default();
            anyhow::bail!("API 返回错误 ({}): {}", status, body);
        }

        resp.into_json()
            .with_context(|| format!("解析响应失败: {}", url))
    }

    pub fn get_with_query<T: DeserializeOwned>(
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

        let resp = ureq::get(&url)
            .set("PRIVATE-TOKEN", &self.token)
            .set("User-Agent", "pma-gitlab")
            .call()
            .with_context(|| format!("请求失败: {}", url))?;

        let status = resp.status();
        if status != 200 {
            let body: String = resp.into_string().unwrap_or_default();
            anyhow::bail!("API 返回错误 ({}): {}", status, body);
        }

        resp.into_json()
            .with_context(|| format!("解析响应失败: {}", url))
    }

    pub fn get_paged<T: DeserializeOwned>(
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

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}
