use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
};
use cookie::Cookie;

use crate::app::AppState;

pub struct User {
    pub user_id: i64,
}

impl FromRequestParts<AppState> for User {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let session_id =
            parse_session_id(&parts.headers).ok_or((StatusCode::UNAUTHORIZED, "Not logged in"))?;

        let user_id = state
            .sessions
            .get_user_id(&session_id)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Not logged in"))?;

        Ok(User { user_id })
    }
}

pub struct MaybeUser(pub Option<User>);

impl FromRequestParts<AppState> for MaybeUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let sid = parse_session_id(&parts.headers);

        if let Some(session_id) = sid {
            let user_id = state
                .sessions
                .get_user_id(&session_id)
                .map_err(|_| (StatusCode::UNAUTHORIZED, "Not logged in"))?;

            return Ok(MaybeUser(Some(User { user_id })));
        }

        Ok(MaybeUser(None))
    }
}

fn parse_session_id(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;

    for part in raw.split(';') {
        let c = Cookie::parse(part.trim()).ok()?;

        if c.name() == "sid" {
            return Some(c.value().to_string());
        }
    }

    None
}
