//! `octopus-harness-session`
//!
//! Session lifecycle, projections, fork/reload behavior, and steering queue.
//!
//! SPEC: docs/architecture/harness/crates/harness-session.md
//! Status: M0 empty skeleton; concrete implementation lands in M3.

#![forbid(unsafe_code)]

pub mod builder;
pub mod lifecycle;
pub mod paths;
pub mod session;

pub use builder::*;
pub use paths::*;
pub use session::*;
