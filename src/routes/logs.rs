use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use crate::routes::plan::AppState;

#[derive(Deserialize)]
pub struct LogQuery {
    plan_id: Option<String>,
    phase_id: Option<String>,
    limit: Option<usize>,
}

/// Handler for GET /logs
pub async fn get_logs(
    Query(params): Query<LogQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if state.redis_client.is_none() {
        // Stateless mode
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "stateless",
                "message": "No persistent storage configured - logs are ephemeral in stateless mode",
                "logs": []
            }))
        )
    } else {
        match state.logging_service.get_logs(
            params.plan_id.as_deref(),
            params.phase_id.as_deref(),
            params.limit,
        ).await {
            Ok(logs) => {
                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "status": "ok",
                        "logs": logs,
                        "count": logs.len(),
                        "filters": {
                            "plan_id": params.plan_id,
                            "phase_id": params.phase_id,
                            "limit": params.limit.unwrap_or(100)
                        }
                    }))
                )
            }
            Err(e) => {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "status": "error",
                        "message": format!("Failed to retrieve logs: {}", e),
                        "logs": []
                    }))
                )
            }
        }
    }
}
