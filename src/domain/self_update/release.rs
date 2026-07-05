use crate::domain::self_update::SelfUpdateError;
use crate::error::Result;
use serde::Deserialize;
use std::env;

const GITHUB_API_URL: &str =
    "https://api.github.com/repos/Jeanhwea/project-manager-app/releases/latest";

#[derive(Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub url: String,
}

pub fn fetch_latest_release() -> Result<Release> {
    let mut req = ureq::get(GITHUB_API_URL)
        .set("User-Agent", "pma-self-update")
        .set("Accept", "application/vnd.github.v3+json");

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
