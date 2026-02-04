use thiserror::Error;

mod links;
mod users;

pub use links::{
    create_link, create_link_with_alias, query_links_by_user_id, query_url_by_alias,
    remove_user_link,
};
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
