//! `pma self`：查询自身版本，或从 GitHub 最新 release 下载对应平台的包替换当前二进制。
//! update 流程为：fetch_latest_release → 去掉 tag 的 `v` 后按 semver 比较 → 选出本平台 asset
//! → 产出 DownloadAndInstall 操作，真正的下载与替换发生在 execute 阶段。

use crate::commands::Command;
use crate::domain::self_update::Release;
use crate::domain::self_update::{SelfUpdateError, asset_name, fetch_latest_release};
use crate::engine::plan;
use crate::error::Result;
use crate::model::operation::SelfUpdateOperation;
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase};
use std::env;

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

/// `pma self` 的子命令：升级自身（update/up）与查看版本（version/ver）
#[derive(Debug, clap::Subcommand)]
pub enum SelfManageArgs {
    /// Update to the latest version from GitHub releases
    #[command(visible_alias = "up")]
    Update(UpdateArgs),
    /// Display the current version
    #[command(visible_alias = "ver")]
    Version,
}

/// `pma self update`：升级到最新版；`force` 可跳过已是最新版的判断
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

/// `pma self version` 的采集结果，全部来自编译期常量与当前平台信息
#[derive(Debug)]
pub(crate) struct VersionContext {
    package_name: &'static str,
    package_version: &'static str,
    os: &'static str,
    arch: &'static str,
}

// Version 子命令没有参数，用它这个零大小类型承接 Command 实现
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

/// update 的采集结果：当前/最新版本、安装渠道与强制执行标记
#[derive(Debug)]
pub(crate) struct UpdateContext {
    current: &'static str,
    latest: String,
    /// 是否由 npm 安装：npm 装的不能自替换，只能提示用 npm 更新
    is_npm: bool,
    force: bool,
    is_latest: bool,
    release: Release,
}

impl Command for UpdateArgs {
    type Context = UpdateContext;
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<UpdateContext> {
        // npm 安装的包装脚本（npm/pma/bin/pma）会带上这个环境变量
        let is_npm = env::var("PMA_NPM_INSTALL").is_ok();

        let release = fetch_latest_release()?;
        // release tag 形如 v1.2.3，semver 解析前要先去掉 v 前缀
        let latest = release.tag_name.trim_start_matches('v').to_string();

        let latest_ver = semver::Version::parse(&latest).map_err(|e| {
            SelfUpdateError::InvalidLatestVersion {
                version: latest.clone(),
                source: e,
            }
        })?;
        let current_ver = semver::Version::parse(PKG_VERSION).map_err(|e| {
            SelfUpdateError::InvalidCurrentVersion {
                version: PKG_VERSION.into(),
                source: e,
            }
        })?;

        // 用 >= 而非 ==：本地可能是源码编译或比 release 更新的版本
        let is_latest = current_ver >= latest_ver;

        if is_npm {
            // npm 安装的版本交给 npm 管理，不自替换；is_latest 置 false 表示跳过版本比较（release 信息该分支用不上）
            return Ok(UpdateContext {
                current: PKG_VERSION,
                latest,
                is_npm: true,
                force: false,
                is_latest: false,
                release,
            });
        }

        // 已是最新版且没加 --force 就直接失败退出，不做任何下载
        if is_latest && !self.force {
            return Err(SelfUpdateError::AlreadyLatest.into());
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
                msg: "检测到通过 npm 安装，请使用 npm 更新".to_string(),
            });
            plan.add_message(DisplayMessage::Item {
                label: "更新命令".to_string(),
                value: "npm update -g @jeansoft/pma".to_string(),
            });
            return Ok(plan);
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

        // 资产名按 CI 产出约定拼出，形如 pma-macos-arm64-v1.2.3.tar.gz；tag 需带 v 前缀
        let asset_name = asset_name(&ctx.release.tag_name)?;
        let asset = ctx
            .release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| SelfUpdateError::AssetNotFound {
                asset_name: asset_name.clone(),
            })?;

        let mut update_phase = Phase::new("下载更新");
        // api_url 需带 octet-stream 才返回二进制，browser_url 是免鉴权的下载回退源
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

/// 分发 `pma self` 子命令
pub fn run(args: SelfManageArgs) -> Result<()> {
    match args {
        SelfManageArgs::Update(update_args) => Command::run(&update_args),
        SelfManageArgs::Version => Command::run(&VersionMarker),
    }
}
