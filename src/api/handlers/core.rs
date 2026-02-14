use argon2::{PasswordHash, PasswordVerifier};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::{
    api::{error::ApiError, extract::MaybeUser},
    app::{AppState, CachedLink, usage_metrics::Category},
    domain::{Alias, MAX_ALIAS_LENGTH, Url},
    services,
};

// TODO: settings
pub const EXPIRY_DAYS: i64 = 30;
pub const UNLOCK_PATH: &str = "unlock";

#[derive(Serialize, Deserialize)]
pub struct ShortenRequest {
    pub url: String,
    pub name: Option<String>,
    pub password: Option<String>,
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

async fn fetch_link(alias: &str, app: &AppState) -> Result<CachedLink, ApiError> {
    if alias.len() > MAX_ALIAS_LENGTH {
        tracing::error!("maximum alias length exceeded");
        return Err(ApiError::internal());
    }

    let link_opt = app
        .cache
        .try_get_with_by_ref(alias, services::query_url_by_alias(alias, &app.pool))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to query the url");
            ApiError::internal()
        })?;

    let link = link_opt.ok_or_else(|| {
        tracing::debug!("alias not found: {alias}");
        ApiError::not_found()
    })?;

    let today = OffsetDateTime::now_utc().date();
    if link.last_seen < today.saturating_sub(Duration::days(EXPIRY_DAYS)) {
        return Err(ApiError::public(StatusCode::GONE, "The link has expired"));
    }
    // TODO: mark the expired link for cleanup

    Ok(link)
}

pub async fn redirect(
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Redirect, ApiError> {
    let link = fetch_link(&alias, &app).await?;

    // Redirect to unlock view if the link is protected
    if link.password_hash.is_some() {
        return Ok(Redirect::temporary(&format!("/{UNLOCK_PATH}/{}", alias)));
    }

    // Update metrics
    app.metrics.record_hit(link.id);

    Ok(Redirect::temporary(&link.url))
}

#[derive(Deserialize)]
pub struct UnlockRequest {
    pub password: String,
}

#[derive(Serialize)]
pub struct UnlockResponse {
    pub url: String,
}

impl IntoResponse for UnlockResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

pub async fn redirect_unlock(
    State(app): State<AppState>,
    Path(alias): Path<String>,
    Json(UnlockRequest { password }): Json<UnlockRequest>,
) -> Result<UnlockResponse, ApiError> {
    let link = fetch_link(&alias, &app).await?;

    let Some(password_hash) = link.password_hash else {
        return Err(ApiError::bad_request());
    };

    let parsed_hash = PasswordHash::new(&password_hash).map_err(|e| {
        tracing::debug!(error = %e, "password hash parse error");
        ApiError::internal()
    })?;

    if app
        .hasher
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(ApiError::public(StatusCode::UNAUTHORIZED, "Wrong password"));
    }

    // Update metrics
    app.metrics.record_hit(link.id);

    Ok(UnlockResponse { url: link.url })
}

pub async fn shorten(
    MaybeUser(session_id_opt): MaybeUser,
    State(app): State<AppState>,
    Json(ShortenRequest {
        url,
        name,
        password,
    }): Json<ShortenRequest>,
) -> Result<ShortenResponse, ApiError> {
    app.usage_metrics.log(Category::Shorten);

    let url = Url::parse(&url).map_err(|e| {
        tracing::debug!(error = %e, "url parse error");
        ApiError::from(e)
    })?;

    let mut user_id = None;

    if let Some(session_id) = session_id_opt {
        let session = app.sessions.get_session_data(&session_id)?;
        user_id = Some(session.user_id);
    }

    let password_ref = password.as_deref();

    match name {
        // If request contains an alias, validate and save it
        Some(alias_str) => {
            let alias = Alias::parse(&alias_str).map_err(|e| {
                tracing::debug!(error = %e, "alias parse error");
                ApiError::from(e)
            })?;

            let result = services::create_link_with_alias(
                url.as_str(),
                alias.as_str(),
                &app.pool,
                user_id,
                password_ref,
                &app.hasher,
            )
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
            let alias = services::create_link(
                url.as_str(),
                &app.sqids,
                &app.pool,
                user_id,
                password_ref,
                &app.hasher,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "service error");
                ApiError::internal()
            })?;

            Ok(ShortenResponse { alias })
        }
    }
}

pub async fn recently_added_links(State(app): State<AppState>) -> Result<Response, ApiError> {
    app.usage_metrics.log(Category::RecentlyAdded);

    let links = services::recently_added_links(10, &app.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "service error");
            ApiError::internal()
        })?;

    Ok((StatusCode::OK, Json(links)).into_response())
}
