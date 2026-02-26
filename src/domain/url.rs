use std::ops::Deref;

use thiserror::Error;
use url::Url as UrlParser;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Url(String);

#[derive(Debug, Error)]
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
    Invalid(#[from] url::ParseError),
}

impl Url {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }

    fn validate(input: &str) -> Result<(), UrlParseError> {
        let url = UrlParser::parse(input)?;

        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(UrlParseError::WrongScheme(scheme.to_owned()));
        }

        if !url.username().is_empty() || url.password().is_some() {
            return Err(UrlParseError::ContainsUserinfo);
        }

        let host = url
            .host_str()
            .ok_or(UrlParseError::EmptyHost)?
            .trim_end_matches('.')
            .to_ascii_lowercase();

        if host.is_empty() {
            return Err(UrlParseError::EmptyHost);
        }

        if host == "localhost" || host.ends_with(".local") || !host.contains('.') {
            return Err(UrlParseError::BlockedHost(host.to_owned()));
        }

        if host == "127.0.0.1" {
            return Err(UrlParseError::BlockedHost(host.to_owned()));
        }

        Ok(())
    }
}

impl Deref for Url {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl TryFrom<String> for Url {
    type Error = UrlParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::validate(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<&str> for Url {
    type Error = UrlParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(value.to_owned()))
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
            let result = Url::try_from(url);
            assert!(result.is_ok(), "{url} should be allowed, got: {result:?}");
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
            let result = Url::try_from(url);
            assert!(
                result.is_err(),
                "{url} should not be allowed, got: {result:?}"
            );
        }
    }

    #[test]
    fn saved_url_format() {
        let test_url = "https://example.com";
        let url = Url::try_from(test_url).expect("Could not parse the URL");
        assert_eq!(test_url, url.as_str(), "Saved URL does not match the input");
    }
}
