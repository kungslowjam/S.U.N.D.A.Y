//! AgentRuntime — high-performance lifecycle manager for SUNDAY agents.

use crate::traits::OjAgent;
use sunday_core::{AgentContext, AgentResult, SUNDAYError, EventBus, EventType};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Running,
    Error,
    NeedsAttention,
    BudgetExceeded,
}

impl Default for AgentStatus {
    fn default() -> Self {
        AgentStatus::Idle
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct AgentState {
    pub id: String,
    pub name: String,
    pub status: AgentStatus,
    pub current_activity: String,
    pub last_activity_at: f64,
    pub total_runs: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub summary_memory: String,
}

/// Native Agent Runtime managing lifecycle, telemetry, and event orchestration.
pub struct AgentRuntime {
    agents: RwLock<HashMap<String, Arc<dyn OjAgent>>>,
    states: RwLock<HashMap<String, AgentState>>,
    event_bus: Arc<EventBus>,
}

impl AgentRuntime {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
            states: RwLock::new(HashMap::new()),
            event_bus,
        }
    }

    /// Register a new agent instance into the runtime.
    pub fn register_agent(&self, agent: Arc<dyn OjAgent>, name: &str) {
        let id = agent.agent_id().to_string();
        self.agents.write().insert(id.clone(), agent);
        self.states.write().insert(id.clone(), AgentState {
            id,
            name: name.to_string(),
            status: AgentStatus::Idle,
            current_activity: "Ready".to_string(),
            last_activity_at: now_ts(),
            ..Default::default()
        });
    }

    /// Execute a single tick (turn) for an agent with full lifecycle tracking.
    pub async fn execute_tick(
        &self, 
        agent_id: &str, 
        input: &str, 
        context: Option<&AgentContext>
    ) -> Result<AgentResult, SUNDAYError> {
        let agent = {
            let agents = self.agents.read();
            agents.get(agent_id).cloned().ok_or_else(|| {
                SUNDAYError::Agent(sunday_core::error::AgentError::NotFound(agent_id.to_string()))
            })?
        };

        // 1. Lifecycle: Start Tick
        self.update_state(agent_id, |state| {
            state.status = AgentStatus::Running;
            state.current_activity = "Executing turn...".to_string();
            state.last_activity_at = now_ts();
        });

        self.event_bus.publish(EventType::AgentTickStart, serde_json::json!({
            "agent_id": agent_id,
            "timestamp": now_ts(),
        }));

        let start = Instant::now();
        
        // 2. Core Execution: Async Run
        let result = agent.run(input, context).await;

        let duration = start.elapsed().as_secs_f64();

        // 3. Lifecycle: Finalize Tick
        match result {
            Ok(res) => {
                self.update_state(agent_id, |state| {
                    state.status = AgentStatus::Idle;
                    state.current_activity = "Idle".to_string();
                    state.total_runs += 1;
                    
                    // Update token metrics if available
                    if let Some(tokens) = res.metadata.get("total_tokens").and_then(|v| v.as_u64()) {
                        state.total_tokens += tokens;
                    }
                    
                    state.last_activity_at = now_ts();
                    state.summary_memory = res.content.chars().take(2000).collect();
                });

                self.event_bus.publish(EventType::AgentTickEnd, serde_json::json!({
                    "agent_id": agent_id,
                    "status": "ok",
                    "duration": duration,
                    "tokens": res.metadata.get("total_tokens"),
                }));

                Ok(res)
            }
            Err(e) => {
                self.update_state(agent_id, |state| {
                    state.status = AgentStatus::Error;
                    state.current_activity = format!("Error: {}", e);
                    state.last_activity_at = now_ts();
                });

                self.event_bus.publish(EventType::AgentTickError, serde_json::json!({
                    "agent_id": agent_id,
                    "error": e.to_string(),
                    "duration": duration,
                }));

                Err(e)
            }
        }
    }

    /// Internal state updater.
    fn update_state<F>(&self, agent_id: &str, f: F)
    where
        F: FnOnce(&mut AgentState),
    {
        let mut states = self.states.write();
        if let Some(state) = states.get_mut(agent_id) {
            f(state);
        }
    }

    /// Get current state for UI/Monitoring.
    pub fn get_state(&self, agent_id: &str) -> Option<AgentState> {
        self.states.read().get(agent_id).cloned()
    }

    /// List all managed agents.
    pub fn list_agents(&self) -> Vec<AgentState> {
        self.states.read().values().cloned().collect()
    }
}

/// Helper for Unix timestamps.
fn now_ts() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
