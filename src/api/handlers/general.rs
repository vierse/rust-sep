use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::{
    api::{auth::MaybeUser, error::ApiError},
    app::AppState,
    domain::{Alias, MAX_ALIAS_LENGTH, Url},
    services,
};

// TODO: settings
const EXPIRY_DAYS: i64 = 30;

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

#[derive(Deserialize)]
pub struct RedirectQuery {
    pub password: Option<String>,
}

pub async fn redirect(
    State(app): State<AppState>,
    Path(alias): Path<String>,
    axum::extract::Query(query): axum::extract::Query<RedirectQuery>,
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

    let today = OffsetDateTime::now_utc().date();
    if link.last_seen < today.saturating_sub(Duration::days(EXPIRY_DAYS)) {
        return Err(ApiError::public(StatusCode::GONE, "The link has expired"));
    }
    // TODO: mark the expired link for cleanup

    // Check password if the link is protected
    if let Some(ref stored_hash) = link.password_hash {
        match query.password.as_deref() {
            // No password provided — redirect to the SPA prompt page
            None | Some("") => {
                return Ok(Redirect::temporary(&format!("/?unlock={}", alias)));
            }
            // Password provided — verify it
            Some(provided) => {
                let parsed_hash = PasswordHash::new(stored_hash).map_err(|e| {
                    tracing::error!(error = %e, "failed to parse stored password hash");
                    ApiError::internal()
                })?;
                if Argon2::default()
                    .verify_password(provided.as_bytes(), &parsed_hash)
                    .is_err()
                {
                    return Err(ApiError::public(StatusCode::UNAUTHORIZED, "Wrong password"));
                }
            }
        }
    }

    // Update metrics
    app.metrics.record_hit(link.id);

    Ok(Redirect::permanent(&link.url))
}

pub async fn shorten(
    MaybeUser(user_id): MaybeUser,
    State(app): State<AppState>,
    Json(ShortenRequest {
        url,
        name,
        password,
    }): Json<ShortenRequest>,
) -> Result<ShortenResponse, ApiError> {
    let url = Url::parse(&url).map_err(|e| {
        tracing::debug!(error = %e, "url parse error");
        ApiError::from(e)
    })?;

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
    let links = services::recently_added_links(10, &app.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "service error");
            ApiError::internal()
        })?;

    Ok((StatusCode::OK, Json(links)).into_response())
}
