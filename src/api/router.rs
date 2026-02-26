use axum::{
    Router,
    middleware::{from_fn, from_fn_with_state},
    routing::{delete, get, post},
};
use metrics_exporter_prometheus::PrometheusBuilder;
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    api::{handlers, metrics, session},
    app::AppState,
};

const DIST_DIR: &str = "web/dist";

pub fn build_router(state: AppState) -> Router {
    let builder = PrometheusBuilder::new();
    let handle = builder
        .install_recorder()
        .expect("failed to install recorder");

    let metrics = Router::new().route(
        "/metrics",
        get(move || {
            let handle = handle.clone();
            async move { handle.render() }
        }),
    );

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

    // collection API
    let collection_api = Router::new()
        .route("/create", post(handlers::collection_create))
        .route(
            "/create/{alias}",
            post(handlers::collection_create_from_link),
        )
        .route("/{alias}/list", get(handlers::collection_list))
        .route("/{alias}/add", post(handlers::collection_add_url));

    // core API functions
    let core_api = Router::new()
        .nest("/auth", auth_api)
        .nest("/user", user_api)
        .nest("/collection", collection_api)
        .route("/shorten", post(handlers::shorten))
        .route("/unlock/{alias}", post(handlers::unlock))
        .route("/info/{alias}", get(handlers::link_info));

    // assemble everything
    let api = Router::new()
        .nest("/api", core_api)
        .route("/r/{alias}", get(handlers::redirect))
        .route("/r/{alias}/{idx}", get(handlers::redirect_indexed))
        .with_state(state.clone())
        .route_layer(from_fn(metrics::request_metrics_mw))
        .layer(from_fn_with_state(state, session::session_manager_mw)); // must be last

    // merge with assets
    let serve = ServeDir::new(DIST_DIR).fallback(ServeFile::new(format!("{DIST_DIR}/index.html")));
    Router::new()
        .merge(metrics)
        .merge(api)
        .fallback_service(serve)
}
