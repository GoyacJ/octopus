//! `octopus-harness-hook`
//!
//! Hook registry, dispatcher, transports, failure modes, and transaction semantics.
//!
//! SPEC: docs/architecture/harness/crates/harness-hook.md
//! Status: M3-T06 public contract surface.

#![forbid(unsafe_code)]

pub mod context;
pub mod event;
pub mod handler;
pub mod outcome;
pub mod views;

pub use context::*;
pub use event::*;
pub use handler::*;
pub use outcome::*;
pub use views::*;

pub use harness_contracts::{
    HookEventKind, HookFailureMode, HookOutcomeDiscriminant, InconsistentReason,
};
