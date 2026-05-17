//! Tools primitive — BaseTool trait, ToolExecutor, built-in tools, storage backends.

pub mod builtin;
pub mod browser_ax;
pub mod browser_native;
pub mod browser_native_js;
pub mod executor;
pub mod rig_tools;
pub mod storage;
pub mod traits;

pub use executor::ToolExecutor;
pub use traits::BaseTool;
pub use browser_native::{NativeBrowser, NativeBrowserSession};
