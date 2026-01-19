use axum::{
    extract::{Path, State},
    response::Redirect,
};

// TODO: unite all the constants into the app settings
const MAX_ALIAS_LENGTH: usize = 20;

use crate::{api::error::ApiError, app::AppState, services};

pub async fn redirect(
    State(app): State<AppState>,
    Path(alias): Path<String>,
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
            services::query_link_by_alias(&key, &pool).await
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

    // Update metrics
    app.metrics.record_hit(link.id);

    Ok(Redirect::permanent(&link.url))
}
