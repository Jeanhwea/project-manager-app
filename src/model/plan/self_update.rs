#[derive(Debug, Clone)]
pub enum SelfUpdateOperation {
    DownloadAndInstall {
        api_url: String,
        browser_url: String,
        asset_name: String,
        current_version: String,
        target_version: String,
    },
}

impl SelfUpdateOperation {
    pub fn description(&self) -> String {
        match self {
            SelfUpdateOperation::DownloadAndInstall {
                asset_name,
                current_version,
                target_version,
                ..
            } => {
                format!(
                    "download {} and update v{} -> v{}",
                    asset_name, current_version, target_version
                )
            }
        }
    }
}
