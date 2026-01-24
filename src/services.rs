use anyhow::{Context, anyhow};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand_core::OsRng;
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

#[tracing::instrument(name = "services::create_user_account", skip_all)]
pub async fn create_user_account(
    username: &str,
    password: &str,
    hasher: &Argon2<'_>,
    pool: &PgPool,
) -> Result<i64, ServiceError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = hasher
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| anyhow!("failed to hash"))?;

    let rec = sqlx::query!(
        r#"
        INSERT INTO users_main (username, password_hash)
        VALUES ($1, $2)
        ON CONFLICT (username) DO NOTHING
        RETURNING id
        "#,
        username,
        hash.to_string()
    )
    .fetch_optional(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    Ok(rec.unwrap().id)
}

pub async fn verify_user_password(
    username: &str,
    password: &str,
    hasher: &Argon2<'_>,
    pool: &PgPool,
) -> Result<Option<i64>, ServiceError> {
    let rec = sqlx::query!(
        r#"
        SELECT id, password_hash
        FROM users_main
        WHERE username = $1
        "#,
        username
    )
    .fetch_optional(pool)
    .await
    .context("failed to fetch user password hash")?;

    let Some(rec) = rec else {
        return Ok(None);
    };

    let parsed_hash = PasswordHash::new(&rec.password_hash)
        .map_err(|e| anyhow::anyhow!("invalid password hash: {e}"))?;

    if hasher
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
    {
        Ok(Some(rec.id))
    } else {
        Ok(None)
    }
}
