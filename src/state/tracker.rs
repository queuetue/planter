use crate::config::state_file_path;
pub fn save_state_file(phases: &[Phase]) -> std::io::Result<()> {
    let path = state_file_path();
    let json = serde_json::to_string_pretty(phases)?;
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Phase, PhaseSpec, Selector};
    use std::collections::HashMap;
    use std::env;
    use tempfile::TempDir;

    fn sample_phases() -> Vec<Phase> {
        vec![Phase {
            kind: "TestKind".to_string(),
            id: "id1".to_string(),
            spec: PhaseSpec {
                description: "desc".to_string(),
                selector: Selector { match_labels: HashMap::new() },
                instance_mode: None,
                wait_for: None,
                retry: None,
                on_failure: None,
                on_success: None,
            },
        }]
    }

    #[test]
    fn test_save_and_load_state_file() {
        // Use a temporary directory for planter root
        let tmp = TempDir::new().expect("create temp dir");
        env::set_var("PLANTER_ROOT", tmp.path());
        // Save sample phases
        let phases = sample_phases();
        save_state_file(&phases).expect("save state");
        // Load back
        let loaded = load_state_file().expect("load state");
        assert_eq!(loaded, phases);
    }

    #[test]
    fn test_load_missing_file() {
        // Use a fresh temp dir without saving
        let tmp = TempDir::new().expect("create temp dir");
        env::set_var("PLANTER_ROOT", tmp.path());
        // No file exists yet
        let loaded = load_state_file();
        assert!(loaded.is_none());
    }
}

pub fn load_state_file() -> Option<Vec<Phase>> {
    let path = state_file_path();
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}
// pub async fn save_plan(phases: &[Phase]) -> Result<()>;
// pub async fn load_current_plan() -> Result<Vec<Phase>>;
// pub async fn get_phase(id: &str) -> Option<Phase>;
use crate::model::Phase;
use crate::state::redis::{RedisClient, get_json, set_json};

const PLAN_CURRENT_KEY: &str = "plan:current";
const PLAN_APPLIED_KEY: &str = "plan:applied";

pub async fn store_current_plan(client: &RedisClient, phases: &[Phase]) {
    // Namespace key by tenant
    let tenant = std::env::var("TENANT_KEY").unwrap_or_else(|_| "global".to_string());
    let key = format!("{}:{}", tenant, PLAN_CURRENT_KEY);
    if let Err(e) = set_json(client, &key, phases).await {
        eprintln!("Failed to store current plan: {e}");
    }
}

pub async fn store_applied_plan(client: &RedisClient, phases: &[Phase]) {
    // Namespace key by tenant
    let tenant = std::env::var("TENANT_KEY").unwrap_or_else(|_| "global".to_string());
    let key = format!("{}:{}", tenant, PLAN_APPLIED_KEY);
    if let Err(e) = set_json(client, &key, phases).await {
        eprintln!("Failed to store applied plan: {e}");
    }
}

pub async fn load_current_plan(client: &RedisClient) -> Option<Vec<Phase>> {
    // Namespace key by tenant
    let tenant = std::env::var("TENANT_KEY").unwrap_or_else(|_| "global".to_string());
    let key = format!("{}:{}", tenant, PLAN_CURRENT_KEY);
    get_json(client, &key).await.ok().flatten()
}

pub async fn load_applied_plan(client: &RedisClient) -> Option<Vec<Phase>> {
    // Namespace key by tenant
    let tenant = std::env::var("TENANT_KEY").unwrap_or_else(|_| "global".to_string());
    let key = format!("{}:{}", tenant, PLAN_APPLIED_KEY);
    get_json(client, &key).await.ok().flatten()
}
