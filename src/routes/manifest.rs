use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::Value;
use std::process::{Command, Stdio};
use crate::routes::plan::{submit_plan, AppState};

#[derive(Debug, serde::Deserialize)]
pub struct ManifestParams {
    pub dry_run: Option<bool>,
    pub validate_only: Option<bool>,
}

/// Handler for POST /manifest
pub async fn submit_manifest(
    State(state): State<AppState>,
    Query(params): Query<ManifestParams>,
    body: String,
) -> impl IntoResponse {
    // Create a temp directory and write plan.yaml for Janet
    let tmp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to create temp directory: {}", e)
                }))
            );
        }
    };
    
    let plan_path = tmp_dir.path().join("plan.yaml");
    if let Err(e) = std::fs::write(&plan_path, &body) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to write plan.yaml: {}", e)
            }))
        );
    }

    // Run Janet as a subprocess to render the manifest
    let mut cmd = Command::new("janet");
    cmd.arg("render");
    cmd.arg("-d");
    cmd.arg(tmp_dir.path());
    cmd.stdout(Stdio::piped());

    let output = match cmd.output() {
        Ok(output) => output,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to run Janet: {}", e)
                }))
            );
        }
    };

    if !output.status.success() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": String::from_utf8_lossy(&output.stderr)
            }))
        );
    }

    // Parse Janet's output as JSON
    let rendered: Value = match serde_json::from_slice(&output.stdout) {
        Ok(val) => val,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Janet output is not valid JSON: {}", e)
                }))
            );
        }
    };

    // Extract phases (assume top-level "phases" key or array)
    let phases = if let Some(phases) = rendered.get("phases") {
        phases.clone()
    } else if rendered.is_array() {
        rendered.clone()
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "No phases found in Janet output"
            }))
        );
    };

    // If validate_only, just return the phases
    if params.validate_only.unwrap_or(false) {
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "phases_extracted": phases.as_array().map(|a| a.len()).unwrap_or(0),
                "from": "janet",
                "phases": phases,
                "execution": { "validate_only": true }
            }))
        );
    }

    // Forward to /plan internally
    // Note: This is a direct function call, not an HTTP request
    let plan_json = match serde_json::from_value::<Vec<crate::model::Phase>>(phases.clone()) {
        Ok(plan) => plan,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Phases are not valid: {}", e)
                }))
            );
        }
    };

    // Use dry_run param if present
    // (You can extend submit_plan to accept dry_run if needed)
    let _plan_response = submit_plan(State(state.clone()), Json(plan_json)).await.into_response();
    // Instead of serializing the full response, just indicate success and timestamp
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "phases_extracted": phases.as_array().map(|a| a.len()).unwrap_or(0),
            "from": "janet",
            "execution": {
                "dry_run": params.dry_run.unwrap_or(false),
                "started_at": chrono::Utc::now().to_rfc3339(),
            },
            "plan_response": "forwarded to /plan"
        }))
    )
}
