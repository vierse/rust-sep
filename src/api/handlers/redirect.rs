use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};

use crate::app::AppState;

pub async fn redirect(State(app): State<AppState>, Path(alias): Path<String>) -> impl IntoResponse {
    let result = app.get_url(&alias).await;

    if let Ok(url) = result {
        Redirect::permanent(&url).into_response()
    } else {
        (StatusCode::NOT_FOUND).into_response()
    }
}
