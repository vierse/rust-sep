use anyhow::Result;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use time::format_description::well_known::Iso8601;
use url::Url;

use crate::app::AppState;

#[derive(Serialize, Deserialize)]
pub struct ShortenRequest {
    pub url: String,
    pub name: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ShortenResponse {
    pub alias: String,
}

pub async fn shorten(
    State(app): State<AppState>,
    Json(ShortenRequest {
        url,
        name,
        expires_at,
    }): Json<ShortenRequest>,
) -> impl IntoResponse {
    if let Err(e) = validate_url(&url) {
        tracing::warn!(cause = %e, "URL validation failed");
        return (StatusCode::BAD_REQUEST).into_response();
    }

    // Parse and validate expires_at if provided, otherwise default to 7 days
    let expires_at = match expires_at {
        Some(expires_str) => match validate_expires_at(&expires_str) {
            Ok(dt) => dt,
            Err(e) => {
                tracing::warn!(cause = %e, "expires_at validation failed");
                return (StatusCode::BAD_REQUEST).into_response();
            }
        },
        None => OffsetDateTime::now_utc() + time::Duration::days(7),
    };

    match name {
        // if request contains an alias, validate and save it
        Some(alias) => {
            if let Err(e) = validate_alias(&alias) {
                tracing::warn!(cause = %e, "alias validation failed");
                return (StatusCode::BAD_REQUEST).into_response();
            }
            match app.save_named_url(&alias, &url, Some(expires_at)).await {
                Ok(()) => (StatusCode::CREATED, Json(ShortenResponse { alias })).into_response(),
                Err(e) => e.into_response(),
            }
        }

        // if request does not contain an alias, generate a new one
        None => match app.shorten_url(&url, Some(expires_at)).await {
            Ok(alias) => (StatusCode::CREATED, Json(ShortenResponse { alias })).into_response(),
            Err(e) => {
                tracing::error!(error = %e, "shorten request error");
                (StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
        },
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ExpiresAtError {
    InvalidFormat,
    InThePast,
}

impl std::fmt::Display for ExpiresAtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "expires_at has invalid ISO 8601 format"),
            Self::InThePast => write!(f, "expires_at is in the past"),
        }
    }
}

fn validate_expires_at(expires_str: &str) -> Result<OffsetDateTime, ExpiresAtError> {
    let dt = OffsetDateTime::parse(expires_str, &Iso8601::DEFAULT)
        .map_err(|_| ExpiresAtError::InvalidFormat)?;

    if dt <= OffsetDateTime::now_utc() {
        return Err(ExpiresAtError::InThePast);
    }

    Ok(dt)
}
#[derive(Debug, PartialEq, Eq)]
enum AliasError {
    TooShort,
    TooLong,
    InvalidCharacters,
}

impl std::fmt::Display for AliasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "alias is too short"),
            Self::TooLong => write!(f, "alias is too long"),
            Self::InvalidCharacters => write!(f, "alias contains invalid characters"),
        }
    }
}

fn validate_alias(alias: &str) -> Result<(), AliasError> {
    const MIN_ALIAS_LENGTH: usize = 6;
    const MAX_ALIAS_LENGTH: usize = 20;
    if alias.len() < MIN_ALIAS_LENGTH {
        return Err(AliasError::TooShort);
    }
    if alias.len() > MAX_ALIAS_LENGTH {
        return Err(AliasError::TooLong);
    }
    if alias.contains(|c: char| !c.is_alphanumeric()) {
        return Err(AliasError::InvalidCharacters);
    }
    Ok(())
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

    #[test]
    fn allowed_aliases() {
        let aliases = ["abcdef", "abcde1234567890", "abcde12345678901234"];
        for alias in aliases {
            let result = validate_alias(alias);
            assert!(
                result.is_ok(),
                "{} should be allowed, instead: {:?}",
                alias,
                result
            );
        }
    }

    // a simple test that ensures a correct alias passes validate_alias
}

#[test]
fn disallowed_aliases() {
    let aliases = [
        "",
        "a",
        "abcde",
        "abcde12345678901234567890",
        "abcde1234567890!@#$%",
        "ab-cde",
        "ab_cde",
        "ab.cde",
        "ab&cde",
        "ab cde",
        "ab/cde",
    ];
    for alias in aliases {
        let result = validate_alias(alias);
        assert!(
            result.is_err(),
            "{} should not be allowed, instead: {:?}",
            alias,
            result
        );
    }

    // a list of aliases that should be disallowed by validate_alias
}
