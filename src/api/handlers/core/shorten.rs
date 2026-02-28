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
    api::{
        AppState,
        error::ApiError,
        extract::{MaybeToken, MaybeUser},
        token::{OwnerToken, Token},
    },
    domain::{LinkAlias, LinkPassword, Url},
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

pub async fn shorten(
    jar: CookieJar,
    MaybeUser(session_id_opt): MaybeUser,
    MaybeToken(token): MaybeToken<OwnerToken>,
    State(app): State<AppState>,
    Json(ShortenRequest {
        url,
        alias,
        password,
    }): Json<ShortenRequest>,
) -> Result<(CookieJar, Response), ApiError> {
    let url: Url = url.try_into()?;

    let mut user_id = None;

    if let Some(session_id) = session_id_opt {
        let session = app.sessions.get_session_data(&session_id)?;
        user_id = Some(session.user_id);
    }

    let password_ref = password.as_deref();

    let (link_id, alias_out) = match alias {
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

    app.link_cache_neg.invalidate(&alias_out).await;

    let response = ShortenResponse { alias: alias_out }.into_response();

    let now = OffsetDateTime::now_utc();
    let now_s = now.unix_timestamp();

    let mut token = token.unwrap_or_default();
    token.update(link_id, now_s);

    let signed_token = app.signer.sign_token(&token)?;

    let cookie = Cookie::build((OwnerToken::TYPE, signed_token))
        .http_only(true)
        .secure(false)
        .path("/")
        .same_site(cookie::SameSite::Lax)
        .max_age(Duration::seconds(OwnerToken::ttl()))
        .expires(now + Duration::seconds(OwnerToken::ttl()))
        .build();
    let jar = jar.add(cookie);

    Ok((jar, response))
}

#[derive(Deserialize)]
pub struct CreateCollectionRequest {
    alias: Option<String>,
    password: Option<String>,
    urls: Vec<String>,
}

#[derive(Serialize)]
pub struct CreateCollectionResponse {
    alias: String,
}

impl IntoResponse for CreateCollectionResponse {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, Json(self)).into_response()
    }
}

pub async fn collection_create(
    jar: CookieJar,
    MaybeToken(token): MaybeToken<OwnerToken>,
    State(app): State<AppState>,
    Json(CreateCollectionRequest {
        alias,
        password,
        urls,
    }): Json<CreateCollectionRequest>,
) -> Result<(CookieJar, Response), ApiError> {
    let alias = alias.map(LinkAlias::try_from).transpose()?;
    let password = password.map(LinkPassword::try_from).transpose()?;
    let urls: Vec<String> = urls
        .into_iter()
        .map(Url::try_from)
        .map(|u| u.map(|url| url.into_string()))
        .collect::<Result<_, _>>()?;

    let (link_id, alias_out) = services::create_collection(
        urls,
        alias.as_deref(),
        password.as_deref(),
        &app.sqids,
        &app.hasher,
        &app.pool,
    )
    .await?;

    app.link_cache_neg.invalidate(&alias_out).await;

    let response = CreateCollectionResponse { alias: alias_out }.into_response();

    let now = OffsetDateTime::now_utc();
    let now_s = now.unix_timestamp();

    let mut token = token.unwrap_or_default();
    token.update(link_id, now_s);

    let signed_token = app.signer.sign_token(&token)?;

    let cookie = Cookie::build((OwnerToken::TYPE, signed_token))
        .http_only(true)
        .secure(false)
        .path("/")
        .same_site(cookie::SameSite::Lax)
        .max_age(Duration::seconds(OwnerToken::ttl()))
        .expires(now + Duration::seconds(OwnerToken::ttl()))
        .build();

    let jar = jar.add(cookie);

    Ok((jar, response))
}
