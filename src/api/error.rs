use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use const_format::formatcp;
use serde::{Deserialize, Serialize};

use crate::{
    api::{session::SessionError, token::TokenError},
    domain::{
        LinkAlias, LinkAliasError, LinkPassword, LinkPasswordError, UrlParseError, UserName,
        UserPassword, UserPasswordError, UsernameError,
    },
    services::{CollectionError, LinkError, ServiceError},
};

#[derive(Debug)]
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

    pub fn unauthorized() -> Self {
        Self {
            status_code: StatusCode::UNAUTHORIZED,
            reason: "Unauthorized",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status_code, Json(ApiErrorBody(self.reason))).into_response()
    }
}

impl From<TokenError> for ApiError {
    fn from(_: TokenError) -> Self {
        Self::unauthorized()
    }
}

impl From<ServiceError> for ApiError {
    fn from(error: ServiceError) -> Self {
        match error {
            ServiceError::LinkError(err) => err.into(),
            ServiceError::CollectionError(err) => err.into(),
        }
    }
}

impl From<LinkError> for ApiError {
    fn from(error: LinkError) -> Self {
        match error {
            LinkError::AlreadyExists => {
                Self::public(StatusCode::CONFLICT, "This alias already exists")
            }
            LinkError::NotFound => Self::not_found(),
            _ => Self::internal(),
        }
    }
}

impl From<SessionError> for ApiError {
    fn from(_: SessionError) -> Self {
        Self::unauthorized()
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

impl From<LinkAliasError> for ApiError {
    fn from(error: LinkAliasError) -> Self {
        match error {
            LinkAliasError::InvalidChars => {
                Self::public(StatusCode::BAD_REQUEST, "Alias contains invalid characters")
            }
            LinkAliasError::TooShort => Self::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Alias must be at least {} characters",
                    LinkAlias::MIN_ALIAS_LENGTH
                ),
            ),
            LinkAliasError::TooLong => Self::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Alias cannot contain more than {} characters",
                    LinkAlias::MAX_ALIAS_LENGTH
                ),
            ),
        }
    }
}

impl From<LinkPasswordError> for ApiError {
    fn from(error: LinkPasswordError) -> Self {
        match error {
            LinkPasswordError::InvalidChars => ApiError::public(
                StatusCode::BAD_REQUEST,
                "Link password contains invalid characters",
            ),
            LinkPasswordError::TooShort => ApiError::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Link password must be at least {} characters",
                    LinkPassword::MIN_LINK_PASSWORD_LENGTH
                ),
            ),
            LinkPasswordError::TooLong => ApiError::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Link password cannot be longer than {} characters",
                    LinkPassword::MAX_LINK_PASSWORD_LENGTH
                ),
            ),
        }
    }
}

impl From<UsernameError> for ApiError {
    fn from(error: UsernameError) -> Self {
        match error {
            UsernameError::InvalidChars => ApiError::public(
                StatusCode::BAD_REQUEST,
                "Username contains invalid characters",
            ),
            UsernameError::TooShort => ApiError::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Username must be at least {} characters",
                    UserName::MIN_USERNAME_LENGTH
                ),
            ),
            UsernameError::TooLong => ApiError::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Username cannot be longer than {} characters",
                    UserName::MAX_USERNAME_LENGTH
                ),
            ),
        }
    }
}

impl From<UserPasswordError> for ApiError {
    fn from(error: UserPasswordError) -> Self {
        match error {
            UserPasswordError::InvalidChars => ApiError::public(
                StatusCode::BAD_REQUEST,
                "Password contains invalid characters",
            ),
            UserPasswordError::TooShort => ApiError::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Password must contain at least {} characters",
                    UserPassword::MIN_PASSWORD_LENGTH
                ),
            ),
            UserPasswordError::TooLong => ApiError::public(
                StatusCode::BAD_REQUEST,
                formatcp!(
                    "Password cannot be longer than {} characters",
                    UserPassword::MAX_PASSWORD_LENGTH
                ),
            ),
        }
    }
}
