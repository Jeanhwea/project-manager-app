use crate::control::pipeline::Pipeline;
use crate::domain::AppError;
use crate::model::plan::{ExecutionPlan, MessageOperation};
use crate::utils::output::Output;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
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

#[derive(Debug, clap::Subcommand)]
pub enum SelfManageArgs {
    /// Update to the latest version from GitHub releases
    #[command(visible_alias = "up")]
    Update(UpdateArgs),
    /// Display the current version
    #[command(visible_alias = "ver")]
    Version,
}

#[derive(Debug, clap::Args)]
pub struct UpdateArgs {
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Force update even if already on the latest version"
    )]
    pub force: bool,
}

struct VersionContext {
    pkg_name: &'static str,
    pkg_version: &'static str,
    os: &'static str,
    arch: &'static str,
}

#[allow(dead_code)]
struct UpdateContext {
    current: &'static str,
    latest: String,
    force: bool,
    release: Release,
}

pub fn run(args: SelfManageArgs) -> Result<()> {
    match args {
        SelfManageArgs::Update(update_args) => Pipeline::run(update_args, get_update_context, make_update_plan),
        SelfManageArgs::Version => Pipeline::run(VersionMarker, get_version_context, make_version_plan),
    }
}

struct VersionMarker;

fn get_version_context(_args: &VersionMarker) -> anyhow::Result<VersionContext> {
    Ok(VersionContext {
        pkg_name: PKG_NAME,
        pkg_version: PKG_VERSION,
        os: env::consts::OS,
        arch: env::consts::ARCH,
    })
}

fn make_version_plan(_args: &VersionMarker, ctx: &VersionContext) -> anyhow::Result<ExecutionPlan> {
    let mut plan = ExecutionPlan::new();
    plan.add(MessageOperation::Skip {
        msg: format!(
            "{} v{} ({}-{})",
            ctx.pkg_name, ctx.pkg_version, ctx.os, ctx.arch
        ),
    });
    Ok(plan)
}

fn get_update_context(args: &UpdateArgs) -> anyhow::Result<UpdateContext> {
    if env::var("PMA_NPM_INSTALL").is_ok() {
        return Err(AppError::SelfUpdate(
            "检测到通过 npm 安装，请使用 npm 更新:\n  npm update -g @jeansoft/pma".to_string(),
        )
        .into());
    }

    Output::info("检查最新版本...");

    let release = fetch_latest_release().context("获取发布信息失败")?;
    let latest = release.tag_name.trim_start_matches('v').to_string();

    Output::item("当前版本", &format!("v{}", PKG_VERSION));
    Output::item("最新版本", &format!("v{}", latest));

    let latest_ver = semver::Version::parse(&latest)
        .with_context(|| format!("无法解析最新版本号: {}", latest))?;
    let current_ver = semver::Version::parse(PKG_VERSION)
        .with_context(|| format!("无法解析当前版本号: {}", PKG_VERSION))?;

    if current_ver >= latest_ver && !args.force {
        return Err(AppError::SelfUpdate("已经是最新版本，无需更新。".to_string()).into());
    }

    if current_ver >= latest_ver && args.force {
        Output::warning("强制更新模式，继续更新...");
    }

    Ok(UpdateContext {
        current: PKG_VERSION,
        latest,
        force: args.force,
        release,
    })
}

fn make_update_plan(_args: &UpdateArgs, ctx: &UpdateContext) -> anyhow::Result<ExecutionPlan> {
    let asset_name = get_asset_name(&ctx.release.tag_name)
        .map_err(|e| AppError::self_update(format!("获取资源名称失败: {}", e)))?;
    let asset = ctx
        .release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            AppError::self_update(format!("未找到适合当前平台的安装包: {}", asset_name))
        })?;

    Output::info(&format!("下载 {}...", asset.name));
    let data = download_asset(&asset.url, &asset.browser_download_url, &asset.name)
        .context("下载资源失败")?;
    Output::success("下载完成");

    let current_exe = env::current_exe().context("无法获取当前可执行文件路径")?;
    install_binary(&data, &asset.name, &current_exe).context("安装二进制文件失败")?;

    let mut plan = ExecutionPlan::new();
    plan.add(MessageOperation::Success {
        msg: format!("更新成功! v{} -> v{}", ctx.current, ctx.latest),
    });
    Ok(plan)
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

    anyhow::bail!(
        "所有下载方式均失败，请手动下载: {}\n\
         提示: 可设置 PMA_DOWNLOAD_URL 环境变量指定下载地址，\n\
         或设置 GITHUB_TOKEN 环境变量提高 API 下载成功率",
        browser_url
    )
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
        return Err(AppError::self_update("下载的文件格式无效".to_string()).into());
    }
    Ok(())
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
        let n = reader.read(&mut buf).context("读取下载内容失败")?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        pb.set_position(data.len() as u64);
    }
    pb.finish_and_clear();
    Ok(data)
}

fn try_download_api(api_url: &str) -> Result<Vec<u8>> {
    let mut req = ureq::get(api_url)
        .set("User-Agent", "pma-self-update")
        .set("Accept", "application/octet-stream");

    if let Ok(token) = env::var("GITHUB_TOKEN") {
        req = req.set("Authorization", &format!("Bearer {}", token));
    }

    let resp = req.call().context("API 下载失败")?;
    read_response_with_progress(resp)
}

fn try_download(url: &str) -> Result<Vec<u8>> {
    let resp = ureq::get(url)
        .set("User-Agent", "pma-self-update")
        .call()
        .context("下载安装包失败")?;

    read_response_with_progress(resp)
}

fn get_asset_name(tag: &str) -> Result<String> {
    let (os, arch, ext) = match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => ("linux", "x86_64", "tar.gz"),
        ("macos", "x86_64") => ("macos", "x86_64", "tar.gz"),
        ("macos", "aarch64") => ("macos", "arm64", "tar.gz"),
        ("windows", "x86_64") => ("windows", "x86_64", "zip"),
        ("windows", "aarch64") => ("windows", "arm64", "zip"),
        (os, arch) => {
            return Err(AppError::not_supported(format!("{}-{}", os, arch)).into());
        }
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
        Err(AppError::self_update(format!("未知的安装包格式: {}", asset_name)).into())
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
    Err(AppError::self_update(format!("在 tar.gz 中未找到 {}", bin_name)).into())
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
    Err(AppError::self_update(format!("在 zip 中未找到 {}", bin_name)).into())
}

fn replace_binary(new_binary: &[u8], target: &PathBuf) -> Result<()> {
    let backup = target.with_extension("bak");
    if backup.exists() {
        let _ = fs::remove_file(&backup);
    }
    fs::rename(target, &backup).context("备份旧版本失败")?;
    fs::write(target, new_binary).context("写入新版本失败")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(target, fs::Permissions::from_mode(0o755))?;
    }

    let _ = fs::remove_file(&backup);
    Ok(())
}
