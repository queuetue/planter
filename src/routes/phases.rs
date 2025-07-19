use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::routes::plan::AppState;

/// Handler for GET /phases/:id
pub async fn get_phase(
    Path(phase_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Get logs for this specific phase
    match state.logging_service.get_phase_logs(&phase_id).await {
        Ok(logs) => {
            let phase_info = if !logs.is_empty() {
                serde_json::json!({
                    "id": phase_id,
                    "status": "found",
                    "logs_count": logs.len(),
                    "logs": logs,
                    "last_activity": logs.first().map(|l| l.timestamp)
                })
            } else {
                serde_json::json!({
                    "id": phase_id,
                    "status": "not_found",
                    "logs_count": 0,
                    "logs": [],
                    "message": "No logs found for this phase"
                })
            };

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "ok",
                    "phase": phase_info
                }))
            )
        }
        Err(e) => {
            if state.redis_client.is_none() {
                // Stateless mode
                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "status": "stateless",
                        "phase_id": phase_id,
                        "message": "No persistent storage configured - phase history not available in stateless mode"
                    }))
                )
            } else {
                // Redis error
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "status": "error",
                        "phase_id": phase_id,
                        "message": format!("Failed to retrieve phase information: {}", e)
                    }))
                )
            }
        }
    }
}
