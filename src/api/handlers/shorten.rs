use anyhow::{Result, bail};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::app::AppState;

#[derive(Deserialize)]
pub struct ShortenRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct ShortenResponse {
    pub alias: String,
}

pub async fn shorten(
    State(app): State<AppState>,
    Json(ShortenRequest { url }): Json<ShortenRequest>,
) -> impl IntoResponse {
    if validate_url(&url).is_err() {
        return (StatusCode::BAD_REQUEST).into_response();
    }

    let result = app.shorten_url(&url).await;
    if let Ok(alias) = result {
        (StatusCode::CREATED, Json(ShortenResponse { alias })).into_response()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR).into_response()
    }
}

fn validate_url(url: &str) -> Result<()> {
    let url = Url::parse(url)?;

    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        bail!("disallowed URL scheme");
    }

    if !url.username().is_empty() || url.password().is_some() {
        bail!("userinfo not allowed");
    }

    let domain = url.domain().unwrap_or("");
    if domain.is_empty() {
        bail!("missing domain");
    }
    if domain
        .trim_end_matches(".")
        .to_ascii_lowercase()
        .eq_ignore_ascii_case("localhost")
        || domain.ends_with(".local")
        || !domain.contains('.')
    {
        bail!("disallowed host");
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
