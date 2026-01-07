use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};

use crate::{api::handlers, app::AppState};

const DIST_DIR: &str = "web/dist";

pub fn build_router(state: Arc<AppState>) -> Router {
    let serve = ServeDir::new(DIST_DIR).fallback(ServeFile::new(format!("{DIST_DIR}/index.html")));

    Router::new()
        .route("/api/shorten", post(handlers::shorten))
        .route("/r/{alias}", get(handlers::redirect))
        .with_state(state)
        .fallback_service(serve)
}
