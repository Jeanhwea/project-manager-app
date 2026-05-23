mod archive;
mod download;
mod error;
mod installer;
mod release;

pub use download::download_asset;
pub use error::SelfUpdateError;
pub use installer::{asset_name, install_binary};
pub use release::{Release, fetch_latest_release};
