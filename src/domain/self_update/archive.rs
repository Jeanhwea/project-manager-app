//! 从下载到的 tar.gz / zip 里取出 pma 可执行文件的内容。

use crate::domain::self_update::SelfUpdateError;
use crate::error::Result;
use std::io::{self, Read};

/// 按 `asset_name` 的后缀选择解包方式，返回 `bin_name` 对应文件的字节
pub fn extract_binary(data: &[u8], asset_name: &str, bin_name: &str) -> Result<Vec<u8>> {
    if asset_name.ends_with(".tar.gz") {
        extract_from_tar_gz(data, bin_name)
    } else if asset_name.ends_with(".zip") {
        extract_from_zip(data, bin_name)
    } else {
        Err(SelfUpdateError::UnknownArchiveFormat {
            asset_name: asset_name.to_string(),
        }
        .into())
    }
}

fn extract_from_tar_gz(data: &[u8], bin_name: &str) -> Result<Vec<u8>> {
    let decoder = flate2::read::GzDecoder::new(io::Cursor::new(data));
    let mut archive = tar::Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|e| SelfUpdateError::TarRead { source: e })?
    {
        let mut entry = entry.map_err(|e| SelfUpdateError::TarEntry { source: e })?;
        let path = entry
            .path()
            .map_err(|e| SelfUpdateError::TarEntryPath { source: e })?;
        // 按文件名而非完整路径匹配：包里的二进制可能顶层裸放，也可能套一层目录
        if path.file_name().and_then(|n| n.to_str()) == Some(bin_name) {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }
    Err(SelfUpdateError::BinaryNotFoundInTar {
        bin_name: bin_name.to_string(),
    }
    .into())
}

fn extract_from_zip(data: &[u8], bin_name: &str) -> Result<Vec<u8>> {
    let cursor = io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| SelfUpdateError::ZipRead {
        source: Box::new(e),
    })?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| SelfUpdateError::ZipEntry {
            source: Box::new(e),
        })?;
        let name = file.name().to_string();
        // 按后缀而非全等匹配：zip 内条目名可能带目录前缀；代价是可能误命中同后缀名的其它文件
        if name.ends_with(bin_name) {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }
    Err(SelfUpdateError::BinaryNotFoundInZip {
        bin_name: bin_name.to_string(),
    }
    .into())
}
