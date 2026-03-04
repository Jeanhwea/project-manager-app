use super::git;
use super::version::Version;

pub fn execute(bump_type: &str) {
    let current_tag = git::get_current_version().unwrap_or_else(|| "v0.0.0".to_string());

    let version = Version::from_tag(&current_tag).unwrap_or_default();
    let new_version = version.bump(bump_type);
    let new_tag = new_version.to_tag();

    match git::create_tag(&new_tag) {
        Ok(()) => println!("成功创建 tag: {}", new_tag),
        Err(e) => {
            eprintln!("错误: {}", e);
            std::process::exit(1);
        }
    }

    if let Some(remotes) = git::get_remote_list() {
        for remote in remotes {
            match git::push_tag(&remote, &new_tag) {
                Ok(()) => println!("成功推送 tag {} 到远程仓库 {}", new_tag, remote),
                Err(e) => eprintln!("错误: {}", e),
            }
        }
    }
}
