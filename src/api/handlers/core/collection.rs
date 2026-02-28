use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use metrics::counter;

use crate::{
    api::{
        AppState,
        error::ApiError,
        extract::{MaybeToken, MaybeUser},
        state::CachedLinkType,
        token::{OwnerToken, UnlockToken},
    },
    domain::{LinkAlias, Url},
    services::{self, CollectionItem},
};

#[derive(serde::Serialize)]
pub struct CollectionResponse {
    pub alias: String,
    pub items: Vec<CollectionItem>,
    pub owned: bool,
}

#[derive(Serialize)]
pub struct LockedResponse {
    unlock: String,
}

pub async fn collection_list(
    MaybeUser(session_id): MaybeUser,
    MaybeToken(unlock_token): MaybeToken<UnlockToken>,
    MaybeToken(owner_token): MaybeToken<OwnerToken>,
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Response, ApiError> {
    let alias: LinkAlias = alias.try_into()?;

    let link = super::fetch_link(&alias, &app).await?;

    // redirect to unlock prompt if the collection is locked
    let unlocked = super::is_token_active(unlock_token.as_ref(), &link);
    if link.password_hash.is_some() && !unlocked {
        return Ok(LockedResponse {
            unlock: format!("/unlock/{}", alias.as_str()),
        }
        .into_response());
    }

    if link.kind != CachedLinkType::Collection {
        return Err(ApiError::bad_request());
    }

    // try load from cache else query the DB
    let items = match app.coll_cache.get(&link.id).await {
        Some(items) => {
            counter!("collection_cache_requests_total", "result" => "hit").increment(1);
            items
        }
        None => {
            counter!("collection_cache_requests_total", "result" => "miss").increment(1);
            let pool = app.pool.clone();
            app.coll_cache
                .try_get_with(link.id, async move {
                    services::query_collection_by_id(link.id, &pool).await
                })
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, link.id, "failed to load collection items");
                    ApiError::internal()
                })?
        }
    };

    app.metrics.record_hit(link.id);

    // check if owned (just visual indicator for the user)
    let owned = super::is_user_owned(session_id.as_ref(), &link, &app)
        || super::is_token_active(owner_token.as_ref(), &link);
    Ok(Json(CollectionResponse {
        alias: alias.as_str().to_owned(),
        items,
        owned,
    })
    .into_response())
}

pub async fn collection_create_from_link(
    MaybeUser(session_id): MaybeUser,
    MaybeToken(token): MaybeToken<OwnerToken>,
    Path(alias): Path<String>,
    State(app): State<AppState>,
) -> Result<Response, ApiError> {
    let alias: LinkAlias = alias.try_into()?;
    let link = super::fetch_link(&alias, &app).await?;

    let owned = super::is_user_owned(session_id.as_ref(), &link, &app)
        || super::is_token_active(token.as_ref(), &link);
    if !owned {
        return Err(ApiError::unauthorized());
    }

    services::convert_to_collection(&alias, &app.pool).await?;

    app.link_cache.invalidate(alias.as_str()).await;

    Ok((
        StatusCode::CREATED,
        Json(format!("/collection/{}", alias.as_str())),
    )
        .into_response())
}

#[derive(Deserialize)]
pub struct AddUrlRequest {
    url: String,
    title: Option<String>,
}
impl IntoResponse for LockedResponse {
    fn into_response(self) -> Response {
        (StatusCode::LOCKED, Json(self)).into_response()
    }
}

pub async fn collection_add_url(
    MaybeUser(session_id): MaybeUser,
    MaybeToken(token): MaybeToken<OwnerToken>,
    Path(alias): Path<String>,
    State(app): State<AppState>,
    Json(AddUrlRequest { url, title }): Json<AddUrlRequest>,
) -> Result<Response, ApiError> {
    let alias: LinkAlias = alias.try_into()?;
    let link = super::fetch_link(&alias, &app).await?;

    let owned = super::is_user_owned(session_id.as_ref(), &link, &app)
        || super::is_token_active(token.as_ref(), &link);
    if !owned {
        return Err(ApiError::unauthorized());
    }

    let url: Url = url.try_into()?;

    services::add_url_to_collection(&alias, &url, title.as_deref(), &app.pool).await?;

    app.coll_cache.invalidate(&link.id).await;

    Ok((StatusCode::CREATED).into_response())
}
