use crate::config::Config;
use crate::detector::ServiceDetector;
use crate::models::ServiceInfo;
use crate::scanner::PortScanner;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/api/services", get(list_services))
        .route("/api/scan", post(trigger_scan))
        .route("/assets/styles.css", get(serve_styles))
        .route("/assets/app.js", get(serve_app_js))
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(state))
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
    let services = scan_and_detect(&state.config).await;
    Json(services)
}

async fn trigger_scan(State(state): State<Arc<AppState>>) -> Json<Vec<ServiceInfo>> {
    let services = scan_and_detect(&state.config).await;
    Json(services)
}

async fn scan_and_detect(config: &Config) -> Vec<ServiceInfo> {
    let scanner = PortScanner::new(
        config.portal.scan_timeout_ms,
        config.scan_ranges.ports.clone(),
    );

    let open_ports = scanner.scan().await;

    let detector = ServiceDetector::new(config.clone(), config.portal.probe_timeout_ms);

    let mut services = Vec::new();
    for port in open_ports {
        let service = detector.identify(port).await;
        services.push(service);
    }

    // Sort by port number
    services.sort_by_key(|s| s.port);

    services
}
