//! WhatsApp Cloud API channel implementation.

use async_trait::async_trait;
use reqwest::Client;

use crate::{Channel, ChannelConfig, ChannelError};

pub struct WhatsAppChannel {
    access_token: String,
    phone_number_id: String,
    client: Client,
}

impl WhatsAppChannel {
    pub fn new(access_token: String, phone_number_id: String) -> Self {
        Self {
            access_token,
            phone_number_id,
            client: Client::new(),
        }
    }

    pub fn from_config(config: &ChannelConfig) -> Result<Self, ChannelError> {
        let token = config
            .api_token
            .as_ref()
            .ok_or_else(|| ChannelError::Config("WhatsApp access token missing".into()))?;
        let phone_id = config
            .webhook_url
            .as_ref()
            .ok_or_else(|| ChannelError::Config("WhatsApp phone number ID missing".into()))?;
        Ok(Self::new(token.clone(), phone_id.clone()))
    }
}

#[async_trait]
impl Channel for WhatsAppChannel {
    fn channel_id(&self) -> &str {
        "whatsapp"
    }

    async fn send_message(&self, recipient: &str, text: &str) -> Result<(), ChannelError> {
        let url = format!(
            "https://graph.facebook.com/v18.0/{}/messages",
            self.phone_number_id
        );
        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "to": recipient,
            "type": "text",
            "text": { "body": text },
        });
        self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    fn health(&self) -> bool {
        !self.access_token.is_empty() && !self.phone_number_id.is_empty()
    }
}
