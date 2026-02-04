use axum::{
    Router,
    body::Body,
    http::{HeaderValue, Request, header},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    api::{auth::ClearSid, handlers},
    app::AppState,
};

const DIST_DIR: &str = "web/dist";

pub async fn clear_sid_mw(req: Request<Body>, next: Next) -> Response {
    let should_clear = req.extensions().get::<ClearSid>().is_some();

    let mut res = next.run(req).await;

    if should_clear {
        res.headers_mut().append(
            header::SET_COOKIE,
            HeaderValue::from_static("sid=; Max-Age=0; Path=/; HttpOnly; SameSite=Lax"),
        );
    }

    res
}

pub fn build_router(state: AppState) -> Router {
    let serve = ServeDir::new(DIST_DIR).fallback(ServeFile::new(format!("{DIST_DIR}/index.html")));

    let auth_api = Router::new()
        .route("/me", get(handlers::authenticate_session))
        .route("/login", post(handlers::authenticate_user))
        .route("/register", post(handlers::create_user));

    let user_api = Router::new().route("/list", get(handlers::list_links));

    let api = Router::new()
        .nest("/auth", auth_api)
        .nest("/user", user_api)
        .route("/shorten", post(handlers::shorten))
        .layer(middleware::from_fn(clear_sid_mw));

    Router::new()
        .nest("/api", api)
        .route("/r/{alias}", get(handlers::redirect))
        .with_state(state)
        .fallback_service(serve)
}
