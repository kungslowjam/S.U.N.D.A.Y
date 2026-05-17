//! LINE Messaging API channel implementation.

use async_trait::async_trait;
use reqwest::Client;

use crate::{Channel, ChannelConfig, ChannelError};

pub struct LineChannel {
    channel_access_token: String,
    client: Client,
}

impl LineChannel {
    pub fn new(channel_access_token: String) -> Self {
        Self {
            channel_access_token,
            client: Client::new(),
        }
    }

    pub fn from_config(config: &ChannelConfig) -> Result<Self, ChannelError> {
        let token = config
            .api_token
            .as_ref()
            .ok_or_else(|| ChannelError::Config("LINE channel access token missing".into()))?;
        Ok(Self::new(token.clone()))
    }
}

#[async_trait]
impl Channel for LineChannel {
    fn channel_id(&self) -> &str {
        "line"
    }

    async fn send_message(&self, recipient: &str, text: &str) -> Result<(), ChannelError> {
        let url = "https://api.line.me/v2/bot/message/push";
        let body = serde_json::json!({
            "to": recipient,
            "messages": [
                { "type": "text", "text": text }
            ]
        });
        self.client
            .post(url)
            .bearer_auth(&self.channel_access_token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    fn health(&self) -> bool {
        !self.channel_access_token.is_empty()
    }
}
