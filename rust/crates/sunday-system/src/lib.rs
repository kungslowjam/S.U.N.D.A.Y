//! SUNDAY System — top-level composition layer.
//!
//! Rust translation of `src/sunday/system/`.
//! Provides JarvisSystem, SystemBuilder, QueryOrchestrator, and subsystem bundles.

pub mod bundles;
pub mod builder;
pub mod core;
pub mod orchestrator;
pub mod protocols;

pub use bundles::{AgentRuntime, Observability, Scheduling, SecurityContext};
pub use builder::SystemBuilder;
pub use core::JarvisSystem;
pub use orchestrator::QueryOrchestrator;
pub use protocols::OrchestratorDeps;
