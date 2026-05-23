#[derive(Debug, thiserror::Error)]
pub enum SelfUpdateError {
    #[error("请求 GitHub API 失败")]
    FetchReleaseRequest {
        #[source]
        source: Box<ureq::Error>,
    },

    #[error("解析 release 信息失败")]
    ParseReleaseJson {
        #[source]
        source: std::io::Error,
    },

    #[error("无法解析最新版本号: {version}")]
    InvalidLatestVersion {
        version: String,
        #[source]
        source: semver::Error,
    },

    #[error("无法解析当前版本号: {version}")]
    InvalidCurrentVersion {
        version: String,
        #[source]
        source: semver::Error,
    },

    #[error("已经是最新版本，无需更新。")]
    AlreadyLatest,

    #[error("请使用 npm 更新")]
    UseNpmUpdate,

    #[error("未找到适合当前平台的安装包: {asset_name}")]
    AssetNotFound { asset_name: String },

    #[error("API 下载失败")]
    ApiDownload {
        #[source]
        source: Box<ureq::Error>,
    },

    #[error("下载安装包失败")]
    AssetDownload {
        #[source]
        source: Box<ureq::Error>,
    },

    #[error("读取下载内容失败")]
    ReadDownload {
        #[source]
        source: std::io::Error,
    },

    #[error("下载的文件格式无效")]
    InvalidArchiveMagic,

    #[error(
        "所有下载方式均失败，请手动下载: {browser_url}\n\
         提示: 可设置 PMA_DOWNLOAD_URL 环境变量指定下载地址，\n\
         或设置 GITHUB_TOKEN 环境变量提高 API 下载成功率"
    )]
    AllDownloadAttemptsFailed { browser_url: String },

    #[error("未知的安装包格式: {asset_name}")]
    UnknownArchiveFormat { asset_name: String },

    #[error("读取 tar.gz 失败")]
    TarRead {
        #[source]
        source: std::io::Error,
    },

    #[error("读取 tar entry 失败")]
    TarEntry {
        #[source]
        source: std::io::Error,
    },

    #[error("读取 entry 路径失败")]
    TarEntryPath {
        #[source]
        source: std::io::Error,
    },

    #[error("在 tar.gz 中未找到 {bin_name}")]
    BinaryNotFoundInTar { bin_name: String },

    #[error("读取 zip 失败")]
    ZipRead {
        #[source]
        source: Box<zip::result::ZipError>,
    },

    #[error("读取 zip entry 失败")]
    ZipEntry {
        #[source]
        source: Box<zip::result::ZipError>,
    },

    #[error("在 zip 中未找到 {bin_name}")]
    BinaryNotFoundInZip { bin_name: String },

    #[error("备份旧版本失败")]
    BackupOld {
        #[source]
        source: std::io::Error,
    },

    #[error("写入新版本失败")]
    WriteNew {
        #[source]
        source: std::io::Error,
    },

    #[error("无法获取当前可执行文件路径")]
    CurrentExePath {
        #[source]
        source: std::io::Error,
    },
}
