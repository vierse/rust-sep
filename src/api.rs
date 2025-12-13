use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::core::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/shorten", post(handle_shorten_request))
        .route("/{alias}", get(handle_redirect_request))
        .with_state(state)
}

#[derive(Deserialize)]
struct ShortenRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct ShortenResponse {
    pub alias: String,
}

async fn handle_shorten_request(
    State(AppState { app }): State<AppState>,
    Json(ShortenRequest { url }): Json<ShortenRequest>,
) -> impl IntoResponse {
    let result = app.create_alias(&url).await;

    if let Ok(alias) = result {
        (StatusCode::CREATED, Json(ShortenResponse { alias })).into_response()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR).into_response()
    }
}

async fn handle_redirect_request(
    State(AppState { app }): State<AppState>,
    Path(alias): Path<String>,
) -> impl IntoResponse {
    let result = app.get_url(&alias).await;

    if let Ok(url) = result {
        Redirect::permanent(&url).into_response()
    } else {
        (StatusCode::NOT_FOUND).into_response()
    }
}
