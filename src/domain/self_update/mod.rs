mod updater;

pub use updater::{
    Release, download_asset, fetch_latest_release, get_asset_name, install_binary,
};
