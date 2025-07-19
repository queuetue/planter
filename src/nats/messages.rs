use serde::{Deserialize, Serialize};
use crate::model::Phase;

/// Message types for NATS communication per PROTOCOL.md
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartMessage {
    pub manifest: Vec<Phase>,
    #[serde(rename = "dryRun", default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlMessage {
    pub command: String, // "pause", "resume", "cancel"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMessage {
    #[serde(rename = "phaseId")]
    pub phase_id: String,
    pub status: String, // "running", "complete", "failed"
    pub updated: String, // ISO 8601 timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMessage {
    #[serde(rename = "phaseId", skip_serializing_if = "Option::is_none")]
    pub phase_id: Option<String>,
    pub level: String,
    pub message: String,
    pub timestamp: String,
}

/// Message wrapper for session communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionMessage {
    Start(StartMessage),
    Control(ControlMessage),
    State(StateMessage),
    Log(LogMessage),
}

/// Control command types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionControl {
    Pause,
    Resume,
    Cancel,
}

impl std::fmt::Display for SessionControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionControl::Pause => write!(f, "pause"),
            SessionControl::Resume => write!(f, "resume"),
            SessionControl::Cancel => write!(f, "cancel"),
        }
    }
}
