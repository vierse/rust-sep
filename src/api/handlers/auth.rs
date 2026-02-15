use axum::{
    Json,
    body::Body,
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use cookie::{Cookie, SameSite};
use serde::{Deserialize, Serialize};

use crate::{
    api::{error::ApiError, extract::RequireUser},
    app::AppState,
    services,
    usage_metrics::Category,
};

#[derive(Serialize, Deserialize)]
pub struct AuthRequest {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthResponse {
    username: String,
}

impl IntoResponse for AuthResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

pub async fn authenticate_session(
    RequireUser(session_id): RequireUser,
    State(app): State<AppState>,
) -> Result<Response<Body>, ApiError> {
    app.usage_metrics.log(Category::AuthenticateSession);
    let session = app.sessions.get_session_data(&session_id)?;

    println!("Logging out");
    Ok(AuthResponse {
        username: session.username.clone(),
    }
    .into_response())
}

pub async fn authenticate_user(
    State(app): State<AppState>,
    Json(AuthRequest { username, password }): Json<AuthRequest>,
) -> Result<Response<Body>, ApiError> {
    app.usage_metrics.log(Category::AuthenticateUser);
    // TODO: validate length
    let user = services::authenticate_user(&username, &password, &app.hasher, &app.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to authenticate");
            ApiError::internal()
        })?;

    let session_id = app.sessions.new_session(&user);

    let cookie = Cookie::build(("sid", session_id.as_str()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(false); // no https for now

    let mut response = AuthResponse { username }.into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).unwrap(),
    );

    Ok(response)
}

pub async fn create_user(
    State(app): State<AppState>,
    Json(AuthRequest { username, password }): Json<AuthRequest>,
) -> Result<Response<Body>, ApiError> {
    // TODO: validate length

    let Some(user) = services::create_user(&username, &password, &app.hasher, &app.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create user account");
            ApiError::internal()
        })?
    else {
        return Err(ApiError::public(
            StatusCode::BAD_REQUEST,
            "User already exists",
        ));
    };

    let session_id = app.sessions.new_session(&user);

    let cookie = Cookie::build(("sid", session_id.as_str()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(false); // no https for now

    let mut response = AuthResponse { username }.into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).unwrap(),
    );

    Ok(response)
}
