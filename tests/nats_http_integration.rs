use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
    routing::post,
};
use planter::{
model::{Phase, PhaseSpec, Selector},
routes::plan::{submit_plan, AppState},
nats::client::NatsClient,
log::LoggingService,
};
use tower::ServiceExt;
use std::sync::Arc;
use tokio::time::{sleep, timeout, Duration};
use tokio_stream::StreamExt;
use serde_json;

fn create_test_phase(id: &str, description: &str) -> Phase {
    Phase {
        kind: "Phase".to_string(),
        id: id.to_string(),
        spec: PhaseSpec {
            description: description.to_string(),
            selector: Selector {
                match_labels: [("phase".to_string(), id.to_string())]
                    .iter()
                    .cloned()
                    .collect(),
            },
            instance_mode: None,
            wait_for: None,
            retry: None,
            on_failure: None,
            on_success: None,
        },
    }
}

async fn create_test_app_with_nats() -> Option<Router> {
    // Skip test if NATS_URL not configured
    let nats_url = match std::env::var("NATS_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("NATS_URL not set, skipping NATS integration test");
            return None;
        }
    };

    // Connect to NATS
    let nats_client = match NatsClient::connect(&nats_url).await {
        Ok(client) => Some(Arc::new(client)),
        Err(e) => {
            println!("Failed to connect to NATS: {}, skipping test", e);
            return None;
        }
    };

    let app_state = AppState {
        redis_client: None,
        nats_client,
        logging_service: LoggingService::new(None),
        tenant_key: "global".to_string(),
    };

    Some(Router::new()
        .route("/plan", post(submit_plan))
        .with_state(app_state))
}

#[tokio::test]
async fn test_plan_endpoint_creates_nats_session() {
    let app = match create_test_app_with_nats().await {
        Some(app) => app,
        None => return, // Skip test if NATS not available
    };

    // Get NATS URL for direct subscription
    let nats_url = std::env::var("NATS_URL").unwrap();
    let sub_client = async_nats::connect(&nats_url).await
        .expect("Failed to connect subscriber client");

    // Subscribe to all session start messages
    let mut start_sub = sub_client.subscribe("plan.session.*.start").await
        .expect("Failed to subscribe to session starts");

    let phases = vec![
        create_test_phase("setup", "Initialize system"),
        create_test_phase("deploy", "Deploy application"),
    ];

    let request = Request::builder()
        .method("POST")
        .uri("/plan")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&phases).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // Should return ACCEPTED with session ID when NATS is configured
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_data: serde_json::Value = serde_json::from_slice(&body_bytes)
        .expect("Failed to parse response JSON");
    
    let session_id = response_data["sessionId"].as_str()
        .expect("Response should contain sessionId");
    assert!(session_id.starts_with("session-"));

    // Wait for the start message on NATS
    let start_msg = timeout(Duration::from_secs(5), start_sub.next()).await
        .expect("Timeout waiting for start message")
        .expect("Expected start message");

    // Verify the subject matches our session
    let expected_subject = format!("plan.session.{}.start", session_id);
    assert_eq!(start_msg.subject, expected_subject.into());

    // Parse the message content
    let msg_data: serde_json::Value = serde_json::from_slice(&start_msg.payload)
        .expect("Failed to parse start message JSON");

    // Verify the manifest was sent correctly
    assert_eq!(msg_data["dryRun"], false);
    let manifest = msg_data["manifest"].as_array().unwrap();
    assert_eq!(manifest.len(), 2);
    assert_eq!(manifest[0]["id"], "setup");
    assert_eq!(manifest[1]["id"], "deploy");

    println!("✓ Plan endpoint NATS session creation test passed");
}

#[tokio::test]
async fn test_full_session_workflow_with_mock_peer() {
    let app = match create_test_app_with_nats().await {
        Some(app) => app,
        None => return, // Skip test if NATS not available
    };

    // Get NATS URL for direct subscription
    let nats_url = std::env::var("NATS_URL").unwrap();
    let peer_client = async_nats::connect(&nats_url).await
        .expect("Failed to connect peer client");

    // Submit a plan via HTTP
    let phases = vec![
        create_test_phase("setup", "Initialize system"),
    ];

    let request = Request::builder()
        .method("POST")
        .uri("/plan")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&phases).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_data: serde_json::Value = serde_json::from_slice(&body_bytes)
        .expect("Failed to parse response JSON");
    
    let session_id = response_data["sessionId"].as_str()
        .expect("Response should contain sessionId");

    // Subscribe to the session's subjects
    let start_subject = format!("plan.session.{}.start", session_id);
    let state_subject = format!("plan.session.{}.state", session_id);
    let log_subject = format!("plan.session.{}.log", session_id);

    let mut start_sub = peer_client.subscribe(start_subject.clone()).await.unwrap();
    let state_sub = peer_client.subscribe(state_subject.clone()).await.unwrap();
    let log_sub = peer_client.subscribe(log_subject.clone()).await.unwrap();

    // Wait for start message
    let start_msg = timeout(Duration::from_secs(5), start_sub.next()).await
        .expect("Timeout waiting for start message")
        .expect("Expected start message");

    let start_data: serde_json::Value = serde_json::from_slice(&start_msg.payload).unwrap();
    let manifest = start_data["manifest"].as_array().unwrap();
    
    // Simulate peer processing: send state and log updates
    for phase in manifest {
        let phase_id = phase["id"].as_str().unwrap();
        
        // Send running state
        peer_client.publish(state_subject.clone(), serde_json::to_vec(&serde_json::json!({
            "phaseId": phase_id,
            "status": "running",
            "updated": chrono::Utc::now().to_rfc3339()
        })).unwrap().into()).await.unwrap();
        
        // Send log message
        peer_client.publish(log_subject.clone(), serde_json::to_vec(&serde_json::json!({
            "phaseId": phase_id,
            "level": "info", 
            "message": format!("Processing phase {}", phase_id),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })).unwrap().into()).await.unwrap();
        
        sleep(Duration::from_millis(100)).await;
        
        // Send complete state
        peer_client.publish(state_subject.clone(), serde_json::to_vec(&serde_json::json!({
            "phaseId": phase_id,
            "status": "complete",
            "updated": chrono::Utc::now().to_rfc3339()
        })).unwrap().into()).await.unwrap();
    }

    // Give time for messages to propagate
    sleep(Duration::from_millis(500)).await;

    println!("✓ Full session workflow test passed");
}
