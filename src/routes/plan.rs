use axum::{
    extract::{Json, State},
    response::IntoResponse,
    http::StatusCode,
};
use serde_json::json;
use std::sync::Arc;

use crate::log::{log_event, Event, LoggingService};
use crate::model::Phase;
use crate::executor::execute_plan;
use crate::state::redis::RedisClient;
use crate::diff::{diff_plans, DiffResult};
use crate::state::tracker::{load_applied_plan, store_current_plan};
use crate::nats::client::NatsClient;

#[allow(dead_code)]
#[derive(Clone)]
pub struct AppState {
    pub redis_client: Option<Arc<RedisClient>>,
    pub nats_client: Option<Arc<NatsClient>>,
    pub logging_service: LoggingService,
    /// Tenant namespace key derived from prefix
    pub tenant_key: String,
}

/// Handler for POST /plan
pub async fn submit_plan(
    State(state): State<AppState>,
    Json(phases): Json<Vec<Phase>>,
) -> impl IntoResponse {
    let plan_id = uuid::Uuid::new_v4().to_string();
    
    // Log plan submission
    log_event(Event::PhaseReceived(format!("Received plan with {} phases", phases.len())));
    let _ = state.logging_service.log_event_with_context(
        Event::PlanSubmitted { 
            plan_id: plan_id.clone(), 
            phases_count: phases.len() 
        },
        Some(plan_id.clone()),
        None,
        std::collections::HashMap::new(),
    ).await;

    // Log received phases
    for phase in &phases {
        println!("- {}: {}", phase.id, phase.spec.description);
        let mut context = std::collections::HashMap::new();
        context.insert("description".to_string(), phase.spec.description.clone());
        
        let _ = state.logging_service.log_event_with_context(
            Event::PhaseReceived(format!("Phase: {}", phase.id)),
            Some(plan_id.clone()),
            Some(phase.id.clone()),
            context,
        ).await;
    }

    // If NATS is configured, dispatch plan as a session
    if let Some(nats_client) = &state.nats_client {
        // Create a new session and start it
        let session = nats_client.new_session();
        if let Err(e) = session.start_session(phases.clone(), false).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("NATS session start failed: {}", e) }))).into_response();
        }
        // Return accepted with session ID
        return (StatusCode::ACCEPTED, Json(json!({"sessionId": session.session_id}))).into_response();
    }

    // Fallback: if Redis is available, compute diff and execute
    if let Some(redis_client) = &state.redis_client {
        // Load the previously applied plan for diffing
        let previous_plan = load_applied_plan(redis_client).await.unwrap_or_default();
        
        // Compute diff
        let diff = diff_plans(&previous_plan, &phases);
        
        // Log diff results
        let mut diff_changes = Vec::new();
        if !diff.is_empty() {
            println!("Plan differences detected:");
            for change in &diff {
                match change {
                    DiffResult::Add(phase) => {
                        let change_desc = format!("Add: {} ({})", phase.id, phase.spec.description);
                        println!("  + {}", change_desc);
                        diff_changes.push(change_desc);
                    }
                    DiffResult::Update { old, new } => {
                        let change_desc = format!("Update: {} ({} -> {})", 
                               new.id, old.spec.description, new.spec.description);
                        println!("  ~ {}", change_desc);
                        diff_changes.push(change_desc);
                    }
                    DiffResult::Delete(phase) => {
                        let change_desc = format!("Delete: {} ({})", phase.id, phase.spec.description);
                        println!("  - {}", change_desc);
                        diff_changes.push(change_desc);
                    }
                }
            }
        } else {
            println!("No changes detected in plan");
        }

        // Log diff computation
        let _ = state.logging_service.log_event_with_context(
            Event::DiffResult { 
                plan_id: plan_id.clone(), 
                changes: diff_changes.clone() 
            },
            Some(plan_id.clone()),
            None,
            std::collections::HashMap::new(),
        ).await;

        // Store current plan
        store_current_plan(redis_client, &phases).await;

        // Execute the plan
        execute_plan(redis_client, &phases).await;

        (StatusCode::OK, Json(json!({
            "status": "success",
            "message": "Plan received and executed",
            "plan_id": plan_id,
            "phases_count": phases.len(),
            "changes_count": diff.len(),
            "changes": diff_changes
        }))).into_response()
    } else {
        // No Redis - just simulate execution
        println!("No Redis configured - simulating execution");
        
        for phase in &phases {
            println!("Simulating execution of phase: {}", phase.id);
            let mut context = std::collections::HashMap::new();
            context.insert("mode".to_string(), "simulation".to_string());
            
            let _ = state.logging_service.log_event_with_context(
                Event::PhaseExecuted { 
                    id: phase.id.clone(), 
                    success: true 
                },
                Some(plan_id.clone()),
                Some(phase.id.clone()),
                context,
            ).await;
        }

        (StatusCode::OK, Json(json!({
            "status": "success",
            "message": "Plan received and simulated",
            "plan_id": plan_id,
            "phases_count": phases.len()
        }))).into_response()
    }
}
