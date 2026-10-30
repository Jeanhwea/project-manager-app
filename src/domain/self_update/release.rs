//! 查询 GitHub 上最新的 release 元信息（tag 与资产列表），不下载资产本身。

use crate::domain::self_update::SelfUpdateError;
use crate::error::Result;
use serde::Deserialize;
use std::env;

const GITHUB_API_URL: &str =
    "https://api.github.com/repos/Jeanhwea/project-manager-app/releases/latest";

/// 一个 release 的元信息，只取 pma 关心的两个字段
#[derive(Debug, Deserialize)]
pub struct Release {
    /// 形如 `v1.2.3`，比较版本前需去掉 `v`
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

/// release 里的一个可下载资产
#[derive(Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    /// 网页直连地址，无需鉴权
    pub browser_download_url: String,
    /// API 地址，需 `Accept: application/octet-stream` 才会返回二进制
    pub url: String,
}

/// 拉取最新 release；设置 `GITHUB_TOKEN` 时带上它以避免匿名限流
pub fn fetch_latest_release() -> Result<Release> {
    let mut req = ureq::get(GITHUB_API_URL)
        .set("User-Agent", "pma-self-update")
        .set("Accept", "application/vnd.github.v3+json");

    // token 是可选的：没有也能匿名访问公开仓库，只是限流更严
    if let Ok(token) = env::var("GITHUB_TOKEN") {
        req = req.set("Authorization", &format!("Bearer {}", token));
    }

    let resp = req
        .call()
        .map_err(|e| SelfUpdateError::FetchReleaseRequest {
            source: Box::new(e),
        })?;
    let release: Release = resp
        .into_json()
        .map_err(|e| SelfUpdateError::ParseReleaseJson { source: e })?;
    Ok(release)
}
