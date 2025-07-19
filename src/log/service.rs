use crate::log::{Event, EventBus, LogEntry, LogStorage};
use crate::state::redis::RedisClient;

/// Redis-backed event bus that persists logs
pub struct RedisEventBus {
    storage: LogStorage,
}

impl RedisEventBus {
    pub fn new(redis_client: RedisClient) -> Self {
        Self {
            storage: LogStorage::new(redis_client),
        }
    }
}

impl EventBus for RedisEventBus {
    fn publish(&self, event: Event) {
        // Print to console for immediate feedback
        println!("[event] {:?}", event);
        
        // Store in Redis asynchronously (fire and forget for now)
        let storage = self.storage.clone();
        let event_clone = event.clone();
        tokio::spawn(async move {
            let log_entry = LogEntry::new(event_clone);
            if let Err(e) = storage.store_log(log_entry).await {
                eprintln!("Failed to store log entry: {}", e);
            }
        });
    }
}

/// Enhanced logging service that integrates with Redis
#[derive(Clone)]
pub struct LoggingService {
    storage: Option<LogStorage>,
}

impl LoggingService {
    pub fn new(redis_client: Option<RedisClient>) -> Self {
        let storage = redis_client.map(|client| {
            LogStorage::new(client)
        });
        
        Self { storage }
    }

    /// Log an event with additional context
    pub async fn log_event_with_context(
        &self,
        event: Event,
        plan_id: Option<String>,
        phase_id: Option<String>,
        context: std::collections::HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(storage) = &self.storage {
            let mut log_entry = LogEntry::new(event);
            
            if let Some(pid) = plan_id {
                log_entry = log_entry.with_plan_id(pid);
            }
            
            if let Some(ph_id) = phase_id {
                log_entry = log_entry.with_phase_id(ph_id);
            }
            
            for (key, value) in context {
                log_entry = log_entry.with_context(key, value);
            }
            
            storage.store_log(log_entry).await?;
        } else {
            // Fallback to console logging in stateless mode
            println!("[log] {:?} (plan: {:?}, phase: {:?})", event, plan_id, phase_id);
        }
        
        Ok(())
    }

    /// Get logs with filtering
    pub async fn get_logs(
        &self,
        plan_id: Option<&str>,
        phase_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<LogEntry>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(storage) = &self.storage {
            storage.get_logs(plan_id, phase_id, limit).await
        } else {
            // Return empty in stateless mode
            Ok(Vec::new())
        }
    }

    /// Get logs for a specific phase
    pub async fn get_phase_logs(&self, phase_id: &str) -> Result<Vec<LogEntry>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(storage) = &self.storage {
            storage.get_phase_logs(phase_id).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Get logs for a specific plan
    pub async fn get_plan_logs(&self, plan_id: &str) -> Result<Vec<LogEntry>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(storage) = &self.storage {
            storage.get_plan_logs(plan_id).await
        } else {
            Ok(Vec::new())
        }
    }
}
