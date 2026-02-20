use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    api::{error::ApiError, extract::RequireUser, session::ClearSid},
    app::AppState,
    domain::Alias,
    services::{self, query_collections_by_user_id, query_links_by_user_id},
};

pub async fn list_user_links(
    RequireUser(session_id): RequireUser,
    State(app): State<AppState>,
) -> Result<Response, ApiError> {
    let session = app.sessions.get_session_data(&session_id)?;
    let links = query_links_by_user_id(&session.user_id, &app.pool).await?;

    Ok((StatusCode::OK, Json(links)).into_response())
}

pub async fn remove_user_link(
    RequireUser(session_id): RequireUser,
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Response, ApiError> {
    let alias: Alias = alias.try_into()?;

    let session = app.sessions.get_session_data(&session_id)?;
    services::remove_user_link(&session.user_id, &alias, &app.pool).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn list_user_collections(
    RequireUser(session_id): RequireUser,
    State(app): State<AppState>,
) -> Result<Response, ApiError> {
    let session = app.sessions.get_session_data(&session_id)?;
    let collections = query_collections_by_user_id(&session.user_id, &app.pool).await?;

    Ok((StatusCode::OK, Json(collections)).into_response())
}

pub async fn remove_user_collection(
    RequireUser(session_id): RequireUser,
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Response, ApiError> {
    let session = app.sessions.get_session_data(&session_id)?;
    services::remove_user_collection(&session.user_id, &alias, &app.pool).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn logout(
    RequireUser(session_id): RequireUser,
    State(app): State<AppState>,
) -> Result<Response, ApiError> {
    app.sessions.close_session(&session_id);

    let mut res = StatusCode::NO_CONTENT.into_response();
    res.extensions_mut().insert(ClearSid);
    Ok(res)
}
