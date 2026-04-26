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
#[cfg(feature = "rule-engine")]
pub mod providers;
pub mod rule;
#[cfg(feature = "rule-engine")]
pub mod rule_engine;
#[cfg(feature = "stream")]
pub mod stream;

pub use broker::*;
pub use decision::*;
#[cfg(feature = "interactive")]
pub use direct::*;
#[cfg(feature = "rule-engine")]
pub use providers::*;
pub use rule::*;
#[cfg(feature = "rule-engine")]
pub use rule_engine::*;
#[cfg(feature = "stream")]
pub use stream::*;
