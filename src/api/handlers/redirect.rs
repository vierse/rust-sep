use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};

use crate::app::AppState;

pub async fn redirect(
    State(app): State<Arc<AppState>>,
    Path(alias): Path<String>,
) -> impl IntoResponse {
    match app.get_url(&alias).await {
        Ok(url) => Redirect::permanent(&url).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "redirect request err");
            (StatusCode::NOT_FOUND).into_response()
        }
    }
}
