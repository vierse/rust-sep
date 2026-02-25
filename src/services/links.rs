use anyhow::Context;
use argon2::Argon2;
use serde::Serialize;
use sqids::Sqids;
use sqlx::PgPool;
use thiserror::Error;

use crate::{
    app::{CachedLink, CachedLinkType},
    domain::{LinkAlias, Url, UserId},
    services::ServiceError,
};

use super::hash_password;

#[derive(Debug, Error)]
pub enum LinkError {
    #[error("alias already exists")]
    AlreadyExists,
    #[error("alias not found")]
    NotFound,
    #[error("alphabet was exhausted")]
    GeneratorError,
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
) -> Result<(i64, String), ServiceError> {
    let password_hash = password
        .filter(|p| !p.is_empty())
        .map(|p| hash_password(p, hasher))
        .transpose()?;
    let password_hash_ref = password_hash.as_deref();

    let mut tx = pool.begin().await?;

    // reserve unique ID
    let seq = sqlx::query!(
        r#"
        SELECT nextval(pg_get_serial_sequence('links', 'id')) AS "id!"
        "#
    )
    .fetch_one(&mut *tx)
    .await?;

    let id = seq.id;

    let alias = generator
        .encode(&[id as u64])
        .context("Sqids alphabet was exhausted")?;

    let rec = sqlx::query!(
        r#"
        INSERT INTO links (id, alias, target_url, user_id, password_hash)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, alias
        "#,
        id,
        alias,
        url.as_str(),
        user_id,
        password_hash_ref,
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((rec.id, rec.alias))
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
    alias: &LinkAlias,
    pool: &PgPool,
    user_id: Option<UserId>,
    password: Option<&str>,
    hasher: &Argon2<'_>,
) -> Result<(i64, String), ServiceError> {
    let password_hash = password
        .filter(|p| !p.is_empty())
        .map(|p| hash_password(p, hasher))
        .transpose()?;
    let password_hash_ref = password_hash.as_deref();

    let rec_opt = sqlx::query!(
        r#"
        INSERT INTO links (alias, target_url, user_id, password_hash)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (alias) DO NOTHING
        RETURNING alias, id
        "#,
        alias.as_str(),
        url.as_str(),
        user_id,
        password_hash_ref,
    )
    .fetch_optional(pool)
    .await?;

    rec_opt
        .map(|rec| (rec.id, rec.alias))
        .ok_or(LinkError::AlreadyExists.into())
}

/// Query url from database
///
/// Returns Ok(None) if the alias does not exist
#[tracing::instrument(name = "services::query_url_by_alias", skip(pool))]
pub async fn query_url_by_alias(
    alias: &LinkAlias,
    pool: &PgPool,
) -> Result<Option<CachedLink>, ServiceError> {
    let rec_opt = sqlx::query!(
        r#"SELECT id, kind, target_url, last_seen, password_hash FROM links WHERE alias = $1"#,
        alias.as_str()
    )
    .fetch_optional(pool)
    .await?;

    let rec = rec_opt.ok_or(LinkError::NotFound)?;
    let kind = if rec.kind == "redirect" {
        CachedLinkType::Redirect
    } else {
        CachedLinkType::Collection
    };

    Ok(Some(CachedLink {
        id: rec.id,
        kind,
        url: rec.target_url.unwrap_or_default(),
        last_seen: rec.last_seen,
        password_hash: rec.password_hash,
    }))
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
        SELECT alias, target_url
        FROM links
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    let links = rec_vec
        .into_iter()
        .map(|rec| LinkItem {
            alias: rec.alias,
            url: rec.target_url.unwrap(),
        })
        .collect();

    Ok(links)
}

/// Remove user's link
#[tracing::instrument(name = "services::remove_user_link", skip(pool))]
pub async fn delete_link_for_user(
    user_id: &UserId,
    alias: &LinkAlias,
    pool: &PgPool,
) -> Result<(), ServiceError> {
    sqlx::query!(
        r#"
        DELETE FROM links
        WHERE user_id = $1
          AND alias = $2
        "#,
        user_id,
        alias.as_str()
    )
    .execute(pool)
    .await?;

    Ok(())
}
