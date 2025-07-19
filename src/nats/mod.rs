pub mod session;
pub mod client;
pub mod messages;

pub use session::NatsSession;
// pub use client::NatsClient;
// pub use messages::{StartMessage, ControlMessage, StateMessage, LogMessage, SessionMessage, SessionControl};

use async_nats::Client;

/// Connect to NATS server
pub async fn connect(url: &str) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
    let client = async_nats::connect(url).await?;
    Ok(client)
}
