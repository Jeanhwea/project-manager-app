//! 自更新：查 GitHub 最新 release，下载当前平台的安装包，解包后替换自身可执行文件。
//! 按流程拆成 release（查版本）、download（多源下载）、archive（解包）、installer（落盘）四步。

mod archive;
mod download;
mod error;
mod installer;
mod release;

pub use download::download_asset;
pub use error::SelfUpdateError;
pub use installer::{asset_name, install_binary};
pub use release::{Release, fetch_latest_release};
