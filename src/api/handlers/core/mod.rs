mod collection;
mod redirect;
mod shorten;
mod unlock;
pub(crate) use collection::*;
pub(crate) use redirect::*;
pub(crate) use shorten::*;
pub(crate) use unlock::*;

use crate::{
    api::error::ApiError,
    app::{AppState, CachedLink},
    domain::LinkAlias,
    services,
};
use axum::http::StatusCode;
use time::{Duration, OffsetDateTime};

pub const EXPIRY_DAYS: i64 = 30;

pub async fn fetch_link(alias: &LinkAlias, app: &AppState) -> Result<CachedLink, ApiError> {
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
