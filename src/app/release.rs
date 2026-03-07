use super::git;
use super::version::Version;
use regex::Regex;

const CARGO_TOML: &str = "Cargo.toml";
const POM_XML: &str = "pom.xml";
const PYPROJECT_TOML: &str = "pyproject.toml";
const PYTHON_VERSION_FILE: &str = "src/__version__.py";

pub fn execute(bump_type: &str) {
    let current_branch = git::get_current_branch().unwrap_or_else(|| "master".to_string());
    if current_branch != "master" {
        eprintln!("错误: 只能在 master 分支上执行 release");
        std::process::exit(1);
    }

    let current_tag = git::get_current_version().unwrap_or_else(|| "v0.0.0".to_string());

    let rev_current_tag = git::get_rev_revision(&current_tag).unwrap();
    let rev_head = git::get_rev_revision("HEAD").unwrap();
    if rev_current_tag == rev_head {
        eprintln!("错误: 当前 HEAD 已被标记为 {}", current_tag);
        std::process::exit(1);
    }

    let version = Version::from_tag(&current_tag).unwrap_or_default();
    let new_version = version.bump(bump_type);
    let new_tag = new_version.to_tag();

    let config_files = detect_config_file();
    for config_file in config_files {
        release_config_file(&new_tag, &config_file);
        if let Err(e) = git::add_file(&config_file) {
            eprintln!("错误: {}", e);
            std::process::exit(1);
        }
    }

    if let Some(changes) = git::get_cached_changes() {
        for change in changes {
            if let Err(e) = git::add_file(&change) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
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

pub fn release_config_file(tag: &str, config_file: &str) {
    if config_file == CARGO_TOML {
        edit_cargo_toml_file(tag, config_file);
    } else if config_file == POM_XML {
        edit_pom_xml_file(tag, config_file);
    } else if config_file == PYPROJECT_TOML {
        edit_pyproject_toml_file(tag, config_file);
    } else if config_file == PYTHON_VERSION_FILE {
        edit_python_package_init_file(tag, config_file);
    } else {
        eprintln!("错误: 不支持的配置文件 {}", config_file);
        std::process::exit(1);
    }
}

pub fn detect_config_file() -> Vec<String> {
    let mut config_files = Vec::new();

    if std::path::Path::new(CARGO_TOML).exists() {
        config_files.push(CARGO_TOML.to_string());
    }
    if std::path::Path::new(POM_XML).exists() {
        config_files.push(POM_XML.to_string());
    }
    if std::path::Path::new(PYPROJECT_TOML).exists() {
        config_files.push(PYPROJECT_TOML.to_string());
    }
    if std::path::Path::new(PYTHON_VERSION_FILE).exists() {
        config_files.push(PYTHON_VERSION_FILE.to_string());
    }

    if config_files.is_empty() {
        eprintln!("错误: 未检测到 Cargo.toml、pom.xml 或 pyproject.toml 文件");
        std::process::exit(1);
    }

    config_files
}

pub fn edit_cargo_toml_file(tag: &str, config_file: &str) {
    let config_content = std::fs::read_to_string(config_file).unwrap_or_else(|e| {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    });

    let version = tag.trim_start_matches('v');

    let mut lines: Vec<String> = config_content.lines().map(|s| s.to_string()).collect();
    let mut in_package_section = false;

    for line in &mut lines {
        if line.trim() == "[package]" {
            in_package_section = true;
        } else if line.starts_with('[') && !line.trim().is_empty() {
            in_package_section = false;
        }

        if in_package_section && line.trim().starts_with("version = ") {
            *line = format!("version = \"{}\"", version);
            break;
        }
    }

    let new_config_content = lines.join("\n");

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

    let re = Regex::new(r#"<version>[0-9]+\.[0-9]+\.[0-9]+</version>"#).unwrap();
    let new_config_content = re.replace(&config_content, &format!(r#"<version>{}</version>"#, tag));

    std::fs::write(config_file, new_config_content.to_string()).unwrap_or_else(|e| {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    });
}

pub fn edit_pyproject_toml_file(tag: &str, config_file: &str) {
    let config_content = std::fs::read_to_string(config_file).unwrap_or_else(|e| {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    });

    let version = tag.trim_start_matches('v');

    let mut lines: Vec<String> = config_content.lines().map(|s| s.to_string()).collect();
    let mut in_project_section = false;

    for line in &mut lines {
        if line.trim() == "[project]" {
            in_project_section = true;
        } else if line.starts_with('[') && !line.trim().is_empty() {
            in_project_section = false;
        }

        if in_project_section && line.trim().starts_with("version = ") {
            *line = format!("version = \"{}\"", version);
            break;
        }
    }

    let new_config_content = lines.join("\n");

    std::fs::write(config_file, new_config_content).unwrap_or_else(|e| {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    });
}

pub fn edit_python_package_init_file(tag: &str, config_file: &str) {
    let version = tag.trim_start_matches('v');

    let content = std::fs::read_to_string(config_file).unwrap_or_else(|e| {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    });

    use regex::Regex;
    let re = Regex::new(r#"__version__ = "[^"]*""#).unwrap();
    let new_content = re.replace(&content, &format!(r#"__version__ = "{}""#, version));

    std::fs::write(config_file, new_content.as_ref()).unwrap_or_else(|e| {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    });
}
