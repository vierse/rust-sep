use anyhow::{Context, anyhow};
use serde::Serialize;
use sqlx::PgPool;
use thiserror::Error;

use crate::{
    domain::{LinkAlias, Url},
    services::ServiceError,
};

// TODO: settings
pub const MAX_COLLECTION_ITEMS: i32 = 20;

#[derive(Debug, Error)]
pub enum CollectionError {
    #[error("link not found")]
    LinkNotFound,
    #[error("reached url limit of a collection")]
    LimitReached,
}

#[derive(Debug, Clone, Serialize)]
pub struct CollectionItem {
    position: i32,
    url: String,
    title: Option<String>,
}

pub async fn convert_to_collection(alias: &LinkAlias, pool: &PgPool) -> Result<(), ServiceError> {
    let mut tx = pool.begin().await?;

    let parent = sqlx::query!(
        r#"
        SELECT id, kind, target_url
        FROM links
        WHERE alias = $1
        FOR UPDATE
        "#,
        alias.as_str(),
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(CollectionError::LinkNotFound)?;

    match parent.kind.as_str() {
        "redirect" => {
            let target_url = parent
                .target_url
                .context("Expected redirect record to contain target URL")?;

            sqlx::query!(
                r#"
                INSERT INTO collection_items (link_id, position, target_url, title)
                VALUES ($1, 0, $2, NULL)
                "#,
                parent.id,
                target_url,
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                r#"
                UPDATE links
                SET kind = 'collection',
                    target_url = NULL,
                    updated_at = now()
                WHERE id = $1
                "#,
                parent.id,
            )
            .execute(&mut *tx)
            .await?;
        }

        "collection" => {
            return Err(anyhow!("Already a collection").into());
        }
        _ => {
            return Err(anyhow!("DB contained unexpected value for record kind").into());
        }
    }

    tx.commit().await?;

    Ok(())
}

pub async fn add_url_to_collection(
    alias: &LinkAlias,
    url: &Url,
    title: Option<&str>,
    pool: &PgPool,
) -> Result<(), ServiceError> {
    let mut tx = pool.begin().await?;

    let parent = sqlx::query!(
        r#"
        SELECT id, kind, target_url
        FROM links
        WHERE alias = $1
        FOR UPDATE
        "#,
        alias.as_str(),
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(CollectionError::LinkNotFound)?;

    match parent.kind.as_str() {
        "collection" => {
            let count = sqlx::query_scalar!(
                r#"
                SELECT COUNT(*)::INT AS "count!"
                FROM collection_items
                WHERE link_id = $1
                "#,
                parent.id,
            )
            .fetch_one(&mut *tx)
            .await?;

            if count >= MAX_COLLECTION_ITEMS {
                return Err(CollectionError::LimitReached.into());
            }

            let next_position = sqlx::query_scalar!(
                r#"
                SELECT COALESCE(MAX(position) + 1, 0)::INT AS "next_position!"
                FROM collection_items
                WHERE link_id = $1
                "#,
                parent.id,
            )
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query!(
                r#"
                INSERT INTO collection_items (link_id, position, target_url, title)
                VALUES ($1, $2, $3, $4)
                "#,
                parent.id,
                next_position,
                url.as_str(),
                title,
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                r#"
                UPDATE links
                SET updated_at = now()
                WHERE id = $1
                "#,
                parent.id,
            )
            .execute(&mut *tx)
            .await?;
        }
        "redirect" => return Err(anyhow!("this is not a collection").into()),
        _ => {
            return Err(anyhow!("DB contained unexpected value for record kind").into());
        }
    }

    tx.commit().await?;

    Ok(())
}

pub async fn query_collection_by_id(
    link_id: i64,
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
    link_id: i64,
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

    let rec = rec_opt.ok_or(CollectionError::LinkNotFound)?;
    Ok(rec.target_url)
}
