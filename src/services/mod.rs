use thiserror::Error;

mod links;
mod users;

pub use links::*;
pub use users::{authenticate_user, create_user};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("authentication failed")]
    AuthError,
    #[error("database error {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
