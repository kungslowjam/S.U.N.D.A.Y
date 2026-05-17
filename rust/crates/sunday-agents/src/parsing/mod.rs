//! Fast text parsing utilities — response parsing, tool extraction, tag stripping.
//!
//! Rust implementations of regex-heavy Python functions to eliminate
//! per-turn GIL contention and regex overhead.

pub mod response_parser;
pub mod tool_extractor;
pub mod tag_stripper;
pub mod slack_formatter;
pub mod browser_cleaner;
pub mod context_optimizer;
pub mod skill_auto_creator;
pub mod user_model;

pub use response_parser::parse_structured_response;
pub use tool_extractor::extract_tool_call;
pub use tag_stripper::strip_think_tags;
pub use slack_formatter::to_slack_fmt;
pub use browser_cleaner::clean_browser_text;
pub use context_optimizer::{compress_tool_outputs, apply_window};
pub use skill_auto_creator::{SkillAutoCreator, SkillCandidate, analyze_conversation_for_skill, generate_skill_manifest, batch_analyze};
pub use user_model::{UserModel, UserModelStore, Conclusion, ConclusionLevel, extract_conclusions};
