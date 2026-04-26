//! `octopus-harness-permission`
//!
//! Permission brokers, rule providers, and decision handling.
//!
//! SPEC: docs/architecture/harness/crates/harness-permission.md
//! Status: M0 empty skeleton; concrete implementation lands in M2.

#![forbid(unsafe_code)]

pub mod broker;
pub mod decision;
#[cfg(feature = "interactive")]
pub mod direct;
pub mod rule;
#[cfg(feature = "stream")]
pub mod stream;

pub use broker::*;
pub use decision::*;
#[cfg(feature = "interactive")]
pub use direct::*;
pub use rule::*;
#[cfg(feature = "stream")]
pub use stream::*;
