//! 推断当前平台的安装包名，并把解包出的新二进制替换到 pma 自身的可执行文件路径。

use super::archive::extract_binary;
use crate::domain::self_update::SelfUpdateError;
use crate::error::{AppError, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

/// 当前平台对应的发布资产名，如 `pma-macos-arm64-v1.2.3.tar.gz`；`tag` 需带 `v` 前缀。
/// 未收录的 OS/ARCH 组合返回 `AppError::NotSupported`。
pub fn asset_name(tag: &str) -> Result<String> {
    // 这里列出的是 CI 实际产出的组合，arch 名沿用发布资产的约定（macOS 的 arm64）
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

/// 解包并把新二进制写入 `target`（通常是 `std::env::current_exe()`）
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
    // 先改名再写新文件：Unix 下直接覆盖正在运行的二进制会报 ETXTBSY，
    // 而 Windows 允许改名运行中的 exe（只是不允许删除），改名后原路径即可写入
    fs::rename(target, &backup).map_err(|e| SelfUpdateError::BackupOld { source: e })?;
    fs::write(target, new_binary).map_err(|e| SelfUpdateError::WriteNew { source: e })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // 解包出来的文件权限由 tar/zip 决定，补上执行位；Windows 无执行位概念，无需处理
        fs::set_permissions(target, fs::Permissions::from_mode(0o755))?;
    }

    // 走到这里说明替换成功，删掉备份；若前面任一写入失败已提前返回，.bak 会被保留以便手工回滚。
    // Windows 上删除被占用的旧 exe 可能失败，故忽略删除结果
    let _ = fs::remove_file(&backup);
    Ok(())
}
