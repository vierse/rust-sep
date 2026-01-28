use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};

use crate::app::{AppState, usage_metrics::Category};

pub async fn redirect(State(app): State<AppState>, Path(alias): Path<String>) -> impl IntoResponse {
    app.metrics.log(Category::Redirect).await;

    match app.get_url(&alias).await {
        Ok(url) => Redirect::permanent(&url).into_response(),
        Err(e) => match e {
            crate::app::GetUrlError::AliasNotFount => {
                tracing::error!("redirect to an untracked alias");
                (StatusCode::NOT_FOUND).into_response()
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
