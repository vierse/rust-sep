use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use tower::ServiceExt;

use axum::Router;

use url_shorten::{api, app, config};

// Deserialize a Response into T
async fn json<T: DeserializeOwned>(response: Response) -> T {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn router() -> Router {
    let config = config::load().expect("Could not load config");
    let state = app::build_app_state(config.database_url.as_str())
        .await
        .unwrap();
    api::build_router(state)
}

#[tokio::test]
async fn shorten_and_redirect() {
    const TEST_URL: &str = "https://example.com";

    let router = router().await;

    // Make a POST request to /api/shorten
    let request_body = Body::from(serde_json::to_vec(&json!({ "url": TEST_URL })).unwrap());
    let request = Request::post("/api/shorten")
        .header("content-type", "application/json")
        .body(request_body)
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "Request to shorten {TEST_URL} failed"
    );

    // Parse the returned alias
    let api::handlers::ShortenResponse { alias } = json(response).await;

    // Make a GET request to /r/{alias}
    let request_body = Body::empty();
    let request = Request::get(format!("/r/{alias}"))
        .body(request_body)
        .unwrap();
    let response = router.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::PERMANENT_REDIRECT,
        "Redirect request to /r/{alias} failed"
    );
    // Check that the redirect location is set to our url
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::LOCATION)
            .unwrap(),
        TEST_URL,
        "Redirect location does not match original url"
    );
}
