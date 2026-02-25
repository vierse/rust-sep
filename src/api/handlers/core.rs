use argon2::{PasswordHash, PasswordVerifier};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use cookie::{Cookie, SameSite};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::{
    api::{
        error::ApiError,
        extract::{MaybeToken, MaybeUser},
        token::Token,
    },
    app::{AppState, CachedLink, CachedLinkType, usage_metrics::Category},
    domain::{Alias, Url},
    services::{self, CollectionItem},
};

// TODO: settings
pub const EXPIRY_DAYS: i64 = 30;
pub const UNLOCK_PATH: &str = "unlock";

async fn fetch_link(alias: &Alias, app: &AppState) -> Result<CachedLink, ApiError> {
    let link_opt = if let Some(link) = app.cache.get(alias).await {
        app.diag.cache_hit();
        link
    } else {
        app.diag.cache_miss();
        app.cache
            .try_get_with_by_ref(alias, services::query_url_by_alias(alias, &app.pool))
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "failed to query the url");
                ApiError::internal()
            })?
    };

    let link = link_opt.ok_or_else(ApiError::not_found)?;

    let today = OffsetDateTime::now_utc().date();
    if link.last_seen < today.saturating_sub(Duration::days(EXPIRY_DAYS)) {
        return Err(ApiError::public(StatusCode::GONE, "The link has expired"));
    }

    Ok(link)
}

pub async fn redirect(
    MaybeToken(token): MaybeToken<UnlockToken>,
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Redirect, ApiError> {
    app.usage_metrics.log(Category::Redirect);
    let alias: Alias = alias.try_into()?;
    redirect_impl(&app, alias, None, token).await
}

pub async fn redirect_indexed(
    MaybeToken(token): MaybeToken<UnlockToken>,
    State(app): State<AppState>,
    Path((alias, idx)): Path<(String, usize)>,
) -> Result<Redirect, ApiError> {
    let alias: Alias = alias.try_into()?;
    redirect_impl(&app, alias, Some(idx), token).await
}

async fn redirect_impl(
    app: &AppState,
    alias: Alias,
    idx_opt: Option<usize>,
    token_opt: Option<UnlockToken>,
) -> Result<Redirect, ApiError> {
    let link = fetch_link(&alias, app).await?;

    // redirect if link is locked and user has no matching token
    let unlocked = token_opt
        .as_ref()
        .is_some_and(|t| t.alias == alias.as_str());
    if link.password_hash.is_some() && !unlocked {
        return Ok(Redirect::temporary(&format!(
            "/{UNLOCK_PATH}/{}",
            alias.as_str()
        )));
    }

    match link.kind {
        CachedLinkType::Redirect => {
            if idx_opt.is_some() {
                return Err(ApiError::not_found());
            }
            app.metrics.record_hit(link.id);
            Ok(Redirect::temporary(&link.url))
        }
        CachedLinkType::Collection => match idx_opt {
            Some(idx) => {
                let url =
                    services::query_collection_url_by_pos(link.id, idx as i32, &app.pool).await?;
                app.metrics.record_hit(link.id);
                Ok(Redirect::temporary(&url))
            }
            None => Ok(Redirect::temporary(&format!(
                "/collection/{}",
                alias.as_str()
            ))),
        },
    }
}

#[derive(Deserialize)]
pub struct UnlockRequest {
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct UnlockToken {
    alias: String,
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

    let link = fetch_link(&alias, &app).await?;

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
    MaybeToken(link_token): MaybeToken<LinkToken>,
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Response, ApiError> {
    let alias: Alias = alias.try_into()?;

    let link = fetch_link(&alias, &app).await?;

    // check if user has a matching token
    let unlocked = unlock_token.map_or(false, |t| t.alias == alias.as_str());
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
    let edit = link_token.map_or(false, |t| t.alias == alias.as_str());
    Ok(Json(CollectionResponse {
        alias: alias.as_str().to_owned(),
        items,
        edit,
    })
    .into_response())
}

#[derive(Serialize, Deserialize)]
pub struct ShortenRequest {
    pub url: String,
    pub alias: Option<String>,
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

#[derive(Serialize, Deserialize)]
pub struct LinkToken {
    alias: String,
    exp: i64,
}

impl LinkToken {
    const TTL_SECS: i64 = 24 * 60 * 60; // 1 hour

    fn new(alias: String, now_s: i64) -> Self {
        Self {
            alias,
            exp: now_s + Self::TTL_SECS,
        }
    }
}

impl Token for LinkToken {
    const TYPE: &'static str = "link_token";

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

    let alias = match alias {
        // If request contains an alias, validate and save it
        Some(user_alias) => {
            let alias: Alias = user_alias.try_into()?;

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
    let token = LinkToken::new(alias, now_s);
    let signed_token = app.signer.sign_token(&token)?;

    let cookie = Cookie::build(("link_token", signed_token))
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

pub async fn collection_create(
    MaybeToken(token): MaybeToken<LinkToken>,
    Path(alias): Path<String>,
    State(app): State<AppState>,
) -> Result<Response, ApiError> {
    let Some(token) = token else {
        return Err(ApiError::unauthorized());
    };

    if token.alias != alias {
        return Err(ApiError::unauthorized());
    }

    let alias: Alias = alias.try_into()?;
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
    MaybeToken(token): MaybeToken<LinkToken>,
    Path(alias): Path<String>,
    State(app): State<AppState>,
    Json(AddUrlRequest { url, title }): Json<AddUrlRequest>,
) -> Result<Response, ApiError> {
    let Some(token) = token else {
        return Err(ApiError::unauthorized());
    };

    if token.alias != alias {
        return Err(ApiError::unauthorized());
    }

    let alias: Alias = alias.try_into()?;
    let url: Url = url.try_into()?;

    services::add_url_to_collection(&alias, &url, title.as_deref(), &app.pool).await?;

    Ok((StatusCode::CREATED).into_response())
}

pub async fn recently_added_links(State(app): State<AppState>) -> Result<Response, ApiError> {
    app.usage_metrics.log(Category::RecentlyAdded);

    let links = services::recently_added_links(10, &app.pool).await?;

    Ok((StatusCode::OK, Json(links)).into_response())
}
