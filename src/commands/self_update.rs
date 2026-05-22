use crate::commands::Command;
use crate::domain::self_update::Release;
use crate::domain::self_update::{fetch_latest_release, get_asset_name};
use crate::engine::plan;
use crate::error::{AppError, Result};
use crate::model::operation::SelfUpdateOperation;
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase};
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
    #[arg(
        long,
        default_value = "false",
        help = "Dry run: show what would be updated without downloading"
    )]
    pub dry_run: bool,
}

#[derive(Debug)]
pub(crate) struct VersionContext {
    package_name: &'static str,
    package_version: &'static str,
    os: &'static str,
    arch: &'static str,
}

struct VersionMarker;

impl Command for VersionMarker {
    type Context = VersionContext;
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<VersionContext> {
        Ok(VersionContext {
            package_name: PKG_NAME,
            package_version: PKG_VERSION,
            os: env::consts::OS,
            arch: env::consts::ARCH,
        })
    }

    fn plan(&self, ctx: &VersionContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();
        plan.add_message(DisplayMessage::Skip {
            msg: format!(
                "{} v{} ({}-{})",
                ctx.package_name, ctx.package_version, ctx.os, ctx.arch
            ),
        });
        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

#[derive(Debug)]
pub(crate) struct UpdateContext {
    current: &'static str,
    latest: String,
    is_npm: bool,
    force: bool,
    is_latest: bool,
    release: Release,
}

impl Command for UpdateArgs {
    type Context = UpdateContext;
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<UpdateContext> {
        let is_npm = env::var("PMA_NPM_INSTALL").is_ok();

        let release = fetch_latest_release()
            .map_err(|e| AppError::self_update(format!("获取发布信息失败: {}", e)))?;
        let latest = release.tag_name.trim_start_matches('v').to_string();

        let latest_ver = semver::Version::parse(&latest)
            .map_err(|_| AppError::self_update(format!("无法解析最新版本号: {}", latest)))?;
        let current_ver = semver::Version::parse(PKG_VERSION)
            .map_err(|_| AppError::self_update(format!("无法解析当前版本号: {}", PKG_VERSION)))?;

        let is_latest = current_ver >= latest_ver;

        if is_npm {
            return Ok(UpdateContext {
                current: PKG_VERSION,
                latest,
                is_npm: true,
                force: false,
                is_latest: false,
                release,
            });
        }

        if is_latest && !self.force {
            return Err(AppError::self_update("已经是最新版本，无需更新。"));
        }

        Ok(UpdateContext {
            current: PKG_VERSION,
            latest,
            is_npm: false,
            force: self.force,
            is_latest,
            release,
        })
    }

    fn plan(&self, ctx: &UpdateContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new().with_dry_run(self.dry_run);

        if ctx.is_npm {
            plan.add_message(DisplayMessage::Warning {
                msg: "检测到通过 npm 安装".to_string(),
            });
            plan.add_message(DisplayMessage::Item {
                label: "更新命令".to_string(),
                value: "npm update -g @jeansoft/pma".to_string(),
            });
            return Err(AppError::self_update("请使用 npm 更新"));
        }

        plan.add_message(DisplayMessage::Item {
            label: "当前版本".to_string(),
            value: format!("v{}", ctx.current),
        });
        plan.add_message(DisplayMessage::Item {
            label: "最新版本".to_string(),
            value: format!("v{}", ctx.latest),
        });

        if ctx.is_latest && ctx.force {
            plan.add_message(DisplayMessage::Warning {
                msg: "强制更新模式，继续更新...".to_string(),
            });
        }

        let asset_name = get_asset_name(&ctx.release.tag_name)?;
        let asset = ctx
            .release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| {
                AppError::self_update(format!("未找到适合当前平台的安装包: {}", asset_name))
            })?;

        let mut update_phase = Phase::new("下载更新");
        update_phase.add(SelfUpdateOperation::DownloadAndInstall {
            api_url: asset.url.clone(),
            browser_url: asset.browser_download_url.clone(),
            asset_name: asset.name.clone(),
            current_version: ctx.current.to_string(),
            target_version: ctx.latest.clone(),
        });
        plan.add_phase(update_phase);

        plan.add_message(DisplayMessage::Success {
            msg: format!("更新成功! v{} -> v{}", ctx.current, ctx.latest),
        });
        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

pub fn run(args: SelfManageArgs) -> Result<()> {
    match args {
        SelfManageArgs::Update(update_args) => Command::run(&update_args),
        SelfManageArgs::Version => Command::run(&VersionMarker),
    }
}
