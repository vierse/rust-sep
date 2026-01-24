use axum::{
    Router,
    routing::{get, post},
};
use tower_http::services::{ServeDir, ServeFile};

use crate::{api::handlers, app::AppState};

const DIST_DIR: &str = "web/dist";

pub fn build_router(state: AppState) -> Router {
    let serve = ServeDir::new(DIST_DIR).fallback(ServeFile::new(format!("{DIST_DIR}/index.html")));

    Router::new()
        .route("/api/shorten", post(handlers::shorten))
        .route("/r/{alias}", get(handlers::redirect))
        .route("/api/auth/register", post(handlers::register))
        .route("/api/auth/login", post(handlers::login))
        .with_state(state)
        .fallback_service(serve)
}
