use std::sync::Arc;

use base64::Engine;
use dashmap::DashMap;
use rand_core::{OsRng, RngCore};

use crate::domain::{User, UserId};

pub enum SessionError {
    NotExists,
    Expired,
}

pub struct Sessions {
    sid_to_uid: Arc<DashMap<String, UserId>>,
    uid_to_user: Arc<DashMap<UserId, User>>,
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
            sid_to_uid: Arc::new(DashMap::new()),
            uid_to_user: Arc::new(DashMap::new()),
        }
    }

    pub fn new_session(&self, user: User) -> SessionId {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD as Base64;

        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);

        let session_id = Base64.encode(bytes);
        self.sid_to_uid.insert(session_id.clone(), user.id());
        self.uid_to_user.insert(user.id(), user);

        SessionId(session_id)
    }

    pub fn get_user(&self, user_id: UserId) -> Result<User, SessionError> {
        if let Some(entry) = self.uid_to_user.get(&user_id) {
            let user = entry.value();
            Ok(user.clone())
        } else {
            Err(SessionError::NotExists)
        }
    }

    pub fn get_user_id(&self, session_id: &str) -> Result<UserId, SessionError> {
        if let Some(entry) = self.sid_to_uid.get(session_id) {
            let user_id = entry.value();
            Ok(*user_id)
        } else {
            Err(SessionError::NotExists)
        }
    }
}
