//! `octopus-harness-tool`
//!
//! Tool traits, registry, execution pool, result budget, and built-in tools.
//!
//! SPEC: docs/architecture/harness/crates/harness-tool.md
//! Status: M0 empty skeleton; concrete implementation lands in M3.

#![forbid(unsafe_code)]

pub mod builder;
#[cfg(feature = "builtin-toolset")]
pub mod builtin;
pub mod context;
pub mod error;
pub mod orchestrator;
pub mod pool;
pub mod registry;
pub mod result_budget;
pub mod tool;

pub use builder::*;
#[cfg(feature = "builtin-toolset")]
pub use builtin::*;
pub use context::*;
pub use error::*;
pub use harness_contracts::ToolSearchMode;
pub use harness_permission::{
    PermissionBroker, PermissionCheck, PermissionContext, PermissionRequest,
};
pub use orchestrator::*;
pub use pool::*;
pub use registry::*;
pub use result_budget::*;
pub use tool::*;
