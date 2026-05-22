use crate::error::{AppError, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::io::Read;

const GITHUB_PROXIES: &[&str] = &[
    "https://gh-proxy.org/",
    "https://ghfast.top/",
    "https://ghproxy.cc/",
    "https://gh-proxy.com/",
    "https://github.moeyy.xyz/",
    "https://mirror.ghproxy.com/",
    "https://ghproxy.net/",
];

pub fn download_asset(api_url: &str, browser_url: &str, asset_name: &str) -> Result<Vec<u8>> {
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

    Err(AppError::self_update(format!(
        "所有下载方式均失败，请手动下载: {}\n\
             提示: 可设置 PMA_DOWNLOAD_URL 环境变量指定下载地址，\n\
             或设置 GITHUB_TOKEN 环境变量提高 API 下载成功率",
        browser_url
    )))
}

fn validate_archive(data: &[u8], asset_name: &str) -> Result<()> {
    let valid = if asset_name.ends_with(".zip") {
        data.len() >= 4 && &data[..4] == b"PK\x03\x04"
    } else if asset_name.ends_with(".tar.gz") {
        data.len() >= 2 && &data[..2] == b"\x1f\x8b"
    } else {
        true
    };

    if !valid {
        return Err(AppError::self_update("下载的文件格式无效"));
    }
    Ok(())
}

fn try_download_api(api_url: &str) -> Result<Vec<u8>> {
    let mut req = ureq::get(api_url)
        .set("User-Agent", "pma-self-update")
        .set("Accept", "application/octet-stream");

    if let Ok(token) = env::var("GITHUB_TOKEN") {
        req = req.set("Authorization", &format!("Bearer {}", token));
    }

    let resp = req
        .call()
        .map_err(|e| AppError::self_update(format!("API 下载失败: {}", e)))?;
    read_response_with_progress(resp)
}

fn try_download(url: &str) -> Result<Vec<u8>> {
    let resp = ureq::get(url)
        .set("User-Agent", "pma-self-update")
        .call()
        .map_err(|e| AppError::self_update(format!("下载安装包失败: {}", e)))?;

    read_response_with_progress(resp)
}

fn read_response_with_progress(resp: ureq::Response) -> Result<Vec<u8>> {
    let total: u64 = resp
        .header("Content-Length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

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
            .map_err(|e| AppError::self_update(format!("读取下载内容失败: {}", e)))?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        pb.set_position(data.len() as u64);
    }
    pb.finish_and_clear();
    Ok(data)
}
