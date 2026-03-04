use super::git;
use super::version::Version;

pub fn execute(bump_type: &str) {
    let current_tag = git::get_current_version().unwrap_or_else(|| "v0.0.0".to_string());

    let version = Version::from_tag(&current_tag).unwrap_or_default();
    let new_version = version.bump(bump_type);
    let new_tag = new_version.to_tag();

    let config_file = detect_config_file();
    bomup_config_file(&new_tag, &config_file);

    match git::add_file(&config_file) {
        Ok(()) => println!("成功添加文件 {}", config_file),
        Err(e) => {
            eprintln!("错误: {}", e);
            std::process::exit(1);
        }
    }

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

pub fn bomup_config_file(tag: &str, config_file: &str) {
    println!("升级 {} 为 {}", config_file, tag);
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

    let new_config_content =
        config_content.replace("version = \"0.0.0\"", &format!("version = \"{}\"", tag));

    std::fs::write(config_file, new_config_content).unwrap_or_else(|e| {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    });
}

pub fn edit_pom_xml_file(tag: &str, config_file: &str) {
    let config_content = std::fs::read_to_string(config_file).unwrap_or_else(|e| {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    });

    let new_config_content = config_content.replace(
        "<version>0.0.0</version>",
        &format!("<version>{}</version>", tag),
    );

    std::fs::write(config_file, new_config_content).unwrap_or_else(|e| {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    });
}
