use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use url::{Host, Url};

use crate::core::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/shorten", post(handle_shorten_request))
        .route("/{alias}", get(handle_redirect_request))
        .with_state(state)
}

#[derive(Deserialize)]
struct ShortenRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct ShortenResponse {
    pub alias: String,
}

async fn handle_shorten_request(
    State(AppState { app }): State<AppState>,
    Json(ShortenRequest { url }): Json<ShortenRequest>,
) -> impl IntoResponse {
    let result = app.create_alias(&url).await;

    if let Ok(alias) = result {
        (StatusCode::CREATED, Json(ShortenResponse { alias })).into_response()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR).into_response()
    }
}

async fn handle_redirect_request(
    State(AppState { app }): State<AppState>,
    Path(alias): Path<String>,
) -> impl IntoResponse {
    let result = app.get_url(&alias).await;

    if let Ok(url) = result {
        Redirect::permanent(&url).into_response()
    } else {
        (StatusCode::NOT_FOUND).into_response()
    }
}

#[derive(Debug, PartialEq, Eq)]
/// encountered an Error while validating a url
/// `ParseErr` happens when the url is invalid
/// the others when we don't accept it for other reasons
pub enum UrlError {
    HasPasswordOrUsername,
    WrongScheme,
    InvalidHost,
    NoHost,
    ParseErr(url::ParseError),
}

impl Display for UrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HasPasswordOrUsername => write!(f, "the host can't have passwords or usernames"),
            Self::WrongScheme => write!(f, "the url must be of either http or htttps scheme"),
            Self::InvalidHost => write!(f, "the url can't have localhost as host"),
            Self::NoHost => write!(f, "the supplied url does not have a host"),
            Self::ParseErr(e) => write!(f, "failed parsing the url: {e}"),
        }
    }
}

impl std::error::Error for UrlError {}

/// validate a url, returning `Ok(())` on success`
pub fn validate_url(url: &str) -> Result<(), UrlError> {
    let url = Url::parse(url).map_err(UrlError::ParseErr)?;

    let host = url.host().ok_or(UrlError::NoHost)?;

    let has_uname_or_pword = !url.username().is_empty() || url.password().is_some();

    if !["http", "https"].contains(&url.scheme()) {
        Err(UrlError::WrongScheme)
    } else if !validate_host(host) {
        Err(UrlError::InvalidHost)
    } else if has_uname_or_pword {
        Err(UrlError::HasPasswordOrUsername)
    } else {
        Ok(())
    }
}

fn validate_host(host: Host<&str>) -> bool {
    match host {
        Host::Domain(name) => {
            let name = name.trim_end_matches('.');
            // check that the top level domain isn't localhost
            let is_localhost = name.ends_with("localhost");
            // and check that we have a domain under the top level
            let has_sub_domain = name.contains('.');

            !is_localhost && has_sub_domain
        }
        // disallow special addresses stable std recognizes
        Host::Ipv4(ipv4_addr) => {
            !ipv4_addr.is_loopback()
                && !ipv4_addr.is_unspecified()
                && !ipv4_addr.is_link_local()
                && !ipv4_addr.is_private()
                && !ipv4_addr.is_multicast()
        }
        Host::Ipv6(ipv6_addr) => {
            !ipv6_addr.is_loopback()
                && !ipv6_addr.is_unspecified()
                && !ipv6_addr.is_unique_local()
                && !ipv6_addr.is_unicast_link_local()
                && !ipv6_addr.is_multicast()
        }
    }
}

#[cfg(test)]
mod test {
    use super::{UrlError, validate_url};

    #[test]
    fn invalid_schemes() {
        let wrong_scheme_urls = [
            "ftp://user:password@hostname.com/txt.txt",
            "ssh://login@server.com:12345/repository.git",
        ];

        let wrong_scheme_results = wrong_scheme_urls.map(validate_url);

        for res in wrong_scheme_results {
            assert_eq!(res, Err(UrlError::WrongScheme))
        }

        let right_scheme_urls = [
            "http://user:password@hostname.com/txt.txt",
            "https:///home/user/.bashrc",
            "http://login@server.com:12345/repository.git",
            "https:/run/foo.socket",
        ];

        let right_scheme_results = right_scheme_urls.map(validate_url);

        for res in right_scheme_results {
            assert_ne!(res, Err(UrlError::WrongScheme))
        }
    }

    #[test]
    fn invalid_hosts() {
        let wrong_host_urls = [
            "http://localhost/txt.txt",
            "https://127.0.0.1/txt.txt",
            "http://localhost.",
        ];

        let wrong_host_results = wrong_host_urls.map(validate_url);

        assert!(
            wrong_host_results
                .into_iter()
                .all(|res| res == Err(UrlError::InvalidHost))
        );

        let no_host_url = validate_url("http:///example");
        assert_eq!(no_host_url, Err(UrlError::InvalidHost));
    }

    #[test]
    fn rejects_authority() {
        let invalid_urls = ["http://user:password@hostname.com/txt.txt"];

        let invalid_results = invalid_urls.map(validate_url);

        for res in invalid_results {
            assert_eq!(res, Err(UrlError::HasPasswordOrUsername))
        }
    }

    #[test]
    fn accepts_good_url() {
        let res = validate_url("https://example.com");
        assert_eq!(res, Ok(()));
    }
}
