pub mod command;
pub mod repository;

#[derive(Debug, Clone)]
pub struct Remote {
    pub name: String,
    pub url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Git command failed: {0}")]
    CommandFailed(String),

    #[error("Invalid remote URL: {0}")]
    InvalidRemoteUrl(String),

    #[error("Remote not found: {0}")]
    RemoteNotFound(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GitProtocol {
    Ssh,
    Http,
    Https,
    Git,
}

pub type Result<T> = std::result::Result<T, GitError>;

pub fn detect_protocol(url: &str) -> Result<GitProtocol> {
    let url = url.trim();
    if url.is_empty() {
        return Err(GitError::InvalidRemoteUrl("Empty URL".to_string()));
    }

    if url.starts_with("git@") || url.starts_with("ssh://") {
        return Ok(GitProtocol::Ssh);
    }

    if url.starts_with("https://") {
        return Ok(GitProtocol::Https);
    }
    if url.starts_with("http://") {
        return Ok(GitProtocol::Http);
    }

    if url.starts_with("git://") {
        return Ok(GitProtocol::Git);
    }

    if url.contains('@') && url.contains(':') {
        return Ok(GitProtocol::Ssh);
    }

    Err(GitError::InvalidRemoteUrl(format!(
        "Invalid URL format: {}",
        url
    )))
}

pub fn extract_host_and_path(url: &str) -> Option<(String, String)> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    let protocol = detect_protocol(url).ok()?;

    let (normalized, separator) = normalize_url(url, &protocol);

    let parts: Vec<&str> = normalized.splitn(2, separator).collect();
    if parts.len() != 2 {
        return None;
    }

    Some((parts[0].to_string(), parts[1].to_string()))
}

fn normalize_url(url: &str, protocol: &GitProtocol) -> (String, char) {
    match protocol {
        GitProtocol::Ssh if url.starts_with("git@") => (url.replace("git@", ""), ':'),
        GitProtocol::Ssh if url.starts_with("ssh://") => {
            let stripped = url.replace("ssh://", "");
            let stripped = stripped.replacen("git@", "", 1);
            (stripped, '/')
        }
        GitProtocol::Https => (url.replace("https://", ""), '/'),
        GitProtocol::Http => (url.replace("http://", ""), '/'),
        GitProtocol::Git => (url.replace("git://", ""), '/'),
        _ => (url.to_string(), ':'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_protocol_ssh() {
        assert_eq!(
            detect_protocol("git@github.com:user/repo.git").unwrap(),
            GitProtocol::Ssh
        );
        assert_eq!(
            detect_protocol("ssh://git@github.com/user/repo.git").unwrap(),
            GitProtocol::Ssh
        );
    }

    #[test]
    fn test_detect_protocol_https() {
        assert_eq!(
            detect_protocol("https://github.com/user/repo.git").unwrap(),
            GitProtocol::Https
        );
    }

    #[test]
    fn test_detect_protocol_http() {
        assert_eq!(
            detect_protocol("http://github.com/user/repo.git").unwrap(),
            GitProtocol::Http
        );
    }

    #[test]
    fn test_detect_protocol_git() {
        assert_eq!(
            detect_protocol("git://github.com/user/repo.git").unwrap(),
            GitProtocol::Git
        );
    }

    #[test]
    fn test_detect_protocol_heuristic_ssh() {
        assert_eq!(detect_protocol("user@host:path").unwrap(), GitProtocol::Ssh);
    }

    #[test]
    fn test_detect_protocol_invalid() {
        assert!(detect_protocol("").is_err());
        assert!(detect_protocol("invalid-url").is_err());
    }

    #[test]
    fn test_extract_host_and_path_ssh() {
        let (host, path) = extract_host_and_path("git@github.com:user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");
    }

    #[test]
    fn test_extract_host_and_path_ssh_url_format() {
        let (host, path) = extract_host_and_path("ssh://git@github.com/user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");
    }

    #[test]
    fn test_extract_host_and_path_https() {
        let (host, path) = extract_host_and_path("https://github.com/user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");
    }

    #[test]
    fn test_extract_host_and_path_http() {
        let (host, path) = extract_host_and_path("http://github.com/user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");
    }

    #[test]
    fn test_extract_host_and_path_git() {
        let (host, path) = extract_host_and_path("git://github.com/user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");
    }

    #[test]
    fn test_extract_host_and_path_invalid() {
        assert!(extract_host_and_path("").is_none());
        assert!(extract_host_and_path("invalid-url").is_none());
    }
}
