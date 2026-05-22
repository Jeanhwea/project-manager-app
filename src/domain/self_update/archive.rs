use crate::error::{AppError, Result};
use std::io::{self, Read};

pub fn extract_binary(data: &[u8], asset_name: &str, bin_name: &str) -> Result<Vec<u8>> {
    if asset_name.ends_with(".tar.gz") {
        extract_from_tar_gz(data, bin_name)
    } else if asset_name.ends_with(".zip") {
        extract_from_zip(data, bin_name)
    } else {
        Err(AppError::self_update(format!(
            "未知的安装包格式: {}",
            asset_name
        )))
    }
}

fn extract_from_tar_gz(data: &[u8], bin_name: &str) -> Result<Vec<u8>> {
    let decoder = flate2::read::GzDecoder::new(io::Cursor::new(data));
    let mut archive = tar::Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|e| AppError::self_update(format!("读取 tar.gz 失败: {}", e)))?
    {
        let mut entry =
            entry.map_err(|e| AppError::self_update(format!("读取 tar entry 失败: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| AppError::self_update(format!("读取 entry 路径失败: {}", e)))?;
        if path.file_name().and_then(|n| n.to_str()) == Some(bin_name) {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }
    Err(AppError::self_update(format!(
        "在 tar.gz 中未找到 {}",
        bin_name
    )))
}

fn extract_from_zip(data: &[u8], bin_name: &str) -> Result<Vec<u8>> {
    let cursor = io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| AppError::self_update(format!("读取 zip 失败: {}", e)))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| AppError::self_update(format!("读取 zip entry 失败: {}", e)))?;
        let name = file.name().to_string();
        if name.ends_with(bin_name) {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }
    Err(AppError::self_update(format!(
        "在 zip 中未找到 {}",
        bin_name
    )))
}
