use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    api::{
        error::ApiError,
        extract::MaybeToken,
        handlers::{OwnerToken, UnlockToken},
    },
    app::{AppState, CachedLinkType},
    domain::{LinkAlias, Url},
    services::{self, CollectionItem},
};

#[derive(serde::Serialize)]
pub struct CollectionResponse {
    pub alias: String,
    pub items: Vec<CollectionItem>,
    pub edit: bool,
}

#[derive(Serialize)]
pub struct LockedResponse {
    unlock: String,
}

pub async fn collection_list(
    MaybeToken(unlock_token): MaybeToken<UnlockToken>,
    MaybeToken(owner_token): MaybeToken<OwnerToken>,
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Response, ApiError> {
    let alias: LinkAlias = alias.try_into()?;

    let link = super::fetch_link(&alias, &app).await?;

    // check if user has a matching token
    let unlocked = unlock_token.map_or(false, |t| link.id == t.link_id());
    if link.password_hash.is_some() && !unlocked {
        return Ok(LockedResponse {
            unlock: format!("/unlock/{}", alias.as_str()),
        }
        .into_response());
    }

    if link.kind != CachedLinkType::Collection {
        return Err(ApiError::bad_request());
    }

    let items = services::query_collection_by_id(link.id, &app.pool).await?;
    app.metrics.record_hit(link.id);

    // check if user can edit the collection
    let now_s = OffsetDateTime::now_utc().unix_timestamp();
    let edit = owner_token.map_or(false, |t| t.is_owner(link.id, now_s));
    Ok(Json(CollectionResponse {
        alias: alias.as_str().to_owned(),
        items,
        edit,
    })
    .into_response())
}

pub async fn collection_create(
    MaybeToken(token): MaybeToken<OwnerToken>,
    Path(alias): Path<String>,
    State(app): State<AppState>,
) -> Result<Response, ApiError> {
    let Some(token) = token else {
        return Err(ApiError::unauthorized());
    };

    let alias: LinkAlias = alias.try_into()?;
    let link = super::fetch_link(&alias, &app).await?;

    let now_s = OffsetDateTime::now_utc().unix_timestamp();
    if !token.is_owner(link.id, now_s) {
        return Err(ApiError::unauthorized());
    }

    services::convert_to_collection(&alias, &app.pool).await?;

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
    MaybeToken(token): MaybeToken<OwnerToken>,
    Path(alias): Path<String>,
    State(app): State<AppState>,
    Json(AddUrlRequest { url, title }): Json<AddUrlRequest>,
) -> Result<Response, ApiError> {
    let Some(token) = token else {
        return Err(ApiError::unauthorized());
    };

    let alias: LinkAlias = alias.try_into()?;

    let link = super::fetch_link(&alias, &app).await?;

    let now_s = OffsetDateTime::now_utc().unix_timestamp();
    if !token.is_owner(link.id, now_s) {
        return Err(ApiError::unauthorized());
    }

    let url: Url = url.try_into()?;

    services::add_url_to_collection(&alias, &url, title.as_deref(), &app.pool).await?;

    Ok((StatusCode::CREATED).into_response())
}
