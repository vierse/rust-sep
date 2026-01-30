use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};

use crate::app::AppState;

const EXPIRED_LINK_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Link Expired</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: #1a1a2e;
            color: #eee;
        }
        .container { text-align: center; padding: 2rem; }
        h1 { color: #e94560; margin-bottom: 0.5rem; }
        p { color: #aaa; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Link Expired</h1>
        <p>This shortened link is no longer available.</p>
    </div>
</body>
</html>"#;

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
