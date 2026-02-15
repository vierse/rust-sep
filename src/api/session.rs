use std::{borrow::Borrow, sync::Arc};

use axum::{
    body::Body,
    extract::State,
    http::Request,
    http::{HeaderMap, HeaderValue, header},
    middleware::Next,
    response::Response,
};
use base64::Engine;
use cookie::Cookie;
use dashmap::DashMap;
use rand_core::{OsRng, RngCore};

use crate::{
    app::AppState,
    domain::{User, UserId},
};

pub enum SessionError {
    NotExists,
    Expired,
}

pub struct SessionData {
    pub user_id: UserId,
    pub username: String,
}

#[derive(Clone)]
pub struct Sessions {
    inner: Arc<DashMap<SessionId, Arc<SessionData>>>,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct SessionId(String);

impl SessionId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for SessionId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Sessions {
    pub fn new_session(&self, user: &User) -> SessionId {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD as Base64;

        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let session_id = SessionId(Base64.encode(bytes));

        self.inner
            .insert(session_id.clone(), Arc::new(SessionData::new(user)));

        session_id
    }

    pub fn get_session_data(
        &self,
        session_id: &SessionId,
    ) -> Result<Arc<SessionData>, SessionError> {
        if let Some(session) = self.inner.get(session_id) {
            Ok(session.value().clone())
        } else {
            Err(SessionError::NotExists)
        }
    }

    pub fn close_session(&self, session_id: &SessionId) -> bool {
        self.inner.remove(session_id).is_some()
    }

    fn is_active(&self, session_id: &str) -> bool {
        self.inner.contains_key(session_id)
    }
}

impl Default for Sessions {
    fn default() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }
}

impl SessionData {
    fn new(user: &User) -> Self {
        Self {
            user_id: user.id(),
            username: user.name().to_string(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct ClearSid;

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

pub async fn session_manager_mw(
    State(app): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let mut clear = false;

    if let Some(sid) = parse_session_id(req.headers()) {
        if app.sessions.is_active(&sid) {
            req.extensions_mut().insert(SessionId(sid));
        } else {
            clear = true;
        }
    }

    let mut res = next.run(req).await;

    if res.extensions().get::<ClearSid>().is_some() {
        clear = true;
    }

    if clear {
        res.headers_mut().append(
            header::SET_COOKIE,
            HeaderValue::from_static("sid=; Max-Age=0; Path=/; HttpOnly; SameSite=Lax"),
        );
    }

    res
}
