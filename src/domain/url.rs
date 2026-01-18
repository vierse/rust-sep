use thiserror::Error;

use url::Url as UrlParser;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Url(String);

#[derive(Error, Debug)]
pub enum UrlParseError {
    #[error("contains userinfo")]
    ContainsUserinfo,
    #[error("scheme `{0}` is not allowed")]
    WrongScheme(String),
    #[error("host `{0}` is blocked")]
    BlockedHost(String),
    #[error("URL does not contain a host")]
    EmptyHost,
    #[error("could not parse the URL")]
    Invalid(url::ParseError),
}

impl Url {
    pub fn parse(input: &str) -> Result<Self, UrlParseError> {
        let url = UrlParser::parse(input).map_err(UrlParseError::Invalid)?;

        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(UrlParseError::WrongScheme(scheme.to_string()));
        }

        if !url.username().is_empty() || url.password().is_some() {
            return Err(UrlParseError::ContainsUserinfo);
        }

        let url_domain = url.domain().unwrap_or("");
        if url_domain.is_empty() {
            return Err(UrlParseError::EmptyHost);
        }
        if url_domain
            .trim_end_matches(".")
            .to_ascii_lowercase()
            .eq_ignore_ascii_case("localhost")
            || url_domain.ends_with(".local")
            || !url_domain.contains('.')
        {
            return Err(UrlParseError::BlockedHost(url_domain.to_string()));
        }

        Ok(Self(input.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
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

    #[test]
    fn saved_url_format() {
        let test_url = "https://example.com";
        let url = Url::parse(test_url).expect("Could not parse the URL");
        assert_eq!(test_url, url.as_str(), "Saved URL does not match the input");
    }
}
