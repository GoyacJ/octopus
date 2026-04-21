//! Prompt and compaction helpers for long-horizon SDK runs.

pub mod compact;
pub mod memory;
pub mod prompt;
pub mod scratchpad;

pub use compact::{CompactionError, Compactor, SessionView};
pub use memory::MemoryBackend;
pub use prompt::{PromptCtx, SystemPromptBuilder, SystemPromptSection};
pub use scratchpad::DurableScratchpad;
