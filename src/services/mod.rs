use anyhow::anyhow;
use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use rand_core::OsRng;
use thiserror::Error;

mod links;
mod users;

pub use links::*;
pub use users::{authenticate_user, create_user};

/// Hash a password with argon2, returning the hash string.
pub fn hash_password(password: &str, hasher: &Argon2<'_>) -> Result<String, ServiceError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = hasher
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| anyhow!("failed to hash password"))?;
    Ok(hash.to_string())
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("authentication failed")]
    AuthError,
    #[error("database error {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error(transparent)]
    LinkServiceError(#[from] LinkServiceError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
