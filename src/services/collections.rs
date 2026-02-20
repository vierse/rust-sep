use anyhow::anyhow;
use serde::Serialize;
use sqlx::PgPool;

use crate::{
    domain::{Alias, Url, UserId},
    services::ServiceError,
};

/// Create a collection: insert multiple URLs under one alias
pub async fn create_collection(
    alias: &str,
    urls: &[String],
    pool: &PgPool,
    user_id: Option<UserId>,
) -> Result<bool, ServiceError> {
    if urls.is_empty() {
        return Err(ServiceError::Other(anyhow!(
            "collection must include at least one URL"
        )));
    }

    let alias: Alias = alias
        .to_string()
        .try_into()
        .map_err(|e: crate::domain::AliasParseError| ServiceError::Other(e.into()))?;

    for url in urls {
        let _: Url = url
            .clone()
            .try_into()
            .map_err(|e: crate::domain::UrlParseError| ServiceError::Other(e.into()))?;
    }

    let mut tx = pool.begin().await.map_err(ServiceError::DatabaseError)?;

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
        return Ok(false);
    };

    let collection_id = rec.id;

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

    Ok(true)
}

/// Get all items in a collection by alias, ordered by position.
/// Returns the collection id alongside the items for metrics tracking.
pub async fn get_collection(
    alias: &str,
    pool: &PgPool,
) -> Result<Option<(i64, Vec<CollectionItem>)>, ServiceError> {
    let rows = sqlx::query!(
        r#"
        SELECT c.id as "collection_id!: i64", url, position
        FROM collection_items ci
        JOIN collections c ON c.id = ci.collection_id
        WHERE c.alias = $1
        ORDER BY position
        "#,
        alias,
    )
    .fetch_all(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    if rows.is_empty() {
        Ok(None)
    } else {
        let collection_id = rows[0].collection_id;
        let items = rows
            .into_iter()
            .map(|r| CollectionItem {
                url: r.url,
                position: r.position,
            })
            .collect();
        Ok(Some((collection_id, items)))
    }
}

/// Get a single item from a collection by alias and index (position).
/// Returns the collection id alongside the url for metrics tracking.
pub async fn get_collection_item(
    alias: &str,
    index: i32,
    pool: &PgPool,
) -> Result<Option<(i64, String)>, ServiceError> {
    let rec = sqlx::query!(
        r#"
        SELECT c.id as "collection_id!: i64", url
        FROM collection_items ci
        JOIN collections c ON c.id = ci.collection_id
        WHERE c.alias = $1 AND position = $2
        "#,
        alias,
        index,
    )
    .fetch_optional(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    Ok(rec.map(|r| (r.collection_id, r.url)))
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
        ORDER BY c.id DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    let collections = rec_vec
        .into_iter()
        .map(|rec| CollectionListItem {
            alias: rec.alias,
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
