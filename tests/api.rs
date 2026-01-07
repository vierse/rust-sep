use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

use axum::Router;

use url_shorten::{api, app};

// Deserialize a Response into T
async fn json<T: DeserializeOwned>(response: Response) -> T {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn router(pool: PgPool) -> Router {
    let state = app::build_app_state(pool).await.unwrap();
    api::build_router(state.into())
}

#[sqlx::test]
async fn shorten_and_redirect(pool: PgPool) {
    const TEST_URL: &str = "https://example.com";

    let router = router(pool).await;

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

#[sqlx::test]
async fn save_named_and_redirect(pool: PgPool) {
    // similar to shorten_and_redirect() but providing "name" in request body
    const TEST_URL: &str = "https://example.com";
    const TEST_ALIAS: &str = "testalias";

    let router = router(pool).await;

    // Make a POST request to /api/shorten
    let request_body =
        Body::from(serde_json::to_vec(&json!({ "url": TEST_URL, "name": TEST_ALIAS })).unwrap());
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
    assert_eq!(alias, TEST_ALIAS, "Response alias does not match request");

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

#[sqlx::test]
async fn save_named_already_exists(pool: PgPool) {
    const TEST_URL: &str = "https://example.com";
    const TEST_URL2: &str = "https://example2.com";
    const TEST_ALIAS: &str = "testalias2";

    let router = router(pool).await;

    let request_body =
        Body::from(serde_json::to_vec(&json!({"url": TEST_URL, "name": TEST_ALIAS })).unwrap());
    // Make a POST request to /api/shorten

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
    // second insert with same alias
    let request_body =
        Body::from(serde_json::to_vec(&json!({"url": TEST_URL2, "name": TEST_ALIAS})).unwrap());
    let request = Request::post("/api/shorten")
        .header("content-type", "application/json")
        .body(request_body)
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::CONFLICT,
        "Shorten request unexpectedly succeeded for an existing alias"
    );
}
