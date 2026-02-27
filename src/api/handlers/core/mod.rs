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
    api::{AppState, error::ApiError, session::SessionId, state::CachedLink, token::Token},
    domain::LinkAlias,
    services::{self, LinkError, ServiceError},
};
use axum::http::StatusCode;
use metrics::counter;
use time::{Duration, OffsetDateTime};

pub const EXPIRY_DAYS: i64 = 30;

async fn fetch_link(alias: &LinkAlias, app: &AppState) -> Result<CachedLink, ApiError> {
    fn validate_cached_link(link: &CachedLink) -> Result<(), ApiError> {
        let today = OffsetDateTime::now_utc().date();
        if link.last_seen < today.saturating_sub(Duration::days(EXPIRY_DAYS)) {
            return Err(ApiError::public(StatusCode::GONE, "The link has expired"));
        }
        Ok(())
    }

    if app.link_cache_neg.contains_key(alias) {
        counter!("link_cache_neg_hits_total").increment(1);
        return Err(ApiError::not_found());
    }

    if let Some(link) = app.link_cache.get(alias).await {
        counter!("link_cache_requests_total", "result" => "hit").increment(1);

        if let Err(e) = validate_cached_link(&link) {
            app.link_cache.invalidate(alias).await;
            return Err(e);
        }

        return Ok(link);
    }
    counter!("link_cache_requests_total", "result" => "miss").increment(1);

    let link_res = app
        .link_cache
        .try_get_with_by_ref(alias, async move {
            services::query_url_by_alias(alias, &app.pool).await
        })
        .await;

    match link_res {
        Ok(link) => {
            if let Err(e) = validate_cached_link(&link) {
                app.link_cache.invalidate(alias).await;
                Err(e)
            } else {
                Ok(link)
            }
        }
        Err(e) => match e.as_ref() {
            ServiceError::LinkError(LinkError::NotFound) => {
                app.link_cache_neg.insert(alias.clone(), ()).await;
                counter!("link_cache_neg_inserts_total").increment(1);
                Err(ApiError::not_found())
            }
            _ => {
                tracing::error!(error = %e, "failed to query link");
                Err(ApiError::internal())
            }
        },
    }
}

fn is_user_owned(session_id: Option<&SessionId>, link: &CachedLink, app: &AppState) -> bool {
    session_id
        .and_then(|sid| app.sessions.get_session_data(sid).ok())
        .is_some_and(|session| Some(session.user_id) == link.user_id)
}

fn is_token_active<T: Token>(token: Option<&T>, link: &CachedLink) -> bool {
    let now_s = OffsetDateTime::now_utc().unix_timestamp();
    token.is_some_and(|t| t.contains(link.id, now_s))
}
