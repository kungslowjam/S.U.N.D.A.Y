//! Messaging channel primitive for SUNDAY.
//!
//! Provides a unified trait and implementations for:
//! - Telegram (Bot API via HTTP)
//! - Slack (Incoming Webhooks + Bolt)
//! - WhatsApp Cloud API (webhooks)
//! - LINE (Messaging API)

pub mod error;
pub mod line;
pub mod slack;
pub mod telegram;
pub mod traits;
pub mod whatsapp;

pub use error::ChannelError;
pub use traits::{Channel, ChannelConfig, Message as ChannelMessage};
