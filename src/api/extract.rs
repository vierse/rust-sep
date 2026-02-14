use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};

use crate::{api::session::SessionId, app::AppState};

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
