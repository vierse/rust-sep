use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::api::error::ApiError;
use crate::api::extract::MaybeUser;
use crate::app::AppState;
use crate::services;
use crate::tasks::link_metrics::EntityKey;

#[derive(Deserialize)]
pub struct CreateCollectionRequest {
    pub alias: String,
    pub urls: Vec<String>,
}

/// POST /api/collection — create a collection (multiple URLs under one alias)
pub async fn create_collection(
    MaybeUser(session_id_opt): MaybeUser,
    State(app): State<AppState>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Response, ApiError> {
    let user_id = session_id_opt
        .map(|sid| app.sessions.get_session_data(&sid).map(|s| s.user_id))
        .transpose()?;

    let created = services::create_collection(&req.alias, &req.urls, &app.pool, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to create collection");
            ApiError::internal()
        })?;

    if !created {
        return Err(ApiError::public(
            StatusCode::CONFLICT,
            "This alias is already taken",
        ));
    }

    Ok(StatusCode::CREATED.into_response())
}

/// GET /api/collection/:alias — list all links in a collection
pub async fn get_collection(
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Response, ApiError> {
    let result = services::get_collection(&alias, &app.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to get collection");
            ApiError::internal()
        })?;

    match result {
        Some((collection_id, items)) => {
            app.metrics.record_hit(EntityKey::Collection(collection_id));
            Ok(Json(items).into_response())
        }
        None => Err(ApiError::not_found()),
    }
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
    let result = services::get_collection_item(&alias, query.i, &app.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to get collection item");
            ApiError::internal()
        })?;

    match result {
        Some((collection_id, url)) => {
            app.metrics.record_hit(EntityKey::Collection(collection_id));
            Ok(Json(CollectionItemResponse { url }).into_response())
        }
        None => Err(ApiError::not_found()),
    }
}

#[derive(Serialize)]
struct CollectionItemResponse {
    url: String,
}
