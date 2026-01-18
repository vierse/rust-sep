use axum::{
    extract::{Path, State},
    response::Redirect,
};

// TODO: unite all the constants into the app settings
const MAX_ALIAS_LENGTH: usize = 20;

use crate::{api::error::ApiError, app::AppState};

pub async fn redirect(
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Redirect, ApiError> {
    if alias.len() > MAX_ALIAS_LENGTH {
        tracing::error!("maximum alias length exceeded");
        return Err(ApiError::internal());
    }

    let url_opt = app.get_url(&alias).await.map_err(|e| {
        tracing::error!(error = %e, "app error");
        ApiError::internal()
    })?;

    let url = url_opt.ok_or_else(|| {
        tracing::debug!("alias not found: {alias}");
        ApiError::not_found()
    })?;

    Ok(Redirect::permanent(&url))
}
