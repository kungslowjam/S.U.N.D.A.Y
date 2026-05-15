use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fmt;
use tokio::sync::broadcast;
use once_cell::sync::Lazy;
use std::collections::VecDeque;
use parking_lot::Mutex;

/// Predefined event types used throughout the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    InferenceStart,
    InferenceEnd,
    ToolCallStart,
    ToolCallEnd,
    ToolTimeout,
    SecurityAlert,
    SecurityBlock,
    TraceStep,
    TraceComplete,
    SessionStart,
    SessionEnd,
    AgentMessage,
    BrainStatus,
    AgentTurnStart,
    AgentTurnEnd,
    SharedMemoryUpdate,
    AgentTickStart,
    AgentTickEnd,
    AgentTickError,
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::InferenceStart => "inference_start",
            Self::InferenceEnd => "inference_end",
            Self::ToolCallStart => "tool_call_start",
            Self::ToolCallEnd => "tool_call_end",
            Self::ToolTimeout => "tool_timeout",
            Self::SecurityAlert => "security_alert",
            Self::SecurityBlock => "security_block",
            Self::TraceStep => "trace_step",
            Self::TraceComplete => "trace_complete",
            Self::SessionStart => "session_start",
            Self::SessionEnd => "session_end",
            Self::AgentMessage => "agent_message",
            Self::BrainStatus => "brain_status",
            Self::AgentTurnStart => "agent_turn_start",
            Self::AgentTurnEnd => "agent_turn_end",
            Self::SharedMemoryUpdate => "shared_memory_update",
            Self::AgentTickStart => "agent_tick_start",
            Self::AgentTickEnd => "agent_tick_end",
            Self::AgentTickError => "agent_tick_error",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for EventType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inference_start" => Ok(Self::InferenceStart),
            "inference_end" => Ok(Self::InferenceEnd),
            "tool_call_start" => Ok(Self::ToolCallStart),
            "tool_call_end" => Ok(Self::ToolCallEnd),
            "tool_timeout" => Ok(Self::ToolTimeout),
            "security_alert" => Ok(Self::SecurityAlert),
            "security_block" => Ok(Self::SecurityBlock),
            "trace_step" => Ok(Self::TraceStep),
            "trace_complete" => Ok(Self::TraceComplete),
            "session_start" => Ok(Self::SessionStart),
            "session_end" => Ok(Self::SessionEnd),
            "agent_message" => Ok(Self::AgentMessage),
            "brain_status" => Ok(Self::BrainStatus),
            "agent_turn_start" => Ok(Self::AgentTurnStart),
            "agent_turn_end" => Ok(Self::AgentTurnEnd),
            "shared_memory_update" => Ok(Self::SharedMemoryUpdate),
            "agent_tick_start" => Ok(Self::AgentTickStart),
            "agent_tick_end" => Ok(Self::AgentTickEnd),
            "agent_tick_error" => Ok(Self::AgentTickError),
            _ => Err(format!("Unknown event type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_type: EventType,
    pub timestamp: f64,
    pub data: serde_json::Value,
}

/// A high-performance, async event bus based on tokio broadcast channels.
/// Uses Arc to avoid cloning large event payloads for every subscriber.
pub struct EventBus {
    sender: broadcast::Sender<Arc<Event>>,
    history: Mutex<VecDeque<Arc<Event>>>,
    record_history: bool,
}

pub static GLOBAL_BUS: Lazy<Arc<EventBus>> = Lazy::new(|| Arc::new(EventBus::new(true)));

impl EventBus {
    pub fn new(record_history: bool) -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self { 
            sender,
            history: Mutex::new(VecDeque::with_capacity(5000)),
            record_history,
        }
    }

    /// Subscribe to all events on the bus.
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Event>> {
        self.sender.subscribe()
    }

    /// Subscribe to events with a callback (spawned in a tokio task).
    pub fn subscribe_callback(&self, callback: Box<dyn Fn(Arc<Event>) + Send + Sync>) {
        let mut rx = self.sender.subscribe();
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                callback(event);
            }
        });
    }

    /// Publish an event to all subscribers.
    pub fn publish(&self, event_type: EventType, data: impl Into<serde_json::Value>) -> Arc<Event> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        let event = Arc::new(Event {
            event_type,
            timestamp,
            data: data.into(),
        });

        if self.record_history {
            let mut history = self.history.lock();
            history.push_back(Arc::clone(&event));
            // Keep history at a reasonable size
            if history.len() > 5000 {
                history.drain(0..1000);
            }
        }

        // We ignore the result if there are no subscribers
        let _ = self.sender.send(Arc::clone(&event));

        event
    }

    pub fn history(&self) -> Vec<Arc<Event>> {
        self.history.lock().iter().cloned().collect()
    }

    pub fn clear_history(&self) {
        self.history.lock().clear();
    }
}

/// Helper to publish to the global bus easily
pub fn emit_event(event_type: EventType, data: serde_json::Value) {
    GLOBAL_BUS.publish(event_type, data);
}
