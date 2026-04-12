use anyhow::{Context, Result};
use colored::*;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

const GITHUB_API_URL: &str =
    "https://api.github.com/repos/Jeanhwea/project-manager-app/releases/latest";
const GITHUB_PROXIES: &[&str] = &[
    "https://gh-proxy.org/",
    "https://ghfast.top/",
    "https://ghproxy.cc/",
    "https://gh-proxy.com/",
    "https://github.moeyy.xyz/",
    "https://mirror.ghproxy.com/",
    "https://ghproxy.net/",
];
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    url: String,
}

pub fn show_version() {
    println!(
        "{} {} ({}-{})",
        PKG_NAME,
        format!("v{}", PKG_VERSION).green(),
        env::consts::OS,
        env::consts::ARCH,
    );
}

pub fn execute(force: bool) -> Result<()> {
    println!("{}", "检查最新版本...".cyan());

    let release = fetch_latest_release()?;
    let latest = release.tag_name.trim_start_matches('v');
    let current = PKG_VERSION;

    println!("当前版本: {}", format!("v{}", current).yellow());
    println!("最新版本: {}", format!("v{}", latest).green());

    let latest_ver = semver::Version::parse(latest)
        .with_context(|| format!("无法解析最新版本号: {}", latest))?;
    let current_ver = semver::Version::parse(current)
        .with_context(|| format!("无法解析当前版本号: {}", current))?;

    if !force && current_ver >= latest_ver {
        println!("{}", "已经是最新版本，无需更新。".green());
        return Ok(());
    }

    if force && current_ver >= latest_ver {
        println!("{}", "强制更新模式，继续更新...".yellow());
    }

    let asset_name = get_asset_name(&release.tag_name)?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .with_context(|| format!("未找到适合当前平台的安装包: {}", asset_name))?;

    println!("下载 {}...", asset.name.cyan());
    let data = download_asset(&asset.url, &asset.browser_download_url, &asset.name)?;
    println!("下载完成，大小: {} bytes", data.len());

    let current_exe = env::current_exe().context("无法获取当前可执行文件路径")?;
    install_binary(&data, &asset.name, &current_exe)?;

    println!(
        "{}",
        format!("更新成功! v{} -> v{}", current, latest).green()
    );
    Ok(())
}

fn fetch_latest_release() -> Result<Release> {
    let mut req = ureq::get(GITHUB_API_URL)
        .set("User-Agent", "pma-selfupdate")
        .set("Accept", "application/vnd.github.v3+json");

    if let Ok(token) = env::var("GITHUB_TOKEN") {
        req = req.set("Authorization", &format!("Bearer {}", token));
    }

    let resp = req.call().context("请求 GitHub API 失败")?;
    let release: Release = resp.into_json().context("解析 release 信息失败")?;
    Ok(release)
}

fn download_asset(api_url: &str, browser_url: &str, asset_name: &str) -> Result<Vec<u8>> {
    // 优先通过 GitHub API 下载（Accept: application/octet-stream）
    println!("{}", "尝试通过 GitHub API 下载...".cyan());
    match try_download_api(api_url) {
        Ok(data) => {
            if validate_archive(&data, asset_name).is_ok() {
                return Ok(data);
            }
            eprintln!("{}", "API 下载的文件格式无效".yellow());
        }
        Err(e) => {
            eprintln!("{}", format!("API 下载失败: {}", e).yellow());
        }
    }

    // 直接下载 browser_download_url
    match try_download(browser_url) {
        Ok(data) => {
            if validate_archive(&data, asset_name).is_ok() {
                return Ok(data);
            }
            eprintln!("{}", "直接下载的文件格式无效，尝试代理下载...".yellow());
        }
        Err(e) => {
            eprintln!("{}", format!("直接下载失败: {}", e).yellow());
        }
    }

    if browser_url.starts_with("https://github.com/") {
        for proxy in GITHUB_PROXIES {
            let proxy_url = format!("{}{}", proxy, browser_url);
            println!("{}", format!("尝试代理: {}", proxy_url).cyan());
            match try_download(&proxy_url) {
                Ok(data) => {
                    if validate_archive(&data, asset_name).is_ok() {
                        return Ok(data);
                    }
                    eprintln!("{}", format!("代理 {} 返回的文件格式无效", proxy).yellow());
                }
                Err(e) => {
                    eprintln!("{}", format!("代理 {} 下载失败: {}", proxy, e).yellow());
                }
            }
        }
    }

    anyhow::bail!("所有下载方式均失败，请手动下载: {}", browser_url)
}

fn validate_archive(data: &[u8], asset_name: &str) -> Result<()> {
    if asset_name.ends_with(".zip") {
        if data.len() < 4 || &data[..4] != b"PK\x03\x04" {
            anyhow::bail!(
                "下载的文件不是有效的 ZIP 格式（可能是 HTML 页面或损坏的数据），请检查网络或稍后重试"
            );
        }
    } else if asset_name.ends_with(".tar.gz") && (data.len() < 2 || &data[..2] != b"\x1f\x8b") {
        anyhow::bail!(
            "下载的文件不是有效的 tar.gz 格式（可能是 HTML 页面或损坏的数据），请检查网络或稍后重试"
        );
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

    let resp = req.call().context("API 下载失败")?;

    let content_type = resp.content_type().to_string();
    let mut data = Vec::new();
    resp.into_reader()
        .read_to_end(&mut data)
        .context("读取下载内容失败")?;

    if content_type.contains("text/html") || content_type.contains("application/json") {
        anyhow::bail!(
            "API 返回了非二进制内容 (Content-Type: {}), 大小: {} bytes",
            content_type,
            data.len()
        );
    }
    Ok(data)
}

fn try_download(url: &str) -> Result<Vec<u8>> {
    let resp = ureq::get(url)
        .set("User-Agent", "pma-self-update")
        .call()
        .context("下载安装包失败")?;

    let content_type = resp.content_type().to_string();
    let mut data = Vec::new();
    resp.into_reader()
        .read_to_end(&mut data)
        .context("读取下载内容失败")?;

    if content_type.contains("text/html") {
        let preview = String::from_utf8_lossy(&data[..data.len().min(200)]);
        anyhow::bail!(
            "返回了 HTML 页面而非二进制文件 (大小: {} bytes), 内容预览: {}",
            data.len(),
            preview
        );
    }
    Ok(data)
}

fn get_asset_name(tag: &str) -> Result<String> {
    let (os, arch, ext) = match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => ("linux", "x86_64", "tar.gz"),
        ("macos", "x86_64") => ("macos", "x86_64", "tar.gz"),
        ("macos", "aarch64") => ("macos", "arm64", "tar.gz"),
        ("windows", "x86_64") => ("windows", "x86_64", "zip"),
        ("windows", "aarch64") => ("windows", "arm64", "zip"),
        (os, arch) => anyhow::bail!("不支持的平台: {}-{}", os, arch),
    };
    Ok(format!("pma-{}-{}-{}.{}", os, arch, tag, ext))
}

fn install_binary(data: &[u8], asset_name: &str, target: &PathBuf) -> Result<()> {
    let bin_name = if cfg!(windows) { "pma.exe" } else { "pma" };

    if asset_name.ends_with(".tar.gz") {
        install_from_tar_gz(data, bin_name, target)
    } else if asset_name.ends_with(".zip") {
        install_from_zip(data, bin_name, target)
    } else {
        anyhow::bail!("未知的安装包格式: {}", asset_name)
    }
}

fn install_from_tar_gz(data: &[u8], bin_name: &str, target: &PathBuf) -> Result<()> {
    let decoder = flate2::read::GzDecoder::new(io::Cursor::new(data));
    let mut archive = tar::Archive::new(decoder);

    for entry in archive.entries().context("读取 tar.gz 失败")? {
        let mut entry = entry.context("读取 tar entry 失败")?;
        let path = entry.path().context("读取 entry 路径失败")?;
        if path.file_name().and_then(|n| n.to_str()) == Some(bin_name) {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            return replace_binary(&buf, target);
        }
    }
    anyhow::bail!("在 tar.gz 中未找到 {}", bin_name)
}

fn install_from_zip(data: &[u8], bin_name: &str, target: &PathBuf) -> Result<()> {
    let cursor = io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).context("读取 zip 失败")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("读取 zip entry 失败")?;
        let name = file.name().to_string();
        if name.ends_with(bin_name) {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            return replace_binary(&buf, target);
        }
    }
    anyhow::bail!("在 zip 中未找到 {}", bin_name)
}

fn replace_binary(new_binary: &[u8], target: &PathBuf) -> Result<()> {
    // 先将旧文件重命名为 .bak，再写入新文件
    let backup = target.with_extension("bak");
    if backup.exists() {
        let _ = fs::remove_file(&backup);
    }
    fs::rename(target, &backup).context("备份旧版本失败")?;
    fs::write(target, new_binary).context("写入新版本失败")?;

    // Unix 下设置可执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(target, fs::Permissions::from_mode(0o755))?;
    }

    let _ = fs::remove_file(&backup);
    Ok(())
}
