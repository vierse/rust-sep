use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use const_format::formatcp;
use serde::{Deserialize, Serialize};

use crate::{
    api::session::SessionError,
    domain::{Alias, AliasParseError, CredentialsError, UrlParseError, UserName, UserPassword},
    services::{LinkServiceError, ServiceError},
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

    pub fn bad_request() -> Self {
        Self {
            status_code: StatusCode::BAD_REQUEST,
            reason: "Invalid request",
        }
    }

    pub fn internal() -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            reason: "Internal server error",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status_code, Json(ApiErrorBody(self.reason))).into_response()
    }
}

impl From<ServiceError> for ApiError {
    fn from(error: ServiceError) -> Self {
        match error {
            ServiceError::LinkServiceError(err) => err.into(),
            _ => {
                // propagated internal errors will be logged here
                tracing::error!(error = %error, "internal error: ");
                Self::internal()
            }
        }
    }
}

impl From<LinkServiceError> for ApiError {
    fn from(error: LinkServiceError) -> Self {
        match error {
            LinkServiceError::AlreadyExists => {
                Self::public(StatusCode::CONFLICT, "This alias already exists")
            }
            LinkServiceError::NotFound => Self::not_found(),
        }
    }
}

impl From<SessionError> for ApiError {
    fn from(_error: SessionError) -> Self {
        // TODO: expired session errors might be relevant to user
        Self::internal()
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
            AliasParseError::TooShort => Self::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Chosen link must be at least {} characters",
                    Alias::MIN_ALIAS_LENGTH
                ),
            ),
            AliasParseError::TooLong => Self::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Chosen link cannot contain more than {} characters",
                    Alias::MAX_ALIAS_LENGTH
                ),
            ),
            AliasParseError::InvalidCharacters => Self::public(
                StatusCode::BAD_REQUEST,
                "Chosen link contains invalid characters",
            ),
        }
    }
}

impl From<CredentialsError> for ApiError {
    fn from(error: CredentialsError) -> Self {
        match error {
            CredentialsError::UsernameInvalidChars => ApiError::public(
                StatusCode::BAD_REQUEST,
                "Username contains invalid characters",
            ),
            CredentialsError::UsernameTooShort => ApiError::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Username must be at least {} characters",
                    UserName::MIN_USERNAME_LENGTH
                ),
            ),
            CredentialsError::UsernameTooLong => ApiError::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Username cannot be longer than {} characters",
                    UserName::MAX_USERNAME_LENGTH
                ),
            ),
            CredentialsError::PasswordInvalidChars => ApiError::public(
                StatusCode::BAD_REQUEST,
                "Password contains invalid characters",
            ),
            CredentialsError::PasswordTooShort => ApiError::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Password must contain at least {} characters",
                    UserPassword::MIN_PASSWORD_LENGTH
                ),
            ),
            CredentialsError::PasswordTooLong => ApiError::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Password cannot be longer than {} characters",
                    UserPassword::MAX_PASSWORD_LENGTH
                ),
            ),
        }
    }
}
