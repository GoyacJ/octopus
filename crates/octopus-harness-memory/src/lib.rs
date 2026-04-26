//! `octopus-harness-memory`
//!
//! Memory store and lifecycle primitives.
//!
//! SPEC: docs/architecture/harness/crates/harness-memory.md
//! Status: M0 empty skeleton; concrete implementation lands in M2.

#![forbid(unsafe_code)]

pub use harness_contracts::MemdirFileTag as MemdirFile;

#[cfg(feature = "external-slot")]
pub mod external;
pub mod lifecycle;
#[cfg(feature = "builtin")]
pub mod memdir;
#[cfg(feature = "external-slot")]
pub mod mock;
#[cfg(feature = "threat-scanner")]
pub mod scanner;
pub mod store;
pub mod types;

#[cfg(feature = "external-slot")]
pub use external::*;
pub use lifecycle::*;
#[cfg(feature = "builtin")]
pub use memdir::*;
#[cfg(feature = "external-slot")]
pub use mock::*;
#[cfg(feature = "threat-scanner")]
pub use scanner::*;
pub use store::*;
pub use types::*;
