mod ops;
mod query;

pub use ops::*;
pub use query::*;
use thiserror::Error;

// TODO: settings
pub const MAX_COLLECTION_ITEMS: i32 = 10;

#[derive(Debug, Error)]
pub enum CollectionError {
    #[error("reached url limit of a collection")]
    LimitReached,
    #[error("cannot insert empty collection")]
    Empty,
}

#[derive(Debug, Error)]
pub enum LinkError {
    #[error("alias already exists")]
    AlreadyExists,
    #[error("alias not found")]
    NotFound,
    #[error("alphabet was exhausted")]
    GeneratorError,
}
