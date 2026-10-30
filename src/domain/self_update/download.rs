//! 下载 release 安装包，按「自定义地址 → GitHub API → 网页直连 → 公共镜像代理」的顺序逐个尝试，
//! 任一来源拿到合法的 archive 就返回；全部失败才报错。

use crate::domain::self_update::SelfUpdateError;
use crate::error::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::io::Read;

/// GitHub 下载加速镜像，仅在 browser 直连失败且 URL 属于 github.com 时兜底使用
const GITHUB_PROXIES: &[&str] = &[
    "https://gh-proxy.org/",
    "https://ghfast.top/",
    "https://ghproxy.cc/",
    "https://gh-proxy.com/",
    "https://github.moeyy.xyz/",
    "https://mirror.ghproxy.com/",
    "https://ghproxy.net/",
];

/// 依次尝试各下载来源，返回安装包字节
pub fn download_asset(api_url: &str, browser_url: &str, asset_name: &str) -> Result<Vec<u8>> {
    // 内网或 GitHub 不通时，用 PMA_DOWNLOAD_URL 直接指定镜像地址
    if let Ok(custom_url) = env::var("PMA_DOWNLOAD_URL")
        && let Ok(data) = try_download(&custom_url)
        && validate_archive(&data, asset_name).is_ok()
    {
        return Ok(data);
    }

    if let Ok(data) = try_download_api(api_url)
        && validate_archive(&data, asset_name).is_ok()
    {
        return Ok(data);
    }

    if let Ok(data) = try_download(browser_url)
        && validate_archive(&data, asset_name).is_ok()
    {
        return Ok(data);
    }

    // 代理的用法是把完整 URL（含 https://）整体追加在代理前缀之后；只对 github.com 的链接有意义
    if browser_url.starts_with("https://github.com/") {
        for proxy in GITHUB_PROXIES {
            let proxy_url = format!("{}{}", proxy, browser_url);
            if let Ok(data) = try_download(&proxy_url)
                && validate_archive(&data, asset_name).is_ok()
            {
                return Ok(data);
            }
        }
    }

    Err(SelfUpdateError::AllDownloadAttemptsFailed {
        browser_url: browser_url.to_string(),
    }
    .into())
}

/// 按文件头魔数确认下载到的确实是安装包；未知格式直接放行
// 镜像与代理经常返回 200 + HTML 错误页，只看 HTTP 状态码会把网页当成安装包写进磁盘
fn validate_archive(data: &[u8], asset_name: &str) -> Result<()> {
    let valid = if asset_name.ends_with(".zip") {
        data.len() >= 4 && &data[..4] == b"PK\x03\x04"
    } else if asset_name.ends_with(".tar.gz") {
        data.len() >= 2 && &data[..2] == b"\x1f\x8b"
    } else {
        true
    };

    if !valid {
        return Err(SelfUpdateError::InvalidArchiveMagic.into());
    }
    Ok(())
}

fn try_download_api(api_url: &str) -> Result<Vec<u8>> {
    // API 地址默认返回 JSON 元数据，必须显式要 octet-stream 才会回传二进制
    let mut req = ureq::get(api_url)
        .set("User-Agent", "pma-self-update")
        .set("Accept", "application/octet-stream");

    // 带 token 可避开匿名请求的严格限流
    if let Ok(token) = env::var("GITHUB_TOKEN") {
        req = req.set("Authorization", &format!("Bearer {}", token));
    }

    let resp = req.call().map_err(|e| SelfUpdateError::ApiDownload {
        source: Box::new(e),
    })?;
    read_response_with_progress(resp)
}

fn try_download(url: &str) -> Result<Vec<u8>> {
    let resp = ureq::get(url)
        .set("User-Agent", "pma-self-update")
        .call()
        .map_err(|e| SelfUpdateError::AssetDownload {
            source: Box::new(e),
        })?;

    read_response_with_progress(resp)
}

fn read_response_with_progress(resp: ureq::Response) -> Result<Vec<u8>> {
    let total: u64 = resp
        .header("Content-Length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    // Content-Length 缺失（分块传输 / 经过代理）时只能退化为不带总量的 spinner
    let pb = if total > 0 {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("=>-"),
        );
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {bytes}")
                .unwrap(),
        );
        pb
    };

    let mut reader = resp.into_reader();
    let mut data = Vec::with_capacity(total as usize);
    let mut buf = [0u8; 8192];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| SelfUpdateError::ReadDownload { source: e })?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        pb.set_position(data.len() as u64);
    }
    pb.finish_and_clear();
    Ok(data)
}
