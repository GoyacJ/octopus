//! `octopus-harness-tool`
//!
//! Tool traits, registry, execution pool, result budget, and built-in tools.
//!
//! SPEC: docs/architecture/harness/crates/harness-tool.md
//! Status: M0 empty skeleton; concrete implementation lands in M3.

#![forbid(unsafe_code)]

pub mod context;
pub mod error;
pub mod result_budget;
pub mod tool;

pub use context::*;
pub use error::*;
pub use result_budget::*;
pub use tool::*;
