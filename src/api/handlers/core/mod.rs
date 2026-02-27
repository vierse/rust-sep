mod collection;
mod info;
mod redirect;
mod shorten;
mod unlock;

pub(crate) use collection::*;
pub(crate) use info::*;
pub(crate) use redirect::*;
pub(crate) use shorten::*;
pub(crate) use unlock::*;

use crate::{
    api::{error::ApiError, session::SessionId},
    app::{AppState, CachedLink},
    domain::LinkAlias,
    services,
};
use axum::http::StatusCode;
use time::{Duration, OffsetDateTime};

pub const EXPIRY_DAYS: i64 = 30;

async fn fetch_link(alias: &LinkAlias, app: &AppState) -> Result<CachedLink, ApiError> {
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

fn is_user_owned(session_id: Option<&SessionId>, link: &CachedLink, app: &AppState) -> bool {
    session_id
        .and_then(|sid| app.sessions.get_session_data(sid).ok())
        .is_some_and(|session| Some(session.user_id) == link.user_id)
}

fn is_token_owned(token: Option<OwnerToken>, link: &CachedLink) -> bool {
    token.is_some_and(|t| {
        let now_s = OffsetDateTime::now_utc().unix_timestamp();
        t.is_owner(link.id, now_s, link.created_at.unix_timestamp())
    })
}
