use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    api::{auth::MaybeUser, error::ApiError},
    app::AppState,
    domain::{Alias, MAX_ALIAS_LENGTH, Url},
    services,
};

#[derive(Serialize, Deserialize)]
pub struct ShortenRequest {
    pub url: String,
    pub name: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ShortenResponse {
    pub alias: String,
}

impl IntoResponse for ShortenResponse {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, Json(self)).into_response()
    }
}

pub async fn redirect(
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Redirect, ApiError> {
    if alias.len() > MAX_ALIAS_LENGTH {
        tracing::error!("maximum alias length exceeded");
        return Err(ApiError::internal());
    }

    let key = alias.clone();
    let pool = app.pool.clone();

    // Try to get the URL from cache else query DB
    let link_opt = app
        .cache
        .try_get_with(key.clone(), async move {
            services::query_url_by_alias(&key, &pool).await
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to query or load url from cache");
            ApiError::internal()
        })?;

    let link = link_opt.ok_or_else(|| {
        tracing::debug!("alias not found: {alias}");
        ApiError::not_found()
    })?;

    // Update metrics
    app.metrics.record_hit(link.id);

    Ok(Redirect::permanent(&link.url))
}

pub async fn shorten(
    MaybeUser(user_id): MaybeUser,
    State(app): State<AppState>,
    Json(ShortenRequest { url, name }): Json<ShortenRequest>,
) -> Result<ShortenResponse, ApiError> {
    let url = Url::parse(&url).map_err(|e| {
        tracing::debug!(error = %e, "url parse error");
        ApiError::from(e)
    })?;

    // Parse and validate expires_at if provided, otherwise default to 7 days
    let expires_at = match expires_at {
        Some(expires_str) => match validate_expires_at(&expires_str) {
            Ok(dt) => dt,
            Err(e) => {
                tracing::warn!(cause = %e, "expires_at validation failed");
                return (StatusCode::BAD_REQUEST).into_response();
            }
        },
        None => OffsetDateTime::now_utc() + time::Duration::days(7),
    };

    match name {
        // If request contains an alias, validate and save it
        Some(alias_str) => {
            let alias = Alias::parse(&alias_str).map_err(|e| {
                tracing::debug!(error = %e, "alias parse error");
                ApiError::from(e)
            })?;

            let result =
                services::create_link_with_alias(url.as_str(), alias.as_str(), &app.pool, user_id)
                    .await
                    .map_err(|e| {
                        tracing::error!(error = %e, "service error");
                        ApiError::internal()
                    })?;

            if !result {
                tracing::debug!(cause = %alias.as_str(), "alias already taken");
                return Err(ApiError::public(
                    StatusCode::CONFLICT,
                    "This alias is already taken",
                ));
            }

            Ok(ShortenResponse { alias: alias_str })
        }

        // If request does not contain an alias, generate a new one
        None => {
            let alias = services::create_link(url.as_str(), &app.sqids, &app.pool, user_id)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "service error");
                    ApiError::internal()
                })?;

            Ok(ShortenResponse { alias })
        }
    }
}
