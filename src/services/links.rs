use anyhow::Context;
use argon2::Argon2;
use serde::Serialize;
use sqids::Sqids;
use sqlx::PgPool;
use thiserror::Error;

use crate::{
    app::CachedLink,
    domain::{Alias, Url, UserId},
    services::ServiceError,
};

use super::hash_password;

#[derive(Debug, Error)]
pub enum LinkServiceError {
    #[error("alias already exists")]
    AlreadyExists,
    #[error("alias not found")]
    NotFound,
}

/// Create a new link for the provided URL
#[tracing::instrument(
    name = "services::create_link",
    skip(generator, pool, password, hasher)
)]
pub async fn create_link(
    url: &Url,
    generator: &Sqids,
    pool: &PgPool,
    user_id: Option<UserId>,
    password: Option<&str>,
    hasher: &Argon2<'_>,
) -> Result<String, ServiceError> {
    let password_hash = password
        .filter(|p| !p.is_empty())
        .map(|p| hash_password(p, hasher))
        .transpose()?;
    let password_hash_ref = password_hash.as_deref();

    let mut tx = pool.begin().await.map_err(ServiceError::DatabaseError)?;
    // Insert the url into database to get a unique id
    let rec = sqlx::query!(
        r#"
        INSERT INTO links_main (url, user_id, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        url.as_str(),
        user_id,
        password_hash_ref,
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
#[tracing::instrument(
    name = "services::create_link_with_alias",
    skip(pool, password, hasher)
)]
pub async fn create_link_with_alias(
    url: &Url,
    alias: &Alias,
    pool: &PgPool,
    user_id: Option<UserId>,
    password: Option<&str>,
    hasher: &Argon2<'_>,
) -> Result<String, ServiceError> {
    let password_hash = password
        .filter(|p| !p.is_empty())
        .map(|p| hash_password(p, hasher))
        .transpose()?;
    let password_hash_ref = password_hash.as_deref();

    let rec_opt = sqlx::query!(
        r#"
        INSERT INTO links_main (alias, url, user_id, password_hash)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (alias) DO NOTHING
        RETURNING alias
        "#,
        alias.as_str(),
        url.as_str(),
        user_id,
        password_hash_ref,
    )
    .fetch_optional(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    match rec_opt {
        Some(rec) => Ok(rec.alias.unwrap()),
        None => Err(LinkServiceError::AlreadyExists.into()),
    }
}

/// Query url from database
///
/// Returns Ok(None) if the alias does not exist
#[tracing::instrument(name = "services::query_url_by_alias", skip(pool))]
pub async fn query_url_by_alias(
    alias: &Alias,
    pool: &PgPool,
) -> Result<Option<CachedLink>, ServiceError> {
    let rec_opt = sqlx::query!(
        r#"SELECT id, url, last_seen, password_hash FROM links_main WHERE alias = $1"#,
        alias.as_str()
    )
    .fetch_optional(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    rec_opt
        .map(|rec| {
            Ok(CachedLink {
                id: rec.id,
                url: rec.url,
                last_seen: rec.last_seen,
                password_hash: rec.password_hash,
            })
        })
        .transpose()
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkItem {
    pub alias: String,
    pub url: String,
}

/// List user's links
#[tracing::instrument(name = "services::query_links_by_user_id", skip(pool))]
pub async fn query_links_by_user_id(
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Vec<LinkItem>, ServiceError> {
    let rec_vec = sqlx::query!(
        r#"
        SELECT alias, url
        FROM links_main
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    let links = rec_vec
        .into_iter()
        .map(|rec| LinkItem {
            alias: rec.alias.unwrap_or_default(),
            url: rec.url,
        })
        .collect();

    Ok(links)
}

/// Remove user's link
#[tracing::instrument(name = "services::remove_user_link", skip(pool))]
pub async fn remove_user_link(
    user_id: &UserId,
    alias: &Alias,
    pool: &PgPool,
) -> Result<(), ServiceError> {
    sqlx::query!(
        r#"
        DELETE FROM links_main
        WHERE user_id = $1
          AND alias = $2
        "#,
        user_id,
        alias.as_str()
    )
    .execute(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    Ok(())
}

#[tracing::instrument(name = "app::recently_added_links", skip(pool))]
pub async fn recently_added_links(limit: i64, pool: &PgPool) -> Result<Vec<String>, ServiceError> {
    let recs = sqlx::query!(
        r#"
        SELECT url
        FROM links_main
        ORDER BY id DESC
        LIMIT $1
        "#,
        limit
    )
    .fetch_all(pool)
    .await
    .context("DB select recent links query failed")?;

    Ok(recs.into_iter().map(|rec| rec.url).collect())
}
