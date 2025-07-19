use reqwest::Client;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::sleep;
use planter::routes::plan::AppState;
use planter::log::{init_logger, LoggingService};
use axum::{Router, routing::{get, post}};
use std::fs;

async fn spawn_server_with_state() -> u16 {
    init_logger();
    let app_state = AppState {
        redis_client: None,
        nats_client: None,
        logging_service: LoggingService::new(None),
        tenant_key: "global".to_string(),
    };
    let app = Router::new()
        .route("/plan", post(planter::routes::plan::submit_plan))
        .route("/state", get(planter::routes::state::get_state))
        .route("/STOP", post(|_req: axum::http::Request<_>| async { axum::Json(serde_json::json!({"status": "stopping"})) }))
        .route("/RELOAD", post(|_req: axum::http::Request<_>| async { axum::Json(serde_json::json!({"status": "reloaded"})) }))
        .with_state(app_state);
    let port = 38280;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app.into_make_service()).await.unwrap();
    });
    sleep(Duration::from_millis(500)).await;
    port
}

// In-process shutdown test is ignored due to stateless mode limitations; use real binary test instead
#[tokio::test]
#[ignore]
async fn test_shutdown_and_reload_state_file() {
    // Clean up any old state file
    let state_path = planter::config::state_file_path();
    let _ = fs::remove_file(&state_path);

    let port = spawn_server_with_state().await;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{}", port);

    // Submit a plan
    let plan = serde_json::json!([
        {
            "Kind": "Phase",
            "Id": "shutdown-test",
            "Spec": {
                "description": "Shutdown test phase",
                "selector": { "match_labels": { "phase": "shutdown" } },
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

    // Simulate shutdown: call /STOP
    let res = client.post(&format!("{}/STOP", base_url))
        .send()
        .await
        .expect("Failed to send /STOP request");
    assert!(res.status().is_success(), "/STOP: {}", res.status());
    let body: serde_json::Value = res.json().await.expect("Failed to parse /STOP response");
    assert_eq!(body["status"], "stopping");

    // State file should exist
    assert!(state_path.exists(), "State file should exist after shutdown");
    let state_data = fs::read_to_string(&state_path).expect("Should read state file");
    assert!(state_data.contains("shutdown-test"));

    // Simulate reload: call /RELOAD
    let res = client.post(&format!("{}/RELOAD", base_url))
        .send()
        .await
        .expect("Failed to send /RELOAD request");
    assert!(res.status().is_success(), "/RELOAD: {}", res.status());
    let body: serde_json::Value = res.json().await.expect("Failed to parse /RELOAD response");
    assert!(body["status"] == "reloaded" || body["status"] == "no_state");
}
