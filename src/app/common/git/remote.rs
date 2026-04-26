const REDINF_PATH_PREFIXES: &[&str] = &["red_8/", "redtool/", "red_base/", "teampuzzle/"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitProtocol {
    Ssh,
    Https,
    Http,
}

pub fn parse_git_remote_url(url: &str) -> Option<(GitProtocol, String, String)> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    let protocol = if url.starts_with("git@") || url.starts_with("ssh://") {
        GitProtocol::Ssh
    } else if url.starts_with("https://") {
        GitProtocol::Https
    } else if url.starts_with("http://") {
        GitProtocol::Http
    } else {
        return None;
    };

    let (url, separator) = if url.starts_with("git@") {
        (url.replace("git@", ""), ':')
    } else if url.starts_with("ssh://") {
        let stripped = url.replace("ssh://", "");
        let stripped = if stripped.starts_with("git@") {
            stripped.replacen("git@", "", 1)
        } else {
            stripped
        };
        (stripped, '/')
    } else if url.starts_with("https://") {
        (url.replace("https://", ""), '/')
    } else if url.starts_with("http://") {
        (url.replace("http://", ""), '/')
    } else {
        (url.to_string(), ':')
    };

    let parts: Vec<&str> = url.splitn(2, separator).collect();
    if parts.len() != 2 {
        return None;
    }

    let (host, path) = (parts[0].to_string(), parts[1].to_string());
    Some((protocol, host, path))
}

pub fn get_remote_name_by_url(url: &str) -> Option<String> {
    let (_, host, path) = parse_git_remote_url(url)?;

    let remote_name = if host == "github.com" || host == "githubfast.com" {
        "github".to_string()
    } else if host == "gitana.jeanhwea.io" {
        "gitana".to_string()
    } else if host == "gitee.com" {
        if REDINF_PATH_PREFIXES
            .iter()
            .any(|prefix| path.to_lowercase().starts_with(prefix))
        {
            "redinf".to_string()
        } else {
            "gitee".to_string()
        }
    } else if is_private_ip(&host) {
        "private".to_string()
    } else {
        "origin".to_string()
    };
    Some(remote_name)
}

fn is_private_ip(host: &str) -> bool {
    let ip_part = host.split(':').next().unwrap_or(host);

    let octets: Vec<u8> = ip_part
        .split('.')
        .filter_map(|s| s.parse::<u8>().ok())
        .collect();

    if octets.len() != 4 {
        return false;
    }

    match (octets[0], octets[1]) {
        (10, _) => true,
        (172, second) if (16..=31).contains(&second) => true,
        (192, 168) => true,
        _ => false,
    }
}
