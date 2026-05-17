use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A generic messaging channel that can send and receive messages.
#[async_trait]
pub trait Channel: Send + Sync {
    /// Unique channel identifier, e.g. "telegram", "slack".
    fn channel_id(&self) -> &str;

    /// Send a text message to the given recipient (user ID, channel ID, etc.).
    async fn send_message(&self, recipient: &str, text: &str) -> Result<(), super::ChannelError>;

    /// Health check — returns true if the channel is ready to send.
    fn health(&self) -> bool;
}

/// Normalized message received from any channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub channel: String,
    pub sender_id: String,
    pub sender_name: Option<String>,
    pub text: String,
    pub timestamp: String,
    pub raw: serde_json::Value,
}

/// Per-channel configuration stub.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelConfig {
    pub enabled: bool,
    pub webhook_url: Option<String>,
    pub api_token: Option<String>,
    pub api_secret: Option<String>,
}
