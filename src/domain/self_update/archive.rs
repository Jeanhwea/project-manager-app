use crate::domain::self_update::SelfUpdateError;
use crate::error::Result;
use std::io::{self, Read};

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
