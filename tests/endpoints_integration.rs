use reqwest::Client;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::sleep;

// Import the app construction from main.rs
use planter::routes::plan::AppState;
use planter::log::{init_logger, LoggingService};
use axum::{Router, routing::{get, post}};

async fn spawn_server() {
    init_logger();
    let app_state = AppState { 
        redis_client: None,
        nats_client: None,
        logging_service: planter::log::LoggingService::new(None),
        tenant_key: "global".to_string(),
    };
    // Get prefix from environment variable for test
    let prefix = std::env::var("PLANTER_PREFIX").unwrap_or_else(|_| "".to_string());
    let prefix = prefix.trim_end_matches('/');
    let route = |path: &str| {
        if prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", prefix, path.trim_start_matches('/'))
        }
    };
    let app = Router::new()
        .route(&route("/plan"), post(planter::routes::plan::submit_plan))
        .route(&route("/manifest"), post(planter::routes::manifest::submit_manifest))
        .route(&route("/state"), get(planter::routes::state::get_state))
        .route(&route("/diff"), get(planter::routes::diff::get_diff))
        .route(&route("/logs"), get(planter::routes::logs::get_logs))
        .route(&route("/phases/:id"), get(planter::routes::phases::get_phase))
        .route(&route("/apply"), post(planter::routes::apply::apply_plan))
        .route(&route("/health"), get(planter::routes::health::health_check))
        .route(&route("/ready"), get(planter::routes::health::readiness_check))
        .route(&route("/metrics"), get(planter::routes::health::metrics))
        .with_state(app_state);
    let addr = SocketAddr::from(([127, 0, 0, 1], 38080));
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });
    // Wait for server to be ready
    sleep(Duration::from_millis(500)).await;
}

#[tokio::test]
async fn test_all_endpoints() {
    spawn_server().await;
    // Use prefix in test URLs if set
    let prefix = std::env::var("PLANTER_PREFIX").unwrap_or_else(|_| "".to_string());
    let prefix = prefix.trim_end_matches('/');
    let base_url = if prefix.is_empty() {
        "http://127.0.0.1:38080".to_string()
    } else {
        format!("http://127.0.0.1:38080/{}", prefix.trim_start_matches('/'))
    };
    let client = Client::new();

    // /plan
    let plan = serde_json::json!([
        {
            "Kind": "Phase",
            "Id": "test-phase",
            "Spec": {
                "description": "Test phase",
                "selector": { "match_labels": { "phase": "test" } },
                "wait_for": { "phases": [] },
                "retry": { "max_attempts": 1 },
                "onFailure": {
                    "action": "continue",
                    "spec": { "message": ["fail"], "labels": { "mode": "test" } }
                }
            }
        }
    ]);
    let res = client.post(&format!("{}/plan", base_url))
        .json(&plan)
        .send()
        .await
        .expect("Failed to send /plan request");
    assert!(res.status().is_success(), "/plan: {}", res.status());

    // /state
    let res = client.get(&format!("{}/state", base_url))
        .send()
        .await
        .expect("Failed to send /state request");
    assert!(res.status().is_success(), "/state: {}", res.status());

    // /diff
    let res = client.get(&format!("{}/diff", base_url))
        .send()
        .await
        .expect("Failed to send /diff request");
    assert!(res.status().is_success(), "/diff: {}", res.status());

    // /logs
    let res = client.get(&format!("{}/logs", base_url))
        .send()
        .await
        .expect("Failed to send /logs request");
    assert!(res.status().is_success(), "/logs: {}", res.status());

    // /phases/:id
    let res = client.get(&format!("{}/phases/test-phase", base_url))
        .send()
        .await
        .expect("Failed to send /phases/:id request");
    assert!(res.status().is_success(), "/phases/:id: {}", res.status());

    // /apply
    let res = client.post(&format!("{}/apply", base_url))
        .send()
        .await
        .expect("Failed to send /apply request");
    assert!(res.status().is_success(), "/apply: {}", res.status());

    // /health
    let res = client.get(&format!("{}/health", base_url))
        .send()
        .await
        .expect("Failed to send /health request");
    assert!(res.status().is_success(), "/health: {}", res.status());

    // /ready
    let res = client.get(&format!("{}/ready", base_url))
        .send()
        .await
        .expect("Failed to send /ready request");
    assert!(res.status().is_success(), "/ready: {}", res.status());

    // /metrics
    let res = client.get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to send /metrics request");
    assert!(res.status().is_success(), "/metrics: {}", res.status());
    let content_type = res.headers().get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(content_type.contains("text/plain"), "Metrics should return text/plain content type");
}
