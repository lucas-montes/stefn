use axum::{
    http::{HeaderValue, Request, StatusCode},
    response::{Html, IntoResponse, Response},
    Router,
};
use hyper::header::CONTENT_TYPE;
use std::{
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};
use tower::ServiceBuilder;
use tower_http::{
    normalize_path::NormalizePathLayer,
    request_id::{MakeRequestId, RequestId},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
    timeout::TimeoutLayer,
    trace::{
        DefaultOnBodyChunk, DefaultOnEos, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse,
        TraceLayer,
    },
    LatencyUnit, ServiceBuilderExt,
};
use tracing::Level;

pub fn get_router<S>(state: S, routes: Router<S>) -> Router
where
    S: Send + Sync + Clone + 'static,
{
    //TODO: depending on test or prod enables some
    // let sensitive_headers: Arc<[_]> = vec![AUTHORIZATION, COOKIE].into();
    let sensitive_headers: Arc<[_]> = vec![].into();
    // Build our middleware stack
    let middleware = ServiceBuilder::new()
        .layer(NormalizePathLayer::trim_trailing_slash())
        // Mark the `Authorization` and `Cookie` headers as sensitive so it doesn't show in logs
        .layer(SetSensitiveRequestHeadersLayer::from_shared(
            sensitive_headers.clone(),
        ))
        .set_x_request_id(MyMakeRequestId::default())
        // Add high level tracing/logging to all requests
        .layer(
            TraceLayer::new_for_http()
                .on_request(DefaultOnRequest::new())
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Micros)
                        .include_headers(true),
                )
                .on_body_chunk(DefaultOnBodyChunk::new())
                .on_eos(DefaultOnEos::new().level(Level::INFO))
                .on_failure(DefaultOnFailure::new().level(Level::INFO)),
        )
        .sensitive_response_headers(sensitive_headers)
        // Set a timeout
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        // Compress responses
        .compression()
        .propagate_x_request_id()
        // Set a `Content-Type` if there isn't one already.
        .insert_response_header_if_not_present(
            CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );

    Router::new()
        .nest("/", routes)
        .fallback(error_404)
        .layer(middleware)
        .with_state(state)
}

#[derive(Clone, Default)]
struct MyMakeRequestId {
    counter: Arc<AtomicU64>,
}

use std::sync::atomic::Ordering;

impl MakeRequestId for MyMakeRequestId {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        self.counter
            .fetch_add(1, Ordering::SeqCst)
            .to_string()
            .parse()
            .ok()
            .map(RequestId::new)
    }
}

async fn error_404() -> Response {
    (StatusCode::NOT_FOUND, Html("<h1>Nothing to see here</h1>")).into_response()
}
