use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
    routing::post,
};
use planter::{
    model::*,
    routes::plan::{submit_plan, AppState},
};
use planter::log::LoggingService;
use tower::ServiceExt;

fn create_test_app() -> Router {
    let app_state = AppState {
        redis_client: None,
        nats_client: None,
        logging_service: LoggingService::new(None),
        tenant_key: "global".to_string(),
    };
    Router::new()
        .route("/plan", post(submit_plan))
        .with_state(app_state)
}

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
async fn test_submit_plan_basic() {
    let app = create_test_app();

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
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_submit_plan_complex() {
    let app = create_test_app();

    let complex_phase = Phase {
        kind: "Phase".to_string(),
        id: "complex-phase".to_string(),
        spec: PhaseSpec {
            description: "A complex phase with all features".to_string(),
            selector: Selector {
                match_labels: [
                    ("phase".to_string(), "complex".to_string()),
                    ("env".to_string(), "test".to_string()),
                ]
                .iter()
                .cloned()
                .collect(),
            },
            instance_mode: Some("parallel".to_string()),
            wait_for: Some(WaitFor {
                phases: vec!["initialization".to_string(), "preflight".to_string()],
                timeout: Some("30s".to_string()),
            }),
            retry: Some(Retry {
                max_attempts: Some(3),
            }),
            on_failure: Some(Handler {
                action: Some("continue".to_string()),
                spec: Some(HandlerSpec {
                    message: vec!["Phase failed, continuing with defaults".to_string()],
                    notify: Some(Notify {
                        email: Some("admin@example.com".to_string()),
                        slack: Some("#alerts".to_string()),
                    }),
                    labels: Some([("status".to_string(), "failed".to_string())]
                        .iter()
                        .cloned()
                        .collect()),
                }),
            }),
            on_success: Some(Handler {
                action: Some("log".to_string()),
                spec: Some(HandlerSpec {
                    message: vec!["Phase completed successfully".to_string()],
                    notify: None,
                    labels: Some([("status".to_string(), "success".to_string())]
                        .iter()
                        .cloned()
                        .collect()),
                }),
            }),
        },
    };

    let phases = vec![complex_phase];

    let request = Request::builder()
        .method("POST")
        .uri("/plan")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&phases).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_submit_plan_empty() {
    let app = create_test_app();

    let phases: Vec<Phase> = vec![];

    let request = Request::builder()
        .method("POST")
        .uri("/plan")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&phases).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_submit_plan_invalid_json() {
    let app = create_test_app();

    let request = Request::builder()
        .method("POST")
        .uri("/plan")
        .header("content-type", "application/json")
        .body(Body::from("invalid json"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
