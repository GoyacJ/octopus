//! `octopus-harness-plugin`
//!
//! Plugin manifests, runtime loading, signer trust, and capability handles.
//!
//! SPEC: docs/architecture/harness/crates/harness-plugin.md
//! Status: M5 plugin implementation.

#![forbid(unsafe_code)]

pub mod capability;
#[cfg(feature = "dynamic-load")]
pub mod dynamic_load;
pub mod error;
pub mod loader;
pub mod manifest;
pub mod plugin;
pub mod registry;
pub mod signer;
pub mod sources;

pub use capability::*;
#[cfg(feature = "dynamic-load")]
pub use dynamic_load::*;
pub use error::*;
pub use loader::*;
pub use manifest::*;
pub use plugin::*;
pub use registry::*;
pub use signer::*;
pub use sources::*;
