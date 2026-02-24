use anyhow::{Context, anyhow};
use serde::Serialize;
use sqids::Sqids;
use sqlx::PgPool;

use crate::{
    app::{CachedCollection, CachedCollectionItem},
    domain::{Alias, Url, UserId},
    services::{LinkServiceError, ServiceError},
};

/// Create a collection: insert multiple URLs under one alias.
/// If `alias` is `Some`, uses user-chosen alias with conflict detection.
/// If `alias` is `None`, auto-generates an alias via Sqids two-step insert.
#[tracing::instrument(name = "services::create_collection", skip(generator, pool))]
pub async fn create_collection(
    alias: Option<&str>,
    urls: &[String],
    generator: &Sqids,
    pool: &PgPool,
    user_id: Option<UserId>,
) -> Result<String, ServiceError> {
    if urls.is_empty() {
        return Err(ServiceError::Other(anyhow!(
            "collection must include at least one URL"
        )));
    }

    for url in urls {
        let _: Url = url
            .clone()
            .try_into()
            .map_err(|e: crate::domain::UrlParseError| ServiceError::Other(e.into()))?;
    }

    let mut tx = pool.begin().await.map_err(ServiceError::DatabaseError)?;

    let (collection_id, alias) = match alias {
        Some(alias_str) => {
            let alias: Alias = alias_str
                .to_string()
                .try_into()
                .map_err(|e: crate::domain::AliasParseError| ServiceError::Other(e.into()))?;

            let rec = sqlx::query!(
                r#"
                INSERT INTO collections(alias, user_id)
                VALUES ($1, $2)
                ON CONFLICT (alias) DO NOTHING
                RETURNING id
                "#,
                alias.as_str(),
                user_id,
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(ServiceError::DatabaseError)?;

            let Some(rec) = rec else {
                return Err(LinkServiceError::AlreadyExists.into());
            };

            (rec.id, alias_str.to_string())
        }
        None => {
            let rec = sqlx::query!(
                r#"
                INSERT INTO collections(user_id)
                VALUES ($1)
                RETURNING id
                "#,
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

            sqlx::query!(
                r#"
                UPDATE collections
                SET alias = $1
                WHERE id = $2
                "#,
                alias,
                rec.id,
            )
            .execute(&mut *tx)
            .await
            .map_err(ServiceError::DatabaseError)?;

            (rec.id, alias)
        }
    };

    for (i, url) in urls.iter().enumerate() {
        let position = i32::try_from(i)
            .map_err(|_| ServiceError::Other(anyhow!("collection item index overed i32")))?;

        sqlx::query!(
            r#"
            INSERT INTO collection_items (collection_id, url, position)
            VALUES ($1, $2, $3)
            "#,
            collection_id,
            url,
            position,
        )
        .execute(&mut *tx)
        .await
        .map_err(ServiceError::DatabaseError)?;
    }

    tx.commit().await.map_err(ServiceError::DatabaseError)?;

    Ok(alias)
}

#[tracing::instrument(name = "services::query_collection_by_alias", skip(pool))]
pub async fn query_collection_by_alias(
    alias: &Alias,
    pool: &PgPool,
) -> Result<Option<CachedCollection>, ServiceError> {
    let rows = sqlx::query!(
        r#"
        SELECT c.id as "collection_id!: i64", c.last_seen, url, position
        FROM collection_items ci
        JOIN collections c ON c.id = ci.collection_id
        WHERE c.alias = $1
        ORDER BY position
        "#,
        alias.as_str(),
    )
    .fetch_all(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    if rows.is_empty() {
        Ok(None)
    } else {
        let id = rows[0].collection_id;
        let last_seen = rows[0].last_seen;
        let items = rows
            .into_iter()
            .map(|r| CachedCollectionItem {
                url: r.url,
                position: r.position,
            })
            .collect();
        Ok(Some(CachedCollection {
            id,
            last_seen,
            items,
        }))
    }
}

/// List user's collections
#[tracing::instrument(name = "services::query_collections_by_user_id", skip(pool))]
pub async fn query_collections_by_user_id(
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Vec<CollectionListItem>, ServiceError> {
    let rec_vec = sqlx::query!(
        r#"
        SELECT c.alias, COUNT(ci.id) as "item_count!"
        FROM collections c
        JOIN collection_items ci ON ci.collection_id = c.id
        WHERE c.user_id = $1
        GROUP BY c.id, c.alias
        ORDER BY c.created_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    let collections = rec_vec
        .into_iter()
        .map(|rec| CollectionListItem {
            alias: rec.alias.unwrap_or_default(),
            item_count: rec.item_count,
        })
        .collect();

    Ok(collections)
}

/// Remove user's collection
#[tracing::instrument(name = "services::remove_user_collection", skip(pool))]
pub async fn remove_user_collection(
    user_id: &UserId,
    alias: &str,
    pool: &PgPool,
) -> Result<(), ServiceError> {
    sqlx::query!(
        r#"
        DELETE FROM collections
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

#[derive(Debug, Clone, Serialize)]
pub struct CollectionItem {
    pub url: String,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CollectionListItem {
    pub alias: String,
    pub item_count: i64,
}
