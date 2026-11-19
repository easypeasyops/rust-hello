//! A minimal service with what the platform's service chart expects of an app:
//!
//! * port 8000: the app's routes, `/healthz` (liveness: the process runs) and `/readyz`
//!   (readiness: it can serve);
//! * port `METRICS_PORT` (9100): `/metrics` for Prometheus, on a port of its own so the
//!   public port carries none of it. `http_requests_total` and
//!   `http_request_duration_seconds` are the names the platform's SLO rules, canary analysis
//!   and dashboard read.
//!
//! Replace `hello` with the app; keep the probes and the metrics.

use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::extract::{MatchedPath, Request, State};
use axum::http::header::CONTENT_TYPE;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;
use serde_json::{Value, json};
use tokio::net::TcpListener;

/// The buckets Python's prometheus_client uses, so every app's histograms line up. An SLO's
/// latency threshold must be one of them (deploy/prometheus/slos.yaml).
const BUCKETS: [f64; 14] = [
    0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 7.5, 10.0,
];

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct RequestLabels {
    method: String,
    route: String,
    status: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct DurationLabels {
    method: String,
    route: String,
}

/// The request metrics and the registry `/metrics` encodes them from.
pub struct Metrics {
    registry: Mutex<Registry>,
    requests: Family<RequestLabels, Counter>,
    duration: Family<DurationLabels, Histogram>,
}

impl Default for Metrics {
    fn default() -> Self {
        let requests = Family::<RequestLabels, Counter>::default();
        let duration = Family::<DurationLabels, Histogram>::new_with_constructor(
            (|| Histogram::new(BUCKETS)) as fn() -> Histogram,
        );
        let mut registry = Registry::default();
        // The encoder adds `_total` to a counter's name: http_requests_total.
        registry.register("http_requests", "Requests served", requests.clone());
        registry.register(
            "http_request_duration_seconds",
            "Time to answer",
            duration.clone(),
        );
        Self {
            registry: Mutex::new(registry),
            requests,
            duration,
        }
    }
}

async fn hello() -> Json<Value> {
    Json(json!({"hello": "world"}))
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz() -> &'static str {
    // Answer 503 here while something the app can't serve without (its database, a broker)
    // is unreachable: the pod then gets no traffic, and is not restarted.
    "ok"
}

/// Counts and times every request, labelled by route, never by raw path: one series per ID
/// or per scanner's guess would grow without bound. A route with a path parameter is
/// labelled by its pattern (`/items/{id}`), a path no route matches as `unmatched`.
async fn track(State(metrics): State<Arc<Metrics>>, request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("unmatched", MatchedPath::as_str)
        .to_owned();
    let response = next.run(request).await;
    let labels = DurationLabels { method, route };
    metrics
        .duration
        .get_or_create(&labels)
        .observe(start.elapsed().as_secs_f64());
    metrics
        .requests
        .get_or_create(&RequestLabels {
            method: labels.method,
            route: labels.route,
            status: response.status().as_u16().to_string(),
        })
        .inc();
    response
}

/// The app's routes on port 8000, every request counted in `metrics`.
pub fn router(metrics: Arc<Metrics>) -> Router {
    Router::new()
        .route("/", get(hello))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        // `layer`, not `route_layer`: a path no route matches is counted too.
        .layer(middleware::from_fn_with_state(metrics, track))
}

async fn metrics_text(State(metrics): State<Arc<Metrics>>) -> impl IntoResponse {
    let mut body = String::new();
    let registry = metrics.registry.lock().expect("metrics registry lock");
    encode(&mut body, &registry).expect("writing to a String does not fail");
    (
        [(
            CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )],
        body,
    )
}

/// `/metrics` on the metrics port, for Prometheus only.
pub fn metrics_router(metrics: Arc<Metrics>) -> Router {
    Router::new()
        .route("/metrics", get(metrics_text))
        .with_state(metrics)
}

/// Serves the app and the metrics until `shutdown` completes, then finishes the requests
/// in hand on both ports and returns.
pub async fn serve(
    app_listener: TcpListener,
    metrics_listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let metrics = Arc::new(Metrics::default());
    let (stop, stopped) = tokio::sync::watch::channel(());
    let wait = |mut rx: tokio::sync::watch::Receiver<()>| async move {
        let _ = rx.changed().await;
    };
    let app = axum::serve(app_listener, router(metrics.clone()))
        .with_graceful_shutdown(wait(stopped.clone()));
    let metrics = axum::serve(metrics_listener, metrics_router(metrics))
        .with_graceful_shutdown(wait(stopped));
    tokio::spawn(async move {
        shutdown.await;
        let _ = stop.send(());
    });
    let (app, metrics) = tokio::join!(app.into_future(), metrics.into_future());
    app.and(metrics)
}
