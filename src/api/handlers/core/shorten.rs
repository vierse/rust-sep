use axum::{
    Json,
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
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
pub struct OwnerToken {
    link_id: i64,
    exp: i64,
}

impl OwnerToken {
    const TTL_SECS: i64 = 24 * 60 * 60; // 1 hour

    fn new(link_id: i64, now_s: i64) -> Self {
        Self {
            link_id,
            exp: now_s + Self::TTL_SECS,
        }
    }

    pub fn link_id(&self) -> i64 {
        self.link_id
    }
}

impl Token for OwnerToken {
    const TYPE: &'static str = "owner_token";

    fn exp(&self) -> i64 {
        self.exp
    }
}

pub async fn shorten(
    MaybeUser(session_id_opt): MaybeUser,
    State(app): State<AppState>,
    Json(ShortenRequest {
        url,
        alias,
        password,
    }): Json<ShortenRequest>,
) -> Result<Response, ApiError> {
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

    let mut response = ShortenResponse {
        alias: alias.clone(),
    }
    .into_response();

    let now = OffsetDateTime::now_utc();
    let now_s = now.unix_timestamp();
    let token = OwnerToken::new(link_id, now_s);
    let signed_token = app.signer.sign_token(&token)?;

    let cookie = Cookie::build(("owner_token", signed_token))
        .http_only(true)
        .secure(false)
        .path("/")
        .same_site(cookie::SameSite::Lax)
        .max_age(Duration::hours(1))
        .expires(now + Duration::hours(1))
        .build();

    let header_val = HeaderValue::from_str(&cookie.to_string()).expect("Could not build a cookie");

    response
        .headers_mut()
        .append(header::SET_COOKIE, header_val);

    Ok(response)
}
