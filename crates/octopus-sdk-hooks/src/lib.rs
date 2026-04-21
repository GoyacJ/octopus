//! Lifecycle hook runner for deterministic harness guards.

pub mod runner;

pub use octopus_sdk_contracts::{HookDecision, HookEvent, RewritePayload};
pub use runner::{
    Hook, HookError, HookRegistration, HookRunOutcome, HookRunner, HookSource,
};
