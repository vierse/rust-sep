use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use url::Url;

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
    WrongScheme,
    LocalHost,
    NoHost,
    ParseErr(url::ParseError),
}

impl Display for UrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongScheme => write!(f, "the url must be of either http or htttps scheme"),
            Self::LocalHost => write!(f, "the url can't have localhost as host"),
            Self::NoHost => write!(f, "the supplied url does not have a host"),
            Self::ParseErr(e) => write!(f, "failed parsing the url: {e}"),
        }
    }
}

impl std::error::Error for UrlError {}

/// validate a url, returning `Ok(())` on success`
pub fn validate_url(url: &str) -> Result<(), UrlError> {
    Url::parse(url).map_err(UrlError::ParseErr).and_then(|url| {
        if url.scheme() != "http" && url.scheme() != "https" {
            Err(UrlError::WrongScheme)
        } else if url
            .host_str()
            .is_some_and(|host| host == "localhost" || host == "127.0.0.1")
        {
            Err(UrlError::LocalHost)
        } else if url.host_str().is_none() {
            Err(UrlError::NoHost)
        } else {
            Ok(())
        }
    })
}
