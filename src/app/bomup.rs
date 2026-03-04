use super::git;
use super::version::Version;
use regex::Regex;

pub fn execute(bump_type: &str) {
    let current_branch = git::get_current_branch().unwrap_or_else(|| "master".to_string());
    if current_branch != "master" {
        eprintln!("错误: 只能在 master 分支上执行 bomup");
        std::process::exit(1);
    }

    let current_tag = git::get_current_version().unwrap_or_else(|| "v0.0.0".to_string());

    let version = Version::from_tag(&current_tag).unwrap_or_default();
    let new_version = version.bump(bump_type);
    let new_tag = new_version.to_tag();

    let config_file = detect_config_file();
    bomup_config_file(&new_tag, &config_file);

    if let Err(e) = git::add_file(&config_file) {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = git::commit(&format!("{}", new_tag)) {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = git::create_tag(&new_tag) {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }

    if let Some(remotes) = git::get_remote_list() {
        for remote in remotes {
            if let Err(e) = git::push_tag(&remote, &new_tag) {
                eprintln!("错误: {}", e);
            }
            if let Err(e) = git::push_branch(&remote, &current_branch) {
                eprintln!("错误: {}", e);
            }
        }
    }
}

pub fn bomup_config_file(tag: &str, config_file: &str) {
    if config_file == "Cargo.toml" {
        edit_cargo_toml_file(tag, config_file);
    } else if config_file == "pom.xml" {
        edit_pom_xml_file(tag, config_file);
    } else {
        eprintln!("错误: 不支持的配置文件 {}", config_file);
        std::process::exit(1);
    }
}

pub fn detect_config_file() -> String {
    if std::path::Path::new("Cargo.toml").exists() {
        "Cargo.toml".to_string()
    } else if std::path::Path::new("pom.xml").exists() {
        "pom.xml".to_string()
    } else {
        eprintln!("错误: 未检测到 Cargo.toml 或 pom.xml 文件");
        std::process::exit(1);
    }
}

pub fn edit_cargo_toml_file(tag: &str, config_file: &str) {
    let config_content = std::fs::read_to_string(config_file).unwrap_or_else(|e| {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    });

    let version = tag.trim_start_matches('v');
    let re = Regex::new(r#"version = "[0-9]+\.[0-9]+\.[0-9]+""#).unwrap();
    let new_config_content = re.replace(&config_content, &format!(r#"version = "{}""#, version));

    std::fs::write(config_file, new_config_content.to_string()).unwrap_or_else(|e| {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    });
}

pub fn edit_pom_xml_file(tag: &str, config_file: &str) {
    let config_content = std::fs::read_to_string(config_file).unwrap_or_else(|e| {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    });

    let re = Regex::new(r#"<version>[0-9]+\.[0-9]+\.[0-9]+</version>"#).unwrap();
    let new_config_content = re.replace(&config_content, &format!(r#"<version>{}</version>"#, tag));

    std::fs::write(config_file, new_config_content.to_string()).unwrap_or_else(|e| {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    });
}
