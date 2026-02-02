use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};

use crate::app::AppState;

const EXPIRED_LINK_HTML: &str = include_str!("expired_link.html");

pub async fn redirect(State(app): State<AppState>, Path(alias): Path<String>) -> impl IntoResponse {
    match app.get_url(&alias).await {
        Ok(url) => Redirect::permanent(&url).into_response(),
        Err(e) => match e {
            crate::app::GetUrlError::AliasNotFount => {
                tracing::error!("redirect to an untracked alias");
                (StatusCode::NOT_FOUND).into_response()
            }
            crate::app::GetUrlError::LinkExpired => {
                tracing::info!("redirect to an expired link");
                (StatusCode::GONE, Html(EXPIRED_LINK_HTML)).into_response()
            }
            crate::app::GetUrlError::HitLogFail(url, error) => {
                tracing::error!(error = %error, "failed to log url access");
                Redirect::permanent(&url).into_response()
            }
            crate::app::GetUrlError::DBErr(error) => {
                tracing::error!(error = %error, "get_url failed with database error");
                (StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
        },
    }
}
