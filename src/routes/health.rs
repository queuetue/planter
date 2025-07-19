use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

/// Handler for GET /health - Basic health check
pub async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "service": "planter",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    )
}

/// Handler for GET /ready - Readiness probe (Prometheus/Kubernetes compatible)
pub async fn readiness_check() -> impl IntoResponse {
    // In a real implementation, you might check:
    // - Database connectivity
    // - Required services availability
    // - Any initialization status
    (
        StatusCode::OK,
        Json(json!({
            "status": "ready",
            "checks": {
                "server": "ok",
                "redis": "optional"
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    )
}

/// Handler for GET /metrics - Prometheus metrics endpoint (placeholder)
pub async fn metrics() -> impl IntoResponse {
    // This is a basic Prometheus-compatible metrics endpoint
    // In production, you'd use a proper metrics library like `prometheus` crate
    let metrics_text = format!(
        "# HELP planter_build_info Build information
# TYPE planter_build_info gauge
planter_build_info{{version=\"{}\"}} 1

# HELP planter_uptime_seconds Time the process has been running in seconds
# TYPE planter_uptime_seconds counter
planter_uptime_seconds {}

# HELP planter_requests_total Total number of requests received
# TYPE planter_requests_total counter
planter_requests_total{{endpoint=\"/plan\"}} 0
planter_requests_total{{endpoint=\"/health\"}} 0
planter_requests_total{{endpoint=\"/ready\"}} 0
",
        env!("CARGO_PKG_VERSION"),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        metrics_text
    )
}
