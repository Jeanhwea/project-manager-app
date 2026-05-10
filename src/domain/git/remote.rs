#[derive(Debug, Clone)]
pub struct Remote {
    pub name: String,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use crate::domain::git::{GitProtocol, detect_protocol, extract_host_and_path};

    #[test]
    fn test_remote_parse_url_valid() {
        assert_eq!(
            detect_protocol("git@github.com:user/repo.git").unwrap(),
            GitProtocol::Ssh
        );
        assert_eq!(
            detect_protocol("https://github.com/user/repo.git").unwrap(),
            GitProtocol::Https
        );
        assert_eq!(
            detect_protocol("http://github.com/user/repo.git").unwrap(),
            GitProtocol::Http
        );
        assert_eq!(
            detect_protocol("git://github.com/user/repo.git").unwrap(),
            GitProtocol::Git
        );
    }

    #[test]
    fn test_remote_parse_url_invalid() {
        assert!(detect_protocol("").is_err());
        assert!(detect_protocol("invalid-url").is_err());
    }

    #[test]
    fn test_remote_extract_host_and_path() {
        let (host, path) = extract_host_and_path("git@github.com:user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");

        let (host, path) = extract_host_and_path("https://github.com/user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(path, "user/repo.git");
    }

    #[test]
    fn test_remote_extract_host_and_path_invalid() {
        assert!(extract_host_and_path("").is_none());
        assert!(extract_host_and_path("invalid-url").is_none());
    }
}
