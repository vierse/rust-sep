use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};

use crate::app::AppState;
pub async fn recently_added_links(State(app): State<AppState>) -> impl IntoResponse {
    match app.recently_added_links(10).await {
        Ok(links) => (StatusCode::OK, Json(links)).into_response(),

        Err(e) => {
            tracing::error!(error = %e, "recently added links request error");
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}
