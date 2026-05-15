//! SUNDAY Core — foundation types, registry, config, and event bus.
//!
//! This crate provides the shared data types, configuration loading,
//! component registry, and event bus used by all other SUNDAY crates.

pub mod config;
pub mod error;
pub mod events;
pub mod shared_mem;
pub mod hardware;
pub mod model_catalog;
pub mod registry;
pub mod types;
pub mod tokenizer;

pub use config::{load_config, JarvisConfig};
pub use error::{SUNDAYError, EngineError};
pub use events::{Event, EventBus, EventType, GLOBAL_BUS, emit_event};
pub use shared_mem::SharedMemorySegment;
pub use model_catalog::{merge_discovered_models, register_builtin_models, BUILTIN_MODELS};
pub use registry::TypedRegistry;
pub use types::*;
pub use tokenizer::*;
