use argon2::{PasswordHash, PasswordVerifier};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use cookie::{Cookie, SameSite};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::{
    api::{error::ApiError, extract::MaybeToken, token::Token},
    app::AppState,
    domain::Alias,
};

#[derive(Deserialize)]
pub struct UnlockRequest {
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct UnlockToken {
    pub alias: String,
    exp: i64,
}

impl UnlockToken {
    const TTL_SECS: i64 = 30 * 60; // 30 minutes

    fn new(alias: String, now_s: i64) -> Self {
        Self {
            alias,
            exp: now_s + Self::TTL_SECS,
        }
    }
}

impl Token for UnlockToken {
    const TYPE: &'static str = "unlock";

    fn exp(&self) -> i64 {
        self.exp
    }
}

pub async fn unlock(
    jar: CookieJar,
    MaybeToken(unlock_token): MaybeToken<UnlockToken>,
    State(app): State<AppState>,
    Path(alias): Path<String>,
    Json(UnlockRequest { password }): Json<UnlockRequest>,
) -> Result<(CookieJar, Response), ApiError> {
    let alias: Alias = alias.try_into()?;

    let link = super::fetch_link(&alias, &app).await?;

    let Some(password_hash) = link.password_hash else {
        return Err(ApiError::bad_request());
    };

    if let Some(token) = unlock_token {
        if token.alias == alias.as_str() {
            return Ok((jar, StatusCode::OK.into_response()));
        }
    }

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

    let now = OffsetDateTime::now_utc();
    let now_s = now.unix_timestamp();

    let token = UnlockToken::new(alias.as_str().to_owned(), now_s);
    let signed_token = app.signer.sign_token(&token)?;

    let cookie = Cookie::build(("unlock", signed_token))
        .http_only(true)
        .secure(false)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(Duration::seconds(UnlockToken::TTL_SECS))
        .expires(now + Duration::seconds(UnlockToken::TTL_SECS))
        .build();

    let jar = jar.add(cookie);

    Ok((jar, StatusCode::OK.into_response()))
}
