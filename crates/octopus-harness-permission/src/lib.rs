//! `octopus-harness-permission`
//!
//! Permission brokers, rule providers, and decision handling.
//!
//! SPEC: docs/architecture/harness/crates/harness-permission.md
//! Status: M0 empty skeleton; concrete implementation lands in M2.

#![forbid(unsafe_code)]

pub mod broker;
pub mod decision;
pub mod rule;

pub use broker::*;
pub use decision::*;
pub use rule::*;
