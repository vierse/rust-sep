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
    api::{error::ApiError, session::SessionData},
    app::AppState,
    services,
};

#[derive(Serialize, Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    username: String,
}

impl IntoResponse for LoginResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

pub async fn login(
    State(app): State<AppState>,
    Json(LoginRequest { username, password }): Json<LoginRequest>,
) -> Result<Response<Body>, ApiError> {
    // TODO: validate length

    let user_id = services::verify_user_password(&username, &password, &app.hasher, &app.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to create user account");
            ApiError::internal()
        })?;

    let Some(user_id) = user_id else {
        return Err(ApiError::public(
            StatusCode::UNAUTHORIZED,
            "Failed to authenticate",
        ));
    };

    let session_id = app.sessions.new_session(SessionData { user_id });

    let cookie = Cookie::build(("sid", session_id.as_str()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(false); // no https for now

    let mut response = LoginResponse { username }.into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).unwrap(),
    );

    Ok(response)
}
