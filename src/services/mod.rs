use thiserror::Error;

mod accounts;
mod links;

pub use accounts::{create_user_account, verify_user_password};
pub use links::{create_link, create_link_with_alias, query_link_by_alias, query_links_by_user};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("database error {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
