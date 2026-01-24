use std::sync::Arc;

use base64::Engine;
use dashmap::DashMap;
use rand_core::{OsRng, RngCore};

pub struct SessionData {
    pub user_id: i64,
}

pub enum SessionError {
    NotExists,
    Expired,
}

pub struct Sessions {
    inner: Arc<DashMap<String, SessionData>>,
}

pub struct SessionId(pub String);

impl SessionId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Sessions {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    pub fn new_session(&self, session: SessionData) -> SessionId {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD as Base64;

        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);

        let session_id = Base64.encode(bytes);
        self.inner.insert(session_id.clone(), session);

        SessionId(session_id)
    }

    pub fn get_user_id(&self, session_id: &str) -> Result<i64, SessionError> {
        if let Some(entry) = self.inner.get(session_id) {
            let session_data = entry.value();

            Ok(session_data.user_id)
        } else {
            Err(SessionError::NotExists)
        }
    }
}
