use axum::{
    body::Body,
    http::{
        Request, StatusCode,
        header::{self, LOCATION},
    },
    response::Response,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::json;
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;

use axum::Router;

use url_shorten::api::{self, AppKeys, AppState, handlers::EXPIRY_DAYS};

#[derive(Deserialize)]
struct ShortenResponse {
    alias: String,
}

// Deserialize a Response into T
async fn json<T: DeserializeOwned>(response: Response) -> T {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn router(pool: PgPool) -> Router {
    dotenv::dotenv().ok();
    let app_keys = AppKeys::from_env().expect("Could not load app keys from env");
    let state = AppState::new(pool, app_keys).expect("Could not build AppState");
    api::build_router(state)
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
    let ShortenResponse { alias } = json(response).await;

    // Make a GET request to /r/{alias}
    let request_body = Body::empty();
    let request = Request::get(format!("/r/{alias}"))
        .body(request_body)
        .unwrap();
    let response = router.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::TEMPORARY_REDIRECT,
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
        Body::from(serde_json::to_vec(&json!({ "url": TEST_URL, "alias": TEST_ALIAS })).unwrap());
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
    let ShortenResponse { alias } = json(response).await;
    assert_eq!(alias, TEST_ALIAS, "Response alias does not match request");

    // Make a GET request to /r/{alias}
    let request_body = Body::empty();
    let request = Request::get(format!("/r/{alias}"))
        .body(request_body)
        .unwrap();
    let response = router.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::TEMPORARY_REDIRECT,
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
        Body::from(serde_json::to_vec(&json!({"url": TEST_URL, "alias": TEST_ALIAS })).unwrap());
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
        Body::from(serde_json::to_vec(&json!({"url": TEST_URL2, "alias": TEST_ALIAS})).unwrap());
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

#[sqlx::test]
async fn redirect_expired_link(pool: PgPool) {
    const TEST_URL: &str = "https://example.com/";
    const ALIAS: &str = "testing";

    let expired_on = OffsetDateTime::now_utc()
        .date()
        .saturating_sub(Duration::days(EXPIRY_DAYS + 1));

    sqlx::query!(
        r#"
        INSERT INTO links (alias, target_url, last_seen)
        VALUES ($1, $2, $3)
        "#,
        ALIAS,
        TEST_URL,
        expired_on
    )
    .execute(&pool)
    .await
    .unwrap();

    let router = router(pool).await;

    let request = Request::get(format!("/r/{ALIAS}"))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::GONE,
        "Expected expired link to return 410 Gone"
    );
}

#[sqlx::test]
async fn password_protected_link_unlock(pool: PgPool) {
    const TEST_URL: &str = "https://example.com";
    const TEST_ALIAS: &str = "testing";
    const TEST_PASSWORD: &str = "password123";

    let router = router(pool).await;

    // 1. Create a password-protected link
    let request_body = Body::from(
        serde_json::to_vec(&json!({
            "url": TEST_URL,
            "alias": TEST_ALIAS,
            "password": TEST_PASSWORD
        }))
        .unwrap(),
    );

    let request = Request::post("/api/shorten")
        .header("content-type", "application/json")
        .body(request_body)
        .unwrap();

    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // 2. Normal redirect should redirect to unlock prompt
    let request = Request::get(format!("/r/{TEST_ALIAS}"))
        .body(Body::empty())
        .unwrap();

    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::TEMPORARY_REDIRECT,
        "Expected 307 redirect to unlock prompt for protected link"
    );

    let location = response.headers().get(LOCATION).unwrap().to_str().unwrap();
    assert_eq!(
        location,
        format!("/unlock/{}", TEST_ALIAS),
        "Expected Location header to point to unlock prompt"
    );

    // 3. POST unlock with wrong password
    let request_body = Body::from(serde_json::to_vec(&json!({ "password": "qwerty" })).unwrap());

    let request = Request::post(format!("/api/unlock/{TEST_ALIAS}"))
        .header("content-type", "application/json")
        .body(request_body)
        .unwrap();

    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Expected 401 when provided with wrong password"
    );

    // 4. POST unlock with correct password
    let request_body =
        Body::from(serde_json::to_vec(&json!({ "password": TEST_PASSWORD })).unwrap());

    let request = Request::post(format!("/api/unlock/{TEST_ALIAS}"))
        .header("content-type", "application/json")
        .body(request_body)
        .unwrap();

    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Expected 200 OK when provided with correct password"
    );

    // 5. Capture unlock token
    let cookie_str = response
        .headers()
        .get(header::SET_COOKIE)
        .expect("Unlock should set cookie")
        .to_str()
        .unwrap();

    // 6. Redirect should work now
    let request = Request::get(format!("/r/{TEST_ALIAS}"))
        .header(header::COOKIE, cookie_str)
        .body(Body::empty())
        .unwrap();

    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::TEMPORARY_REDIRECT,
        "Expected 307 redirect to unlock prompt for protected link"
    );

    let location = response.headers().get(LOCATION).unwrap().to_str().unwrap();
    assert_eq!(
        location, TEST_URL,
        "Expected Location header to point to test URL"
    );
}
