use thiserror::Error;

use url::Url as InnerUrl;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Url(InnerUrl);

#[derive(Error, Debug)]
pub enum UrlParseError {
    #[error("contains userinfo")]
    ContainsUserinfo,
    #[error("scheme `{0}` is not allowed")]
    WrongScheme(String),
    #[error("domain `{0}` is blocked")]
    BlockedDomain(String),
    #[error("URL does not contain a domain")]
    EmptyDomain,
    #[error("could not parse the URL")]
    Invalid(url::ParseError),
}

impl Url {
    pub fn parse(input: &str) -> Result<Self, UrlParseError> {
        let url = InnerUrl::parse(input).map_err(UrlParseError::Invalid)?;

        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(UrlParseError::WrongScheme(scheme.to_string()));
        }

        if !url.username().is_empty() || url.password().is_some() {
            return Err(UrlParseError::ContainsUserinfo);
        }

        let domain = url.domain().unwrap_or("");
        if domain.is_empty() {
            return Err(UrlParseError::EmptyDomain);
        }
        if domain
            .trim_end_matches(".")
            .to_ascii_lowercase()
            .eq_ignore_ascii_case("localhost")
            || domain.ends_with(".local")
            || !domain.contains('.')
        {
            return Err(UrlParseError::BlockedDomain(domain.to_string()));
        }

        Ok(Self(url))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn allowed_urls() {
        let urls = [
            "http://example.com",
            "https://example.com",
            "https://www.example.com",
            "https://example.com:12345",
        ];

        for url in urls {
            let result = Url::parse(url);
            assert!(
                result.is_ok(),
                "{} should be allowed, instead: {:?}",
                url,
                result
            );
        }
    }

    #[test]
    fn disallowed_urls() {
        let urls = [
            "",
            "example",
            ".com",
            "http",
            "http://",
            "example.com",
            "ssh://example.com",
            "https://name@hunter2:example.com",
            "127.0.0.1",
            "127..1",
            "ftp://user:password@hostname.com/txt.txt",
            "ssh://login@server.com:12345/repository.git",
            "http://user:password@hostname.com/txt.txt",
            "https:///home/user/.bashrc",
            "http://login@server.com:12345/repository.git",
            "https:/run/foo.socket",
            "http://localhost/txt.txt",
            "https://127.0.0.1/txt.txt",
            "http://localhost.",
        ];

        for url in urls {
            let result = Url::parse(url);
            assert!(
                result.is_err(),
                "{} should not be allowed, instead: {:?}",
                url,
                result
            );
        }
    }
}
