use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    api::session::SessionError,
    domain::{AliasParseError, UrlParseError},
};

pub struct ApiError {
    status_code: StatusCode,
    reason: &'static str,
}

#[derive(Deserialize, Serialize)]
struct ApiErrorBody(&'static str);

impl ApiError {
    pub fn public(status_code: StatusCode, reason: &'static str) -> Self {
        Self {
            status_code,
            reason,
        }
    }

    pub fn not_found() -> Self {
        Self {
            status_code: StatusCode::NOT_FOUND,
            reason: "Not found",
        }
    }

    pub fn internal() -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            reason: "Internal server error",
        }
    }
}

impl From<SessionError> for ApiError {
    fn from(error: SessionError) -> Self {
        match error {
            _ => Self::internal(),
        }
    }
}

impl From<UrlParseError> for ApiError {
    fn from(error: UrlParseError) -> Self {
        match error {
            UrlParseError::ContainsUserinfo => {
                Self::public(StatusCode::BAD_REQUEST, "URL contains credentials")
            }
            UrlParseError::WrongScheme(_) => {
                Self::public(StatusCode::BAD_REQUEST, "This URL scheme is not supported")
            }
            UrlParseError::BlockedHost(_) => {
                Self::public(StatusCode::BAD_REQUEST, "This host is not allowed")
            }
            UrlParseError::EmptyHost => {
                Self::public(StatusCode::BAD_REQUEST, "This URL is incomplete")
            }
            UrlParseError::Invalid(_) => {
                Self::public(StatusCode::BAD_REQUEST, "This URL is invalid")
            }
        }
    }
}

impl From<AliasParseError> for ApiError {
    fn from(error: AliasParseError) -> Self {
        match error {
            AliasParseError::TooShort => {
                Self::public(StatusCode::BAD_REQUEST, "Chosen link is too short")
            }
            AliasParseError::TooLong => {
                Self::public(StatusCode::BAD_REQUEST, "Chosen link is too long")
            }
            AliasParseError::InvalidCharacters => Self::public(
                StatusCode::BAD_REQUEST,
                "Chosen link contains invalid characters",
            ),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status_code, Json(ApiErrorBody(self.reason))).into_response()
    }
}
