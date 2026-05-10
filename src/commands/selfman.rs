use crate::control::command::Command;
use crate::domain::selfupdate::{
    DownloadContext as SelfUpdateContext, download_asset, fetch_latest_release, get_asset_name,
    install_binary,
};
use crate::error::{AppError, Result};
use crate::model::plan::{ExecutionPlan, MessageOperation};
use crate::utils::output::Output;
use std::env;

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

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

pub(crate) struct VersionContext {
    pkg_name: &'static str,
    pkg_version: &'static str,
    os: &'static str,
    arch: &'static str,
}

struct VersionMarker;

impl Command for VersionMarker {
    type Context = VersionContext;

    fn context(&self) -> Result<VersionContext> {
        Ok(VersionContext {
            pkg_name: PKG_NAME,
            pkg_version: PKG_VERSION,
            os: env::consts::OS,
            arch: env::consts::ARCH,
        })
    }

    fn plan(&self, ctx: &VersionContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();
        plan.add(MessageOperation::Skip {
            msg: format!(
                "{} v{} ({}-{})",
                ctx.pkg_name, ctx.pkg_version, ctx.os, ctx.arch
            ),
        });
        Ok(plan)
    }
}

impl Command for UpdateArgs {
    type Context = SelfUpdateContext;

    fn context(&self) -> Result<SelfUpdateContext> {
        if env::var("PMA_NPM_INSTALL").is_ok() {
            return Err(AppError::SelfUpdate(
                "检测到通过 npm 安装，请使用 npm 更新:\n  npm update -g @jeansoft/pma"
                    .to_string(),
            ));
        }

        Output::info("检查最新版本...");

        let release = fetch_latest_release()
            .map_err(|e| AppError::SelfUpdate(format!("获取发布信息失败: {}", e)))?;
        let latest = release.tag_name.trim_start_matches('v').to_string();

        Output::item("当前版本", &format!("v{}", PKG_VERSION));
        Output::item("最新版本", &format!("v{}", latest));

        let latest_ver = semver::Version::parse(&latest)
            .map_err(|_| AppError::SelfUpdate(format!("无法解析最新版本号: {}", latest)))?;
        let current_ver = semver::Version::parse(PKG_VERSION)
            .map_err(|_| AppError::SelfUpdate(format!("无法解析当前版本号: {}", PKG_VERSION)))?;

        if current_ver >= latest_ver && !self.force {
            return Err(AppError::SelfUpdate(
                "已经是最新版本，无需更新。".to_string(),
            ));
        }

        if current_ver >= latest_ver && self.force {
            Output::warning("强制更新模式，继续更新...");
        }

        Ok(SelfUpdateContext {
            current: PKG_VERSION,
            latest,
            release,
        })
    }

    fn plan(&self, ctx: &SelfUpdateContext) -> Result<ExecutionPlan> {
        let asset_name = get_asset_name(&ctx.release.tag_name)?;
        let asset = ctx
            .release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| {
                AppError::SelfUpdate(format!("未找到适合当前平台的安装包: {}", asset_name))
            })?;

        Output::info(&format!("下载 {}...", asset.name));
        let data = download_asset(&asset.url, &asset.browser_download_url, &asset.name)
            .map_err(|e| AppError::SelfUpdate(format!("下载资源失败: {}", e)))?;
        Output::success("下载完成");

        let current_exe = env::current_exe()
            .map_err(|e| AppError::SelfUpdate(format!("无法获取当前可执行文件路径: {}", e)))?;
        install_binary(&data, &asset.name, &current_exe)
            .map_err(|e| AppError::SelfUpdate(format!("安装二进制文件失败: {}", e)))?;

        let mut plan = ExecutionPlan::new();
        plan.add(MessageOperation::Success {
            msg: format!("更新成功! v{} -> v{}", ctx.current, ctx.latest),
        });
        Ok(plan)
    }
}

pub fn run(args: SelfManageArgs) -> Result<()> {
    match args {
        SelfManageArgs::Update(update_args) => Command::run(&update_args),
        SelfManageArgs::Version => Command::run(&VersionMarker),
    }
}
