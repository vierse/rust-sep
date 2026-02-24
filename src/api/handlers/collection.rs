use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::api::error::ApiError;
use crate::api::extract::MaybeUser;
use crate::app::{AppState, CachedCollection};
use crate::domain::Alias;
use crate::services;
use crate::tasks::link_metrics::EntityKey;

const EXPIRY_DAYS: i64 = 30;

#[derive(Deserialize)]
pub struct CreateCollectionRequest {
    pub alias: Option<String>,
    pub urls: Vec<String>,
}

#[derive(Serialize)]
pub struct CreateCollectionResponse {
    pub alias: String,
}

/// POST /api/collection — create a collection (multiple URLs under one alias)
pub async fn create_collection(
    MaybeUser(session_id_opt): MaybeUser,
    State(app): State<AppState>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Response, ApiError> {
    if req.urls.is_empty() {
        return Err(ApiError::public(
            StatusCode::BAD_REQUEST,
            "Collection must include at least one URL",
        ));
    }

    let user_id = session_id_opt
        .map(|sid| app.sessions.get_session_data(&sid).map(|s| s.user_id))
        .transpose()?;

    if let Some(ref alias) = req.alias {
        let _: crate::domain::Alias = alias.clone().try_into().map_err(ApiError::from)?;
    }

    let alias = services::create_collection(
        req.alias.as_deref(),
        &req.urls,
        &app.sqids,
        &app.pool,
        user_id,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to create collection");
        ApiError::from(e)
    })?;

    Ok((
        StatusCode::CREATED,
        Json(CreateCollectionResponse { alias }),
    )
        .into_response())
}

async fn fetch_collection(alias: &Alias, app: &AppState) -> Result<CachedCollection, ApiError> {
    let coll_opt = if let Some(cached) = app.collection_cache.get(alias).await {
        app.diag.cache_hit();
        cached
    } else {
        app.diag.cache_miss();
        app.collection_cache
            .try_get_with_by_ref(alias, services::query_collection_by_alias(alias, &app.pool))
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "failed to query collection");
                ApiError::internal()
            })?
    };

    let collection = coll_opt.ok_or_else(ApiError::not_found)?;

    let today = OffsetDateTime::now_utc().date();
    if collection.last_seen < today.saturating_sub(Duration::days(EXPIRY_DAYS)) {
        return Err(ApiError::public(
            StatusCode::GONE,
            "The collection has expired",
        ));
    }

    Ok(collection)
}

/// GET /api/collection/:alias — list all links in a collection
pub async fn get_collection(
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Response, ApiError> {
    let alias: Alias = alias.try_into()?;
    let collection = fetch_collection(&alias, &app).await?;

    app.metrics.record_hit(EntityKey::Collection(collection.id));

    let items: Vec<_> = collection
        .items
        .into_iter()
        .map(|item| services::CollectionItem {
            url: item.url,
            position: item.position,
        })
        .collect();

    Ok(Json(items).into_response())
}

#[derive(Deserialize)]
pub struct CollectionItemQuery {
    pub i: i32,
}

/// GET /api/collection/:alias/item?i=N — get the Nth link in a collection
pub async fn get_collection_item(
    State(app): State<AppState>,
    Path(alias): Path<String>,
    Query(query): Query<CollectionItemQuery>,
) -> Result<Response, ApiError> {
    let alias: Alias = alias.try_into()?;
    let collection = fetch_collection(&alias, &app).await?;

    let url = collection
        .items
        .iter()
        .find(|item| item.position == query.i)
        .map(|item| item.url.clone())
        .ok_or_else(ApiError::not_found)?;

    app.metrics.record_hit(EntityKey::Collection(collection.id));

    Ok(Json(CollectionItemResponse { url }).into_response())
}

#[derive(Serialize)]
struct CollectionItemResponse {
    url: String,
}
