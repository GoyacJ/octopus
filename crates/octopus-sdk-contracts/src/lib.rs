//! Shared SDK contract types for sessions, model IO, and UI intent IR.
//!
//! W1 Task 1 intentionally keeps this crate as a minimal scaffold.

mod ask_resolver;
mod compaction;
mod event;
mod hooks;
mod id;
mod message;
mod memory;
mod permissions;
mod prompt_cache;
mod secret;
mod tool_schema;
mod tools;
mod ui_intent;
mod usage;

pub use ask_resolver::*;
pub use compaction::*;
pub use event::*;
pub use hooks::*;
pub use id::*;
pub use message::*;
pub use memory::*;
pub use permissions::*;
pub use prompt_cache::*;
pub use secret::*;
pub use tool_schema::*;
pub use tools::*;
pub use ui_intent::*;
pub use usage::*;
