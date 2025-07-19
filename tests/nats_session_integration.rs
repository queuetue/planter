use std::time::Duration;
use tokio::time::timeout;
use tokio_stream::StreamExt;
use planter::nats::client::NatsClient;
use planter::model::{Phase, PhaseSpec, Selector};
use serde_json;

/// Helper to create a test phase
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

#[tokio::test]
async fn test_nats_session_start_message() {
    // Skip test if NATS_URL not configured
    let nats_url = match std::env::var("NATS_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("NATS_URL not set, skipping NATS integration test");
            return;
        }
    };

    // Connect to NATS
    let client = match NatsClient::connect(&nats_url).await {
        Ok(client) => client,
        Err(e) => {
            println!("Failed to connect to NATS: {}, skipping test", e);
            return;
        }
    };

    // Create a session
    let session = client.new_session();
    let session_id = session.session_id.clone();
    
    // Subscribe to start messages on this session
    let start_sub = session.subscribe_start().await.expect("Failed to subscribe to start");
    
    // Create test manifest
    let manifest = vec![
        create_test_phase("setup", "Initialize system"),
        create_test_phase("deploy", "Deploy application"),
    ];

    // Start the session
    session.start_session(manifest.clone(), false).await
        .expect("Failed to start session");

    // Wait for the start message
    let mut start_sub = start_sub;
    let received_msg = timeout(Duration::from_secs(5), start_sub.next()).await
        .expect("Timeout waiting for start message")
        .expect("Expected start message");

    // Parse the log message
    let received_data: serde_json::Value = serde_json::from_slice(&received_msg.payload)
        .expect("Failed to parse log message JSON");

    // Verify the message content
    assert_eq!(received_data["dryRun"], false);
    assert!(received_data["manifest"].is_array());
    let received_manifest = received_data["manifest"].as_array().unwrap();
    assert_eq!(received_manifest.len(), 2);
    
    // Check first phase
    assert_eq!(received_manifest[0]["id"], "setup");
    assert_eq!(received_manifest[0]["spec"]["description"], "Initialize system");
    
    // Check second phase  
    assert_eq!(received_manifest[1]["id"], "deploy");
    assert_eq!(received_manifest[1]["spec"]["description"], "Deploy application");

    println!("✓ NATS session start message test passed");
}

#[tokio::test]
async fn test_nats_session_control_message() {
    // Skip test if NATS_URL not configured
    let nats_url = match std::env::var("NATS_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("NATS_URL not set, skipping NATS integration test");
            return;
        }
    };

    // Connect to NATS
    let client = match NatsClient::connect(&nats_url).await {
        Ok(client) => client,
        Err(e) => {
            println!("Failed to connect to NATS: {}, skipping test", e);
            return;
        }
    };

    // Create a session
    let session = client.new_session();
    
    // Subscribe to control messages
    let control_sub = session.subscribe_control().await.expect("Failed to subscribe to control");
    
    // Send control command
    session.send_control("pause".to_string()).await
        .expect("Failed to send control command");

    // Wait for the control message
    let mut control_sub = control_sub;
    let received_msg = timeout(Duration::from_secs(5), control_sub.next()).await
        .expect("Timeout waiting for control message")
        .expect("Expected control message");

    // Parse the control message
    let received_data: serde_json::Value = serde_json::from_slice(&received_msg.payload)
        .expect("Failed to parse control message JSON");

    // Verify the message content
    assert_eq!(received_data["command"], "pause");

    println!("✓ NATS session control message test passed");
}

#[tokio::test]
async fn test_nats_session_state_and_log_publishing() {
    // Skip test if NATS_URL not configured
    let nats_url = match std::env::var("NATS_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("NATS_URL not set, skipping NATS integration test");
            return;
        }
    };

    // Connect to NATS
    let client = match NatsClient::connect(&nats_url).await {
        Ok(client) => client,
        Err(e) => {
            println!("Failed to connect to NATS: {}, skipping test", e);
            return;
        }
    };

    // Create a session
    let session = client.new_session();
    
    // Create another client to subscribe to state and log messages
    let sub_client = async_nats::connect(&nats_url).await
        .expect("Failed to connect subscriber client");
    
    let state_sub = sub_client.subscribe(session.state_subject()).await
        .expect("Failed to subscribe to state");
    let log_sub = sub_client.subscribe(session.log_subject()).await
        .expect("Failed to subscribe to log");
    
    // Publish state update
    session.publish_state("test-phase".to_string(), "running".to_string()).await
        .expect("Failed to publish state");
        
    // Publish log message
    session.publish_log(Some("test-phase".to_string()), "info".to_string(), "Phase started".to_string()).await
        .expect("Failed to publish log");

    // Wait for messages
    let mut state_sub = state_sub;
    let mut log_sub = log_sub;
    
    let state_msg = timeout(Duration::from_secs(5), state_sub.next()).await
        .expect("Timeout waiting for state message")
        .expect("Expected state message");
        
    let log_msg = timeout(Duration::from_secs(5), log_sub.next()).await
        .expect("Timeout waiting for log message") 
        .expect("Expected log message");

    // Parse state message
    let state_data: serde_json::Value = serde_json::from_slice(&state_msg.payload)
        .expect("Failed to parse state message JSON");
    assert_eq!(state_data["phaseId"], "test-phase");
    assert_eq!(state_data["status"], "running");
    assert!(state_data["updated"].is_string());

    // Parse log message
    let log_data: serde_json::Value = serde_json::from_slice(&log_msg.payload)
        .expect("Failed to parse log message JSON");
    assert_eq!(log_data["phaseId"], "test-phase");
    assert_eq!(log_data["level"], "info");
    assert_eq!(log_data["message"], "Phase started");
    assert!(log_data["timestamp"].is_string());

    println!("✓ NATS session state and log publishing test passed");
}
