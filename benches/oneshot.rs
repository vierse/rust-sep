use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;
use tower::ServiceExt;

fn bench_axum_handler(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let app = rt.block_on(async {
        let config = url_shorten::config::load().unwrap();
        let pool = url_shorten::app::connect_to_db(config.database_url.as_str())
            .await
            .unwrap();
        let state = url_shorten::app::build_app_state(pool).await.unwrap();
        url_shorten::api::build_router(state)
    });

    c.bench_function("axum GET /r/ (oneshot)", |b| {
        b.to_async(&rt).iter_batched(
            || {
                Request::builder()
                    .method("GET")
                    .uri("/r/testing")
                    .body(Body::empty())
                    .unwrap()
            },
            // service call
            |req| async {
                let resp = app.clone().oneshot(req).await.unwrap();
                std::hint::black_box(resp.status() == StatusCode::OK);
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_axum_handler);
criterion_main!(benches);
