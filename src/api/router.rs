use axum::{
    Router,
    routing::{get, post},
};
use tower_http::services::ServeDir;

use crate::{api::handlers, core::AppState};

pub fn build_router(state: AppState) -> Router {
    let serve = ServeDir::new("web/dist");

    Router::new()
        .route("/api/shorten", post(handlers::shorten))
        .route("/r/{alias}", get(handlers::redirect))
        .with_state(state)
        .fallback_service(serve)
}
