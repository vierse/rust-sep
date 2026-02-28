use serde::Serialize;
use sqlx::PgPool;

use crate::{
    api::{CachedLink, CachedLinkType},
    domain::{LinkId, UserId},
    services::{LinkError, ServiceError},
};

pub async fn query_url_by_alias(alias: &str, pool: &PgPool) -> Result<CachedLink, ServiceError> {
    let rec_opt = sqlx::query!(
        r#"
        SELECT
            id,
            kind,
            target_url,
            user_id,
            last_seen,
            password_hash,
            created_at
        FROM links
        WHERE alias = $1
        "#,
        alias
    )
    .fetch_optional(pool)
    .await?;

    let rec = rec_opt.ok_or(LinkError::NotFound)?;
    let kind = if rec.kind == "redirect" {
        CachedLinkType::Redirect
    } else {
        CachedLinkType::Collection
    };

    Ok(CachedLink {
        id: rec.id,
        kind,
        target_url: rec.target_url,
        user_id: rec.user_id,
        last_seen: rec.last_seen,
        password_hash: rec.password_hash,
        created_at: rec.created_at,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkItem {
    alias: String,
    kind: String,
    protected: bool,
}

pub async fn query_links_by_user_id(
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Vec<LinkItem>, ServiceError> {
    let rec_vec = sqlx::query!(
        r#"
        SELECT
            alias,
            kind,
            (password_hash IS NOT NULL) AS "protected!"
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
            kind: rec.kind,
            protected: rec.protected,
        })
        .collect();

    Ok(links)
}

#[derive(Debug, Clone, Serialize)]
pub struct CollectionItem {
    position: i32,
    url: String,
    title: Option<String>,
}

pub async fn query_collection_by_id(
    link_id: LinkId,
    pool: &PgPool,
) -> Result<Vec<CollectionItem>, ServiceError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, position, target_url, title
        FROM collection_items
        WHERE link_id = $1
        ORDER BY position ASC
        "#,
        link_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| CollectionItem {
            position: r.position,
            url: r.target_url,
            title: r.title,
        })
        .collect())
}

pub async fn query_collection_url_by_pos(
    link_id: LinkId,
    pos: i32,
    pool: &PgPool,
) -> Result<String, ServiceError> {
    let rec_opt = sqlx::query!(
        r#"
        SELECT target_url
        FROM collection_items
        WHERE link_id = $1
          AND position = $2
        "#,
        link_id,
        pos
    )
    .fetch_optional(pool)
    .await?;

    let rec = rec_opt.ok_or(LinkError::NotFound)?;
    Ok(rec.target_url)
}
