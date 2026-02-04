use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use cookie::Cookie;

use crate::{app::AppState, domain::UserId};

pub struct RequireUser(pub UserId);

impl FromRequestParts<AppState> for RequireUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let sid = parse_session_id(&parts.headers).ok_or_else(|| {
            let mut res = (StatusCode::UNAUTHORIZED, "Not logged in").into_response();
            res.headers_mut().append(
                header::SET_COOKIE,
                HeaderValue::from_static("sid=; Max-Age=0; Path=/; HttpOnly; SameSite=Lax"),
            );
            res
        })?;

        let user_id = state.sessions.get_user_id(&sid).map_err(|_| {
            let mut res = (StatusCode::UNAUTHORIZED, "Not logged in").into_response();
            res.headers_mut().append(
                header::SET_COOKIE,
                HeaderValue::from_static("sid=; Max-Age=0; Path=/; HttpOnly; SameSite=Lax"),
            );
            res
        })?;

        Ok(RequireUser(user_id))
    }
}

#[derive(Clone, Copy)]
pub struct ClearSid;

pub struct MaybeUser(pub Option<UserId>);

impl FromRequestParts<AppState> for MaybeUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(sid) = parse_session_id(&parts.headers) else {
            return Ok(MaybeUser(None));
        };

        match state.sessions.get_user_id(&sid) {
            Ok(user_id) => Ok(MaybeUser(Some(user_id))),
            Err(_) => {
                parts.extensions.insert(ClearSid);
                Ok(MaybeUser(None))
            }
        }
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
