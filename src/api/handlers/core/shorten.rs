use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use cookie::Cookie;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::{
    api::{error::ApiError, extract::MaybeUser, token::Token},
    app::{AppState, usage_metrics::Category},
    domain::{LinkAlias, Url},
    services,
};

#[derive(Deserialize)]
pub struct ShortenRequest {
    pub url: String,
    pub alias: Option<String>,
    pub password: Option<String>,
}

#[derive(Serialize)]
pub struct ShortenResponse {
    pub alias: String,
}

impl IntoResponse for ShortenResponse {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, Json(self)).into_response()
    }
}

#[derive(Serialize, Deserialize)]
pub struct OwnerToken(Vec<(i64, i64)>);

impl OwnerToken {
    const MAX_OWNERS: usize = 50;
    const TTL_SECS: i64 = 24 * 60 * 60; // 24 hours

    pub fn is_owner(&self, link_id: i64, now_s: i64) -> bool {
        self.0
            .iter()
            .any(|(id, exp)| *id == link_id && *exp > now_s)
    }

    pub fn remaining_secs(&self, link_id: i64, now_s: i64) -> Option<i64> {
        self.0
            .iter()
            .find(|(id, _)| *id == link_id)
            .and_then(|(_, exp)| (*exp > now_s).then_some(*exp - now_s))
    }

    fn empty() -> Self {
        Self(Vec::new())
    }

    fn update(&mut self, link_id: i64, now_s: i64) {
        // prune expired owners
        self.0.retain(|(_, exp)| *exp > now_s);

        let owner = (link_id, now_s + Self::TTL_SECS);

        // refresh if owner exists
        if let Some(existing) = self.0.iter_mut().find(|(id, _)| *id == link_id) {
            *existing = owner;
            return;
        }

        if self.0.len() >= Self::MAX_OWNERS {
            // prune the oldest
            if let Some((idx, _)) = self.0.iter().enumerate().min_by_key(|(_, (_, exp))| *exp) {
                self.0.swap_remove(idx);
            }
        }

        self.0.push(owner);
    }

    fn max_exp(&self) -> Option<i64> {
        self.0.iter().map(|(_, exp)| *exp).max()
    }
}

impl Token for OwnerToken {
    const TYPE: &'static str = "owner";

    fn exp(&self) -> i64 {
        self.max_exp().unwrap_or(0)
    }
}

pub async fn shorten(
    jar: CookieJar,
    MaybeUser(session_id_opt): MaybeUser,
    State(app): State<AppState>,
    Json(ShortenRequest {
        url,
        alias,
        password,
    }): Json<ShortenRequest>,
) -> Result<(CookieJar, Response), ApiError> {
    app.usage_metrics.log(Category::Shorten);

    let url: Url = url.try_into()?;

    let mut user_id = None;

    if let Some(session_id) = session_id_opt {
        let session = app.sessions.get_session_data(&session_id)?;
        user_id = Some(session.user_id);
    }

    let password_ref = password.as_deref();

    let (link_id, alias) = match alias {
        // If request contains an alias, validate and save it
        Some(user_alias) => {
            let alias: LinkAlias = user_alias.try_into()?;

            services::create_link_with_alias(
                &url,
                &alias,
                &app.pool,
                user_id,
                password_ref,
                &app.hasher,
            )
            .await?
        }
        // If request does not contain an alias, generate a new one
        None => {
            services::create_link(
                &url,
                &app.sqids,
                &app.pool,
                user_id,
                password_ref,
                &app.hasher,
            )
            .await?
        }
    };

    let response = ShortenResponse { alias }.into_response();

    let now = OffsetDateTime::now_utc();
    let now_s = now.unix_timestamp();

    let mut token = match jar.get(OwnerToken::TYPE) {
        Some(cookie) => app
            .signer
            .verify_token(cookie.value(), now_s)
            .unwrap_or_else(|_| OwnerToken::empty()),
        None => OwnerToken::empty(),
    };
    token.update(link_id, now_s);

    let signed_token = app.signer.sign_token(&token)?;

    let cookie = Cookie::build((OwnerToken::TYPE, signed_token))
        .http_only(true)
        .secure(false)
        .path("/")
        .same_site(cookie::SameSite::Lax)
        .max_age(Duration::seconds(OwnerToken::TTL_SECS))
        .expires(now + Duration::seconds(OwnerToken::TTL_SECS))
        .build();
    let jar = jar.add(cookie);

    Ok((jar, response))
}
