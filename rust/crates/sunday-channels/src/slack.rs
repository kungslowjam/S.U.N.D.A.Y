//! Slack Incoming Webhook / Bot channel implementation.

use async_trait::async_trait;
use reqwest::Client;

use crate::{Channel, ChannelConfig, ChannelError};

pub struct SlackChannel {
    webhook_url: Option<String>,
    bot_token: Option<String>,
    client: Client,
}

impl SlackChannel {
    pub fn new(webhook_url: Option<String>, bot_token: Option<String>) -> Self {
        Self {
            webhook_url,
            bot_token,
            client: Client::new(),
        }
    }

    pub fn from_config(config: &ChannelConfig) -> Result<Self, ChannelError> {
        Ok(Self::new(config.webhook_url.clone(), config.api_token.clone()))
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn channel_id(&self) -> &str {
        "slack"
    }

    async fn send_message(&self, recipient: &str, text: &str) -> Result<(), ChannelError> {
        // Prefer webhook if available, otherwise use chat.postMessage
        if let Some(url) = &self.webhook_url {
            let body = serde_json::json!({
                "channel": recipient,
                "text": text,
            });
            self.client
                .post(url)
                .json(&body)
                .send()
                .await?
                .error_for_status()?;
        } else if let Some(token) = &self.bot_token {
            let url = "https://slack.com/api/chat.postMessage";
            let body = serde_json::json!({
                "channel": recipient,
                "text": text,
            });
            self.client
                .post(url)
                .bearer_auth(token)
                .json(&body)
                .send()
                .await?
                .error_for_status()?;
        } else {
            return Err(ChannelError::Config("Slack webhook or bot token missing".into()));
        }
        Ok(())
    }

    fn health(&self) -> bool {
        self.webhook_url.is_some() || self.bot_token.is_some()
    }
}
