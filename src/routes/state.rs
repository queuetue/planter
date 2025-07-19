use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::routes::plan::AppState;

/// Handler for GET /state
pub async fn get_state(
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Some(redis_client) = &state.redis_client {
        // Try to get the current plan from Redis
        match crate::state::tracker::load_current_plan(redis_client).await {
            Some(plan) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "ok",
                    "plan": plan,
                    "source": "redis"
                }))
            ),
            None => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "status": "not_found",
                    "message": "No plan currently stored"
                }))
            )
        }
    } else {
        // No Redis - stateless mode
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "stateless",
                "message": "No persistent storage configured - running in stateless mode",
                "plan": null
            }))
        )
    }
}
