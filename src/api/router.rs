use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{delete, get, post},
};
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    api::{handlers, session},
    app::AppState,
};

const DIST_DIR: &str = "web/dist";

pub fn build_router(state: AppState) -> Router {
    // user API (auth required)
    let user_api = Router::new()
        .route("/list", get(handlers::list_user_links))
        .route("/link/{alias}", delete(handlers::remove_user_link))
        .route("/logout", post(handlers::logout));

    // auth management API
    let auth_api = Router::new()
        .route("/me", get(handlers::authenticate_session))
        .route("/login", post(handlers::authenticate_user))
        .route("/register", post(handlers::create_user));

    // core API functions
    let core_api = Router::new()
        .nest("/auth", auth_api)
        .nest("/user", user_api)
        .route("/shorten", post(handlers::shorten))
        .route("/recent", get(handlers::recently_added_links))
        .route("/unlock/{alias}", post(handlers::redirect_unlock));

    // assemble everything
    let api = Router::new()
        .nest("/api", core_api)
        .route("/r/{alias}", get(handlers::redirect))
        .with_state(state.clone())
        .layer(from_fn_with_state(state, session::session_manager_mw)); // must be last

    // merge with assets
    let serve = ServeDir::new(DIST_DIR).fallback(ServeFile::new(format!("{DIST_DIR}/index.html")));
    Router::new().merge(api).fallback_service(serve)
}
