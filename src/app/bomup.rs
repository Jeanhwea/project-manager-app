use super::git;
use super::version::Version;

pub fn execute(bump_type: &str) {
    let current_tag = git::get_current_version().unwrap_or_else(|| "v0.0.0".to_string());
    println!("当前版本: {}", current_tag);

    let version = Version::from_tag(&current_tag).unwrap_or_default();
    let new_version = version.bump(bump_type);
    let new_tag = new_version.to_tag();

    println!("升级类型: {}", bump_type);
    println!("新版本: {}", new_tag);

    match git::create_tag(&new_tag) {
        Ok(()) => println!("成功创建并推送 tag: {}", new_tag),
        Err(e) => {
            eprintln!("错误: {}", e);
            std::process::exit(1);
        }
    }
}
