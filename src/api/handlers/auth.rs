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
    app::{AppState, usage_metrics::Category},
    domain::{UserName, UserPassword},
    services,
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

fn build_cookie_header(sid: &str) -> HeaderValue {
    let cookie = Cookie::build(("sid", sid))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(false); // no https for now

    HeaderValue::from_str(&cookie.to_string()).expect("Could not build a cookie")
}

pub async fn authenticate_session(
    RequireUser(session_id): RequireUser,
    State(app): State<AppState>,
) -> Result<Response<Body>, ApiError> {
    app.usage_metrics.log(Category::AuthenticateSession);
    let session = app.sessions.get_session_data(&session_id)?;

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

    let username: UserName = username.try_into()?;
    let password: UserPassword = password.try_into()?;

    let user = services::authenticate_user(username, password, &app.hasher, &app.pool).await?;

    let session_id = app.sessions.new_session(&user);

    let mut response = AuthResponse {
        username: user.name().to_string(),
    }
    .into_response();
    response
        .headers_mut()
        .append(header::SET_COOKIE, build_cookie_header(session_id.as_str()));

    Ok(response)
}

pub async fn create_user(
    State(app): State<AppState>,
    Json(AuthRequest { username, password }): Json<AuthRequest>,
) -> Result<Response<Body>, ApiError> {
    let username: UserName = username.try_into()?;
    let password: UserPassword = password.try_into()?;

    let Some(user) = services::create_user(username, password, &app.hasher, &app.pool).await?
    else {
        return Err(ApiError::public(
            StatusCode::BAD_REQUEST,
            "User already exists",
        ));
    };

    let session_id = app.sessions.new_session(&user);

    let mut response = AuthResponse {
        username: user.name().to_string(),
    }
    .into_response();
    response
        .headers_mut()
        .append(header::SET_COOKIE, build_cookie_header(session_id.as_str()));

    Ok(response)
}
