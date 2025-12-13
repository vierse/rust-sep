use std::sync::Arc;

use axum::{body::Body, http::Request, http::StatusCode};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

use url_shorten::{api::build_router, core::AppState};

use crate::common::mock_core::MockApp;

mod common;

#[tokio::test]
async fn shorten_request_ok() {
    let app = MockApp {
        url: "https://www.example.com".to_string(),
        alias: "example".to_string(),
    };

    let router = build_router(AppState { app: Arc::new(app) });

    let request = serde_json::to_vec(&json!({ "url": "https://example.com" })).unwrap();
    let response = router
        .oneshot(
            Request::post("/api/shorten")
                .header("content-type", "application/json")
                .body(Body::from(request))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v.get("alias").is_some());
}

#[tokio::test]
async fn redirect_request_ok() {
    let app = MockApp {
        url: "https://www.example.com".to_string(),
        alias: "example".to_string(),
    };

    let router = build_router(AppState { app: Arc::new(app) });

    let response = router
        .oneshot(Request::get("/example").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::LOCATION)
            .unwrap(),
        "https://www.example.com"
    );
}
