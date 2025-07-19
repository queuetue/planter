use async_nats::{Client, ConnectOptions};
use std::time::Duration;
// use tokio_stream::StreamExt;  // not needed here
use super::NatsSession;

/// NATS client wrapper for PMP
pub struct NatsClient {
    client: Client,
    server_url: String,
}

impl NatsClient {
    /// Connect to NATS server
    pub async fn connect(server_url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let connect_options = ConnectOptions::new()
            .retry_on_initial_connect()
            .max_reconnects(5)
            .reconnect_delay_callback(|attempts| {
                // exponential backoff: 1000ms * 2^attempts, capped at 8000ms
                let exp = 2u64.pow(attempts as u32);
                let backoff = 1000u64.saturating_mul(exp);
                let delay_ms = std::cmp::min(backoff, 8000u64);
                Duration::from_millis(delay_ms)
            });

        let client = async_nats::connect_with_options(server_url, connect_options).await?;
        
        Ok(Self {
            client,
            server_url: server_url.to_string(),
        })
    }

    /// Create a new session
    pub fn new_session(&self) -> NatsSession {
        let session_id = NatsSession::generate_session_id();
        NatsSession::new(self.client.clone(), session_id)
    }

    /// Create session with specific ID
    pub fn session_with_id(&self, session_id: String) -> NatsSession {
        NatsSession::new(self.client.clone(), session_id)
    }

    /// Check connection health
    pub async fn health_check(&self) -> bool {
        // Try to publish to a health check subject
        match self.client.publish("health.check", "ping".into()).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Get connection info
    pub fn server_info(&self) -> &str {
        &self.server_url
    }

    /// Subscribe to all session events (for debugging/monitoring)
    pub async fn subscribe_all_sessions(&self) -> Result<async_nats::Subscriber, Box<dyn std::error::Error + Send + Sync>> {
        let sub = self.client.subscribe("plan.session.>").await?;
        Ok(sub)
    }
}
