use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, StatusCode, header, request::Parts},
};
use cookie::Cookie;

use crate::{app::AppState, domain::User};

pub struct RequireUser(pub User);

impl FromRequestParts<AppState> for RequireUser {
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

        Ok(RequireUser(User::new(user_id)))
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

            return Ok(MaybeUser(Some(User::new(user_id))));
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
