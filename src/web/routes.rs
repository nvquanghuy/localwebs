use crate::incremental_scanner::IncrementalScanner;
use crate::models::ServiceInfo;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub scanner: IncrementalScanner,
}

#[derive(Serialize)]
pub struct ScanResponse {
    pub services: Vec<ServiceInfo>,
    pub scan_type: String,
    pub duration_ms: u64,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/api/services", get(list_services))
        .route("/api/scan", post(trigger_scan))
        .route("/api/stats", get(get_stats))
        .route("/assets/styles.css", get(serve_styles))
        .route("/assets/app.js", get(serve_app_js))
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(state))
}

pub async fn start_background_scanner(state: Arc<AppState>, interval_secs: u64) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

    loop {
        interval.tick().await;

        let start = Instant::now();
        let services = state.scanner.scan().await;
        let duration = start.elapsed();

        println!(
            "📡 Incremental scan complete: {} services in {}ms",
            services.len(),
            duration.as_millis()
        );
    }
}

async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../ui/index.html"))
}

async fn serve_styles() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/css")],
        include_str!("../ui/styles.css"),
    )
}

async fn serve_app_js() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "application/javascript")],
        include_str!("../ui/app.js"),
    )
}

async fn list_services(State(state): State<Arc<AppState>>) -> Json<Vec<ServiceInfo>> {
    // Return current services (from last incremental scan)
    let services = state.scanner.get_cached().await;
    Json(services)
}

async fn trigger_scan(State(state): State<Arc<AppState>>) -> Json<ScanResponse> {
    // Force a full refresh scan
    let start = Instant::now();
    let services = state.scanner.force_full_scan().await;
    let duration = start.elapsed();

    Json(ScanResponse {
        services,
        scan_type: "full".to_string(),
        duration_ms: duration.as_millis() as u64,
    })
}

async fn get_stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let stats = state.scanner.get_stats().await;

    Json(serde_json::json!({
        "service_count": stats.service_count,
        "last_full_scan_seconds_ago": stats.last_full_scan.elapsed().as_secs(),
        "next_full_scan_in_seconds": stats.time_until_full_scan.as_secs(),
    }))
}
