use reqwest::Client;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::sleep;
use std::fs;
use std::process::{Command, Child};
use std::thread;
use std::env;

// Real binary shutdown/reload test is ignored in CI environment
#[tokio::test]
#[ignore]
async fn test_shutdown_and_reload_real() {
    // Clean up any old state file
    let state_path = planter::config::state_file_path();
    let _ = fs::remove_file(&state_path);
    let port = 38480;
    let addr = format!("127.0.0.1:{}", port);

    // Launch the real binary as a subprocess
    let mut child = Command::new(env::current_exe().unwrap())
        .arg("run")
        .env("PORT", port.to_string())
        .env("PLANTER_ROOT", ".")
        .spawn()
        .expect("Failed to start planter binary");

    // Wait for server to start
    sleep(Duration::from_secs(2)).await;
    let client = Client::new();
    let base_url = format!("http://{}", addr);

    // Submit a plan
    let plan = serde_json::json!([
        {
            "Kind": "Phase",
            "Id": "shutdown-test-real",
            "Spec": {
                "description": "Shutdown test phase real",
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

    // Wait for process to exit
    let _ = child.wait();

    // State file should exist
    assert!(state_path.exists(), "State file should exist after shutdown");
    let state_data = fs::read_to_string(&state_path).expect("Should read state file");
    assert!(state_data.contains("shutdown-test-real"));
}
