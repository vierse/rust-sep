use argon2::{PasswordHash, PasswordVerifier};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use cookie::{Cookie, SameSite};
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use crate::{
    api::{
        error::ApiError,
        extract::MaybeToken,
        token::{Token, UnlockToken},
    },
    app::AppState,
    domain::LinkAlias,
};

#[derive(Deserialize)]
pub struct UnlockRequest {
    pub password: String,
}

pub async fn unlock(
    jar: CookieJar,
    MaybeToken(token): MaybeToken<UnlockToken>,
    State(app): State<AppState>,
    Path(alias): Path<String>,
    Json(UnlockRequest { password }): Json<UnlockRequest>,
) -> Result<(CookieJar, Response), ApiError> {
    let alias: LinkAlias = alias.try_into()?;

    let link = super::fetch_link(&alias, &app).await?;

    let Some(ref password_hash) = link.password_hash else {
        return Err(ApiError::bad_request());
    };

    if super::is_token_active(token.as_ref(), &link) {
        return Ok((jar, StatusCode::OK.into_response()));
    }

    let mut token = token.unwrap_or_else(UnlockToken::empty);

    let parsed_hash = PasswordHash::new(password_hash).map_err(|e| {
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

    let now = OffsetDateTime::now_utc();
    let now_s = now.unix_timestamp();
    token.update(link.id, now_s);
    let signed_token = app.signer.sign_token(&token)?;

    let cookie = Cookie::build((UnlockToken::TYPE, signed_token))
        .http_only(true)
        .secure(false)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(Duration::seconds(UnlockToken::ttl()))
        .expires(now + Duration::seconds(UnlockToken::ttl()))
        .build();

    let jar = jar.add(cookie);

    Ok((jar, StatusCode::OK.into_response()))
}
