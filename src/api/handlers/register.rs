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
    api::{auth::MaybeUser, error::ApiError},
    app::AppState,
    domain::User,
    services,
};

#[derive(Serialize, Deserialize)]
pub struct RegisterRequest {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
pub struct RegisterResponse {
    username: String,
}

impl IntoResponse for RegisterResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

pub async fn register(
    MaybeUser(user): MaybeUser,
    State(app): State<AppState>,
    Json(RegisterRequest { username, password }): Json<RegisterRequest>,
) -> Result<Response<Body>, ApiError> {
    // TODO: validate length

    if user.is_some() {
        return Err(ApiError::public(
            StatusCode::BAD_REQUEST,
            "Already signed in",
        ));
    }

    let user_id = services::create_user_account(&username, &password, &app.hasher, &app.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to create user account");
            ApiError::internal()
        })?;

    let session_id = app.sessions.new_session(User::new(user_id));

    let cookie = Cookie::build(("sid", session_id.as_str()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(false); // no https for now

    let mut response = RegisterResponse { username }.into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).unwrap(),
    );

    Ok(response)
}
