use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    api::error::ApiError,
    app::AppState,
    domain::{Alias, Url},
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

pub async fn shorten(
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

            let result = services::create_link_with_alias(alias.as_str(), url.as_str(), &app.pool)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "app error");
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
            let alias = services::create_link(url.as_str(), &app.sqids, &app.pool)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "app error");
                    ApiError::internal()
                })?;

            Ok(ShortenResponse { alias })
        }
    }
}
