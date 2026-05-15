//! Structural protocols for substituting fakes in place of JarvisSystem.
//!
//! Rust translation of `src/sunday/system/protocols.py`.

use sunday_core::{EventBus, JarvisConfig};
use sunday_engine::InferenceEngine;
use std::sync::Arc;

/// Minimum surface of JarvisSystem that QueryOrchestrator depends on.
///
/// Tests can satisfy this with a lightweight struct — no need to construct
/// the full JarvisSystem or materialize every subsystem.
pub trait OrchestratorDeps: Send + Sync {
    fn config(&self) -> &JarvisConfig;
    fn bus(&self) -> &EventBus;
    fn engine(&self) -> &dyn InferenceEngine;
    fn engine_key(&self) -> &str;
    fn model(&self) -> &str;
    fn agent_name(&self) -> &str;
    fn memory_backend(&self) -> Option<&Arc<dyn Send + Sync>>;
    fn capability_policy(&self) -> Option<&Arc<dyn Send + Sync>>;
    fn session_store(&self) -> Option<&Arc<dyn Send + Sync>>;
    fn trace_store(&self) -> Option<&Arc<dyn Send + Sync>>;
    fn trace_collector(&self) -> Option<&Arc<dyn Send + Sync>>;
}
