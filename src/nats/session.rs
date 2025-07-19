use async_nats::Client;
use crate::nats::messages::{StartMessage, ControlMessage, StateMessage, LogMessage};
 use chrono::Utc;
 use serde_json;
use uuid::Uuid;
use crate::model::Phase;

/// NATS session manager for PMP protocol
pub struct NatsSession {
    client: Client,
    pub session_id: String,
}

impl NatsSession {
    pub fn new(client: Client, session_id: String) -> Self {
        Self { client, session_id }
    }

    /// Generate a session ID
    pub fn generate_session_id() -> String {
        format!("session-{}", Uuid::new_v4())
    }

    /// Subject patterns according to PROTOCOL.md
    pub fn start_subject(&self) -> String {
        format!("plan.session.{}.start", self.session_id)
    }

    pub fn control_subject(&self) -> String {
        format!("plan.session.{}.control", self.session_id)
    }

    pub fn log_subject(&self) -> String {
        format!("plan.session.{}.log", self.session_id)
    }

    pub fn events_subject(&self) -> String {
        format!("plan.session.{}.events", self.session_id)
    }

    pub fn state_subject(&self) -> String {
        format!("plan.session.{}.state", self.session_id)
    }

    pub fn get_state_subject(&self) -> String {
        format!("plan.session.{}.get_state", self.session_id)
    }

    pub fn diff_subject(&self) -> String {
        format!("plan.session.{}.diff", self.session_id)
    }

    /// Send manifest to start session
    pub async fn start_session(&self, manifest: Vec<Phase>, dry_run: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let msg = StartMessage { manifest, dry_run };
        let payload = serde_json::to_vec(&msg)?;
        self.client.publish(self.start_subject(), payload.into()).await?;
        Ok(())
    }

    /// Send control command
    pub async fn send_control(&self, command: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let msg = ControlMessage { command };
        let payload = serde_json::to_vec(&msg)?;
        self.client.publish(self.control_subject(), payload.into()).await?;
        Ok(())
    }

    /// Publish state update
    pub async fn publish_state(&self, phase_id: String, status: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let updated = Utc::now().to_rfc3339();
        
        let msg = StateMessage { phase_id, status, updated };
        let payload = serde_json::to_vec(&msg)?;
        self.client.publish(self.state_subject(), payload.into()).await?;
        Ok(())
    }

    /// Publish log message
    pub async fn publish_log(&self, phase_id: Option<String>, level: String, message: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let timestamp = Utc::now().to_rfc3339();
        
        let msg = LogMessage { phase_id, level, message, timestamp };
        let payload = serde_json::to_vec(&msg)?;
        self.client.publish(self.log_subject(), payload.into()).await?;
        Ok(())
    }

    /// Subscribe to control messages
    pub async fn subscribe_control(&self) -> Result<async_nats::Subscriber, Box<dyn std::error::Error + Send + Sync>> {
        let sub = self.client.subscribe(self.control_subject()).await?;
        Ok(sub)
    }

    /// Subscribe to start messages
    pub async fn subscribe_start(&self) -> Result<async_nats::Subscriber, Box<dyn std::error::Error + Send + Sync>> {
        let sub = self.client.subscribe(self.start_subject()).await?;
        Ok(sub)
    }
}

