use anyhow::Context;
use serde::Serialize;
use sqids::Sqids;
use sqlx::PgPool;

use crate::{
    app::CachedLink,
    domain::{Url, UserId},
    services::ServiceError,
};

/// Create a new link for the provided URL
#[tracing::instrument(name = "services::create_link", skip(generator, pool))]
pub async fn create_link(
    url: &str,
    generator: &Sqids,
    pool: &PgPool,
    user_id: Option<UserId>,
) -> Result<String, ServiceError> {
    let mut tx = pool.begin().await.map_err(ServiceError::DatabaseError)?;
    // Insert the url into database to get a unique id
    let rec = sqlx::query!(
        r#"
        INSERT INTO links_main (url, user_id)
        VALUES ($1, $2)
        RETURNING id
        "#,
        url,
        user_id,
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
    user_id: Option<UserId>,
) -> Result<bool, ServiceError> {
    let rec = sqlx::query!(
        r#"
        INSERT INTO links_main (alias, url, user_id)
        VALUES ($1, $2, $3)
        ON CONFLICT (alias) DO NOTHING
        RETURNING id
        "#,
        alias,
        url,
        user_id,
    )
    .fetch_optional(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    Ok(rec.is_some())
}

/// Query url from database
///
/// Returns Ok(None) if the alias does not exist
#[tracing::instrument(name = "services::query_url_by_alias", skip(pool))]
pub async fn query_url_by_alias(
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
    alias: &str,
    pool: &PgPool,
) -> Result<(), ServiceError> {
    sqlx::query!(
        r#"
        DELETE FROM links_main
        WHERE user_id = $1
          AND alias = $2
        "#,
        user_id,
        alias
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
