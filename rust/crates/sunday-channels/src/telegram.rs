//! Telegram Bot API channel implementation.

use async_trait::async_trait;
use reqwest::Client;

use crate::{Channel, ChannelConfig, ChannelError};

pub struct TelegramChannel {
    bot_token: String,
    client: Client,
    base_url: String,
}

impl TelegramChannel {
    pub fn new(bot_token: String) -> Self {
        Self {
            bot_token: bot_token.clone(),
            client: Client::new(),
            base_url: format!("https://api.telegram.org/bot{}", bot_token),
        }
    }

    pub fn from_config(config: &ChannelConfig) -> Result<Self, ChannelError> {
        let token = config
            .api_token
            .as_ref()
            .ok_or_else(|| ChannelError::Config("Telegram bot token missing".into()))?;
        Ok(Self::new(token.clone()))
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn channel_id(&self) -> &str {
        "telegram"
    }

    async fn send_message(&self, recipient: &str, text: &str) -> Result<(), ChannelError> {
        let url = format!("{}/sendMessage", self.base_url);
        let body = serde_json::json!({
            "chat_id": recipient,
            "text": text,
        });
        self.client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    fn health(&self) -> bool {
        !self.bot_token.is_empty()
    }
}
