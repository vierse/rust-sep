use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    api::{error::ApiError, extract::RequireUser},
    app::AppState,
    services::{self, query_links_by_user_id},
};

pub async fn list_user_links(
    RequireUser(session_id): RequireUser,
    State(app): State<AppState>,
) -> Result<Response, ApiError> {
    let session = app.sessions.get_session_data(&session_id)?;
    let links = query_links_by_user_id(&session.user_id, &app.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "service error");
            ApiError::internal()
        })?;

    Ok((StatusCode::OK, Json(links)).into_response())
}

pub async fn remove_user_link(
    RequireUser(session_id): RequireUser,
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Response, ApiError> {
    let session = app.sessions.get_session_data(&session_id)?;
    services::remove_user_link(&session.user_id, &alias, &app.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "service error");
            ApiError::internal()
        })?;

    Ok(StatusCode::NO_CONTENT.into_response())
}
