use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    api::{auth::RequireUser, error::ApiError},
    app::AppState,
    services::query_links_by_user,
};

pub async fn list_links(
    RequireUser(user): RequireUser,
    State(app): State<AppState>,
) -> Result<Response, ApiError> {
    // TODO: cache

    let links = query_links_by_user(&user, &app.pool).await.map_err(|e| {
        tracing::error!(error = %e, "app error");
        ApiError::internal()
    })?;

    let links: Vec<String> = links.iter().map(|l| l.url.clone()).collect();

    Ok((StatusCode::OK, Json(links)).into_response())
}
