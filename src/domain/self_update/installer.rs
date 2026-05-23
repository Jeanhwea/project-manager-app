use super::archive::extract_binary;
use crate::domain::self_update::SelfUpdateError;
use crate::error::{AppError, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn asset_name(tag: &str) -> Result<String> {
    let (os, arch, ext) = match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => ("linux", "x86_64", "tar.gz"),
        ("macos", "x86_64") => ("macos", "x86_64", "tar.gz"),
        ("macos", "aarch64") => ("macos", "arm64", "tar.gz"),
        ("windows", "x86_64") => ("windows", "x86_64", "zip"),
        ("windows", "aarch64") => ("windows", "arm64", "zip"),
        (os, arch) => {
            return Err(AppError::NotSupported {
                what: format!("{}-{}", os, arch),
            });
        }
    };
    Ok(format!("pma-{}-{}-{}.{}", os, arch, tag, ext))
}

pub fn install_binary(data: &[u8], asset_name: &str, target: &PathBuf) -> Result<()> {
    let bin_name = if cfg!(windows) { "pma.exe" } else { "pma" };
    let new_binary = extract_binary(data, asset_name, bin_name)?;
    replace_binary(&new_binary, target)
}

fn replace_binary(new_binary: &[u8], target: &PathBuf) -> Result<()> {
    let backup = target.with_extension("bak");
    if backup.exists() {
        let _ = fs::remove_file(&backup);
    }
    fs::rename(target, &backup).map_err(|e| SelfUpdateError::BackupOld { source: e })?;
    fs::write(target, new_binary).map_err(|e| SelfUpdateError::WriteNew { source: e })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(target, fs::Permissions::from_mode(0o755))?;
    }

    let _ = fs::remove_file(&backup);
    Ok(())
}
