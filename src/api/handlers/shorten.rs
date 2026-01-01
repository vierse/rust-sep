use anyhow::Result;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::app::AppState;

#[derive(Serialize, Deserialize)]
pub struct ShortenRequest {
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct ShortenResponse {
    pub alias: String,
}

pub async fn shorten(
    State(app): State<AppState>,
    Json(ShortenRequest { url }): Json<ShortenRequest>,
) -> impl IntoResponse {
    if let Err(e) = validate_url(&url) {
        tracing::warn!(cause = %e, "URL verification failed");
        return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
    }

    match app.shorten_url(&url).await {
        Ok(alias) => (StatusCode::CREATED, Json(ShortenResponse { alias })).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "shorten request err");
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

/// encountered an Error while validating a url
/// `ParseErr` happens when the url is invalid
/// the others when we don't accept it for other reasons
#[derive(Debug, PartialEq, Eq)]
pub enum UrlError {
    ContainsUserinfo,
    WrongScheme,
    DisallowedDomain,
    EmptyDomain,
    ParseErr(url::ParseError),
}

impl std::fmt::Display for UrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContainsUserinfo => write!(f, "the host can't have passwords or usernames"),
            Self::WrongScheme => write!(f, "the url must be of either http or https scheme"),
            Self::DisallowedDomain => write!(f, "the url can't have localhost as host"),
            Self::EmptyDomain => write!(f, "the supplied url does not have a host"),
            Self::ParseErr(e) => write!(f, "failed parsing the url: {e}"),
        }
    }
}

fn validate_url(url: &str) -> Result<(), UrlError> {
    let url = Url::parse(url).map_err(UrlError::ParseErr)?;

    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(UrlError::WrongScheme);
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err(UrlError::ContainsUserinfo);
    }

    let domain = url.domain().unwrap_or("");
    if domain.is_empty() {
        return Err(UrlError::EmptyDomain);
    }
    if domain
        .trim_end_matches(".")
        .to_ascii_lowercase()
        .eq_ignore_ascii_case("localhost")
        || domain.ends_with(".local")
        || !domain.contains('.')
    {
        return Err(UrlError::DisallowedDomain);
    }

    Ok(())
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
            let result = validate_url(url);
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
            let result = validate_url(url);
            assert!(
                result.is_err(),
                "{} should not be allowed, instead: {:?}",
                url,
                result
            );
        }
    }
}
