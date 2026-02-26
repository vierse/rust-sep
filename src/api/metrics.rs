use axum::{body::Body, extract::MatchedPath, http::Request, middleware::Next, response::Response};
use metrics::SharedString;
use std::time::Instant;

pub async fn request_metrics_mw(req: Request<Body>, next: Next) -> Response {
    let start = Instant::now();

    let route: SharedString = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
        .into();

    let method: SharedString = req.method().as_str().to_string().into();

    let res = next.run(req).await;

    let status_class: &'static str = match res.status().as_u16() / 100 {
        2 => "2xx",
        3 => "3xx",
        4 => "4xx",
        5 => "5xx",
        _ => "other",
    };

    metrics::counter!(
        "http_requests_total",
        "method" => method.clone(),
        "route" => route.clone(),
        "status" => status_class,
    )
    .increment(1);

    metrics::histogram!(
        "http_request_duration_seconds",
        "method" => method,
        "route" => route,
    )
    .record(start.elapsed().as_secs_f64());

    res
}
