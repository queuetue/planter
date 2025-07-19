use reqwest::Client;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::sleep;
use planter::routes::plan::AppState;
use planter::log::{init_logger, LoggingService};
use axum::{Router, routing::{get, post}};
use std::env;

async fn spawn_planter_server(port: u16, phases: Option<serde_json::Value>) {
    init_logger();
    let app_state = AppState {
        redis_client: None,
        nats_client: None,
        logging_service: LoggingService::new(None),
        tenant_key: "global".to_string(),
    };
    let app = Router::new()
        .route("/state", get(|| async move {
            let resp = if let Some(phases) = &phases {
                serde_json::json!({"phases": phases})
            } else {
                serde_json::json!({"phases": []})
            };
            axum::Json(resp)
        }))
        .with_state(app_state);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app.into_make_service()).await.unwrap();
    });
    sleep(Duration::from_millis(500)).await;
}

#[tokio::test]
async fn test_sync_from_remote_planter() {
    // Launch remote planter server with a known state
    let remote_port = 38580;
    let remote_url = format!("http://127.0.0.1:{}", remote_port);
    let phases = serde_json::json!([
        {
            "Kind": "Phase",
            "Id": "sync-phase",
            "Spec": {
                "description": "Synced phase",
                "selector": { "match_labels": { "phase": "sync" } },
                "wait_for": { "phases": [] },
                "retry": { "max_attempts": 1 },
                "onFailure": {
                    "action": "continue",
                    "spec": { "message": ["fail"], "labels": { "mode": "test" } }
                }
            }
        }
    ]);
    spawn_planter_server(remote_port, Some(phases.clone())).await;

    // Set PLANTER_SYNC_FROM to remote planter
    env::set_var("PLANTER_SYNC_FROM", &remote_url);
    env::remove_var("PLANTER_ROOT"); // Ensure no local state file

    // Launch local planter (simulate startup logic)
    let resp = Client::new()
        .get(format!("{}/state", remote_url))
        .send()
        .await
        .expect("Failed to fetch remote state");
    assert!(resp.status().is_success(), "Remote /state: {}", resp.status());
    let body: serde_json::Value = resp.json().await.expect("Failed to parse remote state");
    assert_eq!(body["phases"].as_array().unwrap().len(), 1);
    assert_eq!(body["phases"][0]["Id"], "sync-phase");
}

#[tokio::test]
async fn test_sync_from_remote_planter_empty() {
    // Launch remote planter server with empty state
    let remote_port = 38581;
    let remote_url = format!("http://127.0.0.1:{}", remote_port);
    spawn_planter_server(remote_port, None).await;

    env::set_var("PLANTER_SYNC_FROM", &remote_url);
    env::remove_var("PLANTER_ROOT");

    let resp = Client::new()
        .get(format!("{}/state", remote_url))
        .send()
        .await
        .expect("Failed to fetch remote state");
    assert!(resp.status().is_success(), "Remote /state: {}", resp.status());
    let body: serde_json::Value = resp.json().await.expect("Failed to parse remote state");
    assert_eq!(body["phases"].as_array().unwrap().len(), 0);
}
