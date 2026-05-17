use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChannelError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Channel not connected: {0}")]
    NotConnected(String),
    #[error("Unknown channel: {0}")]
    UnknownChannel(String),
}
