use axum::body::Body;
use http_body_util::BodyExt;
use axum::http::{Request, StatusCode};
use axum::Router;
use tower::ServiceExt; // for `oneshot`
use planter::routes::manifest::submit_manifest;
use planter::routes::plan::AppState;

#[tokio::test]
async fn test_manifest_endpoint_with_yaml() {
    // Check if Janet is available
    if std::process::Command::new("janet").arg("--version").output().is_err() {
        println!("Skipping test: Janet not available");
        return;
    }

    // Load Janet-compatible YAML manifest from file
    let yaml = include_str!("fixtures/test_manifest.yaml");

    let app_state = AppState { 
        redis_client: None,
        nats_client: None,
        logging_service: planter::log::LoggingService::new(None),
        tenant_key: "global".to_string(),
    };
    let app = Router::new()
        .route("/manifest", axum::routing::post(submit_manifest))
        .with_state(app_state);

    let request = Request::builder()
        .method("POST")
        .uri("/manifest")
        .header("content-type", "application/x-yaml")
        .body(Body::from(yaml))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    if status != StatusCode::OK {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        println!("Response body: {}", String::from_utf8_lossy(&bytes));
        panic!("/manifest endpoint failed with status {}", status);
    }
    // Optionally, check the body for expected keys
}

#[tokio::test]
async fn test_manifest_endpoint_without_janet() {
    // Test what happens when Janet is not available or fails
    let yaml = include_str!("fixtures/test_manifest.yaml");

    let app_state = AppState { 
        redis_client: None,
        nats_client: None,
        logging_service: planter::log::LoggingService::new(None),
        tenant_key: "global".to_string(),
    };
    let app = Router::new()
        .route("/manifest", axum::routing::post(submit_manifest))
        .with_state(app_state);

    let request = Request::builder()
        .method("POST")
        .uri("/manifest")
        .header("content-type", "application/x-yaml")
        .body(Body::from(yaml))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should get 500 status when Janet is not available
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
