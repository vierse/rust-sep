use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    api::{auth::RequireUser, error::ApiError},
    app::AppState,
    services::query_links_by_user_id,
};

pub async fn list_links(
    RequireUser(user_id): RequireUser,
    State(app): State<AppState>,
) -> Result<Response, ApiError> {
    let links = query_links_by_user_id(&user_id, &app.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "app error");
            ApiError::internal()
        })?;

    Ok((StatusCode::OK, Json(links)).into_response())
}
