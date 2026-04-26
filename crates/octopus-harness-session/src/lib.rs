//! `octopus-harness-session`
//!
//! Session lifecycle, projections, fork/reload behavior, and steering queue.
//!
//! SPEC: docs/architecture/harness/crates/harness-session.md
//! Status: M0 empty skeleton; concrete implementation lands in M3.

#![forbid(unsafe_code)]

pub mod builder;
pub mod fork;
pub mod lifecycle;
pub mod paths;
pub mod projection;
#[cfg(feature = "hot-reload-fork")]
pub mod reload;
pub mod session;
pub mod snapshot;

pub use builder::*;
pub use paths::*;
pub use projection::*;
#[cfg(feature = "hot-reload-fork")]
pub use reload::*;
pub use session::*;
