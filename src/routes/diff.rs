use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use crate::routes::plan::AppState;
use crate::state::tracker::load_applied_plan;
use crate::diff::{diff_plans, DiffResult};

#[derive(Deserialize)]
pub struct DiffQuery {
    plan_id: Option<String>,
}

/// Handler for GET /diff
pub async fn get_diff(
    Query(params): Query<DiffQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Some(redis_client) = &state.redis_client {
        // Load the previously applied plan for diffing
        let previous_plan = load_applied_plan(redis_client).await.unwrap_or_default();
        
        // For now, we can only diff against the current plan in Redis
        // In a full implementation, we'd accept a plan in the request body
        // or allow specifying which plans to compare
        
        if previous_plan.is_empty() {
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "no_baseline",
                    "message": "No baseline plan found for comparison",
                    "diff": null
                }))
            );
        }

        // For demonstration, let's show what a diff would look like
        // In practice, this endpoint would accept a new plan to compare against
        let current_plan = load_applied_plan(redis_client).await.unwrap_or_default();
        let diff = diff_plans(&previous_plan, &current_plan);
        
        let mut diff_details = Vec::new();
        let mut adds = 0;
        let mut updates = 0; 
        let mut deletes = 0;
        
        for change in &diff {
            match change {
                DiffResult::Add(phase) => {
                    adds += 1;
                    diff_details.push(serde_json::json!({
                        "type": "add",
                        "phase_id": phase.id,
                        "description": phase.spec.description
                    }));
                }
                DiffResult::Update { old, new } => {
                    updates += 1;
                    diff_details.push(serde_json::json!({
                        "type": "update",
                        "phase_id": new.id,
                        "old_description": old.spec.description,
                        "new_description": new.spec.description
                    }));
                }
                DiffResult::Delete(phase) => {
                    deletes += 1;
                    diff_details.push(serde_json::json!({
                        "type": "delete", 
                        "phase_id": phase.id,
                        "description": phase.spec.description
                    }));
                }
            }
        }

        // Log the diff computation
        let _ = state.logging_service.log_event_with_context(
            crate::log::Event::DiffComputed { adds, updates, deletes },
            params.plan_id,
            None,
            std::collections::HashMap::new(),
        ).await;

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "diff": {
                    "summary": {
                        "adds": adds,
                        "updates": updates,
                        "deletes": deletes,
                        "total_changes": adds + updates + deletes
                    },
                    "changes": diff_details
                },
                "baseline_phases": previous_plan.len(),
                "current_phases": current_plan.len()
            }))
        )
    } else {
        // No Redis - diff requires stored state
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "stateless",
                "message": "No persistent storage configured - diff requires stored state",
                "diff": null
            }))
        )
    }
}
