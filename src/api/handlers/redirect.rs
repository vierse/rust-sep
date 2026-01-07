use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
};

use crate::app::AppState;

pub async fn redirect(State(app): State<AppState>, Path(alias): Path<String>) -> impl IntoResponse {
    match app.get_url(&alias).await {
        Ok(url) => Redirect::permanent(&url).into_response(),
        Err(e) => e.into_response(),
    }
}
