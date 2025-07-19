use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::state::redis::{RedisClient, get_json, set_json};
use crate::log::Event;

const LOGS_KEY_PREFIX: &str = "logs:";
const LOGS_INDEX_KEY: &str = "logs:index";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event: Event,
    pub plan_id: Option<String>,
    pub phase_id: Option<String>,
    pub context: std::collections::HashMap<String, String>,
}

impl LogEntry {
    pub fn new(event: Event) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event,
            plan_id: None,
            phase_id: None,
            context: std::collections::HashMap::new(),
        }
    }

    pub fn with_plan_id(mut self, plan_id: String) -> Self {
        self.plan_id = Some(plan_id);
        self
    }

    pub fn with_phase_id(mut self, phase_id: String) -> Self {
        self.phase_id = Some(phase_id);
        self
    }

    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.context.insert(key, value);
        self
    }
}

#[derive(Clone)]
pub struct LogStorage {
    client: RedisClient,
}

impl LogStorage {
    pub fn new(client: RedisClient) -> Self {
        Self { client }
    }

    /// Store a log entry in Redis
    pub async fn store_log(&self, entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Namespace log key by tenant
        let tenant = std::env::var("TENANT_KEY").unwrap_or_else(|_| "global".to_string());
        let key = format!("{}:{}{}", tenant, LOGS_KEY_PREFIX, entry.id);
        
        // Store the log entry
        set_json(&self.client, &key, &entry).await?;
        
        // Add to index for chronological access
        self.add_to_index(&entry.id, &entry.timestamp).await?;
        
        Ok(())
    }

    /// Retrieve logs with optional filtering
    pub async fn get_logs(
        &self,
        plan_id: Option<&str>,
        phase_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<LogEntry>, Box<dyn std::error::Error + Send + Sync>> {
        // Get log IDs from index (most recent first)
        let log_ids: Vec<String> = self.get_log_ids_from_index(limit.unwrap_or(100)).await?;
        
        let mut logs = Vec::new();
        for log_id in log_ids {
            let key = format!("{}{}", LOGS_KEY_PREFIX, log_id);
            if let Ok(Some(entry)) = get_json::<LogEntry>(&self.client, &key).await {
                // Apply filters
                let matches_plan = plan_id.map_or(true, |pid| entry.plan_id.as_deref() == Some(pid));
                let matches_phase = phase_id.map_or(true, |pid| entry.phase_id.as_deref() == Some(pid));
                
                if matches_plan && matches_phase {
                    logs.push(entry);
                }
            }
        }
        
        Ok(logs)
    }

    /// Get recent logs for a specific phase
    pub async fn get_phase_logs(&self, phase_id: &str) -> Result<Vec<LogEntry>, Box<dyn std::error::Error + Send + Sync>> {
        self.get_logs(None, Some(phase_id), Some(50)).await
    }

    /// Get all logs for a specific plan
    pub async fn get_plan_logs(&self, plan_id: &str) -> Result<Vec<LogEntry>, Box<dyn std::error::Error + Send + Sync>> {
        self.get_logs(Some(plan_id), None, Some(200)).await
    }

    async fn add_to_index(&self, log_id: &str, timestamp: &DateTime<Utc>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Use a Redis sorted set with timestamp as score for chronological ordering
        let _score = timestamp.timestamp_millis() as f64;
        
        // In a real implementation, we'd use Redis ZADD command
        // For now, we'll store a simple list (this is a simplified version)
        let mut index: Vec<String> = get_json(&self.client, LOGS_INDEX_KEY).await?.unwrap_or_default();
        index.push(log_id.to_string());
        
        // Keep only the most recent 1000 entries in the index
        if index.len() > 1000 {
            let len = index.len();
            index = index.into_iter().skip(len - 1000).collect();
        }
        
        set_json(&self.client, LOGS_INDEX_KEY, &index).await?;
        Ok(())
    }

    async fn get_log_ids_from_index(&self, limit: usize) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let index: Vec<String> = get_json(&self.client, LOGS_INDEX_KEY).await?.unwrap_or_default();
        
        // Return most recent entries (reverse order)
        let start = if index.len() > limit { index.len() - limit } else { 0 };
        Ok(index.into_iter().skip(start).rev().collect())
    }
}
