use reqwest::Client;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::sleep;
use planter::routes::plan::AppState;
use planter::log::{init_logger, LoggingService};
use axum::{Router, routing::{get, post}};

async fn spawn_stateless_server() {
    init_logger();
    let app_state = AppState {
        redis_client: None,
        nats_client: None,
        logging_service: LoggingService::new(None),
        tenant_key: "global".to_string(),
    };
    let app = Router::new()
        .route("/plan", post(planter::routes::plan::submit_plan))
        .route("/logs", get(planter::routes::logs::get_logs))
        .with_state(app_state);
    let addr = SocketAddr::from(([127, 0, 0, 1], 38180));
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app.into_make_service()).await.unwrap();
    });
    sleep(Duration::from_millis(500)).await;
}

#[tokio::test]
async fn test_logs_stateless() {
    spawn_stateless_server().await;
    let client = Client::new();
    let base_url = "http://127.0.0.1:38180";

    // Submit a plan
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

    // /logs should return stateless response
    let res = client.get(&format!("{}/logs", base_url))
        .send()
        .await
        .expect("Failed to send /logs request");
    assert!(res.status().is_success(), "/logs: {}", res.status());
    let body: serde_json::Value = res.json().await.expect("Failed to parse /logs response");
    assert_eq!(body["status"], "stateless");
    assert!(body["logs"].as_array().unwrap().is_empty());
    assert!(body["message"].as_str().unwrap().contains("ephemeral"));
}
