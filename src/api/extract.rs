use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use time::OffsetDateTime;

use crate::{
    api::{session::SessionId, token::Token},
    app::AppState,
};

pub struct RequireUser(pub SessionId);

impl FromRequestParts<AppState> for RequireUser {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _: &AppState) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<SessionId>()
            .cloned()
            .map(RequireUser)
            .ok_or_else(|| StatusCode::UNAUTHORIZED.into_response())
    }
}

pub struct MaybeUser(pub Option<SessionId>);

impl FromRequestParts<AppState> for MaybeUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &AppState) -> Result<Self, Self::Rejection> {
        Ok(MaybeUser(
            parts.extensions.get::<SessionId>().cloned().or(None),
        ))
    }
}

pub struct MaybeToken<T>(pub Option<T>);

impl<T: Token> FromRequestParts<AppState> for MaybeToken<T> {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);

        let Some(cookie) = jar.get(T::TYPE) else {
            return Ok(Self(None));
        };

        let now_s = OffsetDateTime::now_utc().unix_timestamp();

        match state.signer.verify_token(cookie.value(), now_s) {
            Ok(token) => Ok(Self(Some(token))),
            Err(_) => Ok(Self(None)),
        }
    }
}
