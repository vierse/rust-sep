use anyhow::Context;
use sqids::Sqids;
use sqlx::PgPool;
use thiserror::Error;

use crate::{app::CachedLink, domain::Url};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("database error {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Create a new link for the provided URL
#[tracing::instrument(name = "services::create_link", skip(generator, pool))]
pub async fn create_link(
    url: &str,
    generator: &Sqids,
    pool: &PgPool,
) -> Result<String, ServiceError> {
    let mut tx = pool.begin().await.map_err(ServiceError::DatabaseError)?;

    // Insert the url into database to get a unique id
    let rec = sqlx::query!(
        r#"
        INSERT INTO links_main (url)
        VALUES ($1)
        RETURNING id
        "#,
        url,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ServiceError::DatabaseError)?;

    let id = rec.id as u64;

    let alias = generator
        .encode(&[id])
        .context("Sqids alphabet was exhausted")
        .map_err(ServiceError::Other)?;

    // Update the record with generated alias
    let updated = sqlx::query!(
        r#"
        UPDATE links_main
        SET alias = $1
        WHERE id = $2
        RETURNING alias
        "#,
        alias,
        rec.id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ServiceError::DatabaseError)?;

    tx.commit().await.map_err(ServiceError::DatabaseError)?;

    let alias = updated
        .alias
        .context("Updated record contained no alias")
        .map_err(ServiceError::Other)?;

    Ok(alias)
}

/// Create a link with user-defined alias for the provided URL
///
/// Returns Ok(false) if the alias is already taken
#[tracing::instrument(name = "services::create_link_with_alias", skip(pool))]
pub async fn create_link_with_alias(
    url: &str,
    alias: &str,
    pool: &PgPool,
) -> Result<bool, ServiceError> {
    let rec = sqlx::query!(
        r#"
        INSERT INTO links_main (alias, url)
        VALUES ($1, $2)
        ON CONFLICT (alias) DO NOTHING
        RETURNING id
        "#,
        alias,
        url,
    )
    .fetch_optional(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    Ok(rec.is_some())
}

/// Query link from database
///
/// Returns Ok(None) if the alias does not exist
#[tracing::instrument(name = "services::query_link_by_alias", skip(pool))]
pub async fn query_link_by_alias(
    alias: &str,
    pool: &PgPool,
) -> Result<Option<CachedLink>, ServiceError> {
    let rec_opt = sqlx::query!(r#"SELECT id, url FROM links_main WHERE alias = $1"#, alias)
        .fetch_optional(pool)
        .await
        .map_err(ServiceError::DatabaseError)?;

    rec_opt
        .map(|rec| {
            let url = Url::parse(&rec.url)
                .with_context(|| format!("Failed to validate url from {alias}"))
                .map_err(ServiceError::Other)?
                .into_string();

            Ok(CachedLink { id: rec.id, url })
        })
        .transpose()
}
