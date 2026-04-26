//! `octopus-harness-sandbox`
//!
//! Sandbox backend contracts and built-in local/noop backends.
//!
//! SPEC: docs/architecture/harness/crates/harness-sandbox.md
//! Status: M2 L1-C T11 trait and type skeleton.

#![forbid(unsafe_code)]

pub mod backend;
#[cfg(feature = "code-runtime")]
pub mod code_sandbox;
pub mod cwd;
pub mod dangerous;
#[cfg(feature = "docker")]
pub mod docker;
#[cfg(feature = "local")]
pub mod local;
#[cfg(feature = "noop")]
pub mod noop;
pub mod policy;
#[cfg(feature = "ssh")]
pub mod ssh;

pub use backend::*;
#[cfg(feature = "code-runtime")]
pub use code_sandbox::*;
pub use cwd::*;
pub use dangerous::*;
#[cfg(feature = "docker")]
pub use docker::*;
#[cfg(feature = "local")]
pub use local::*;
#[cfg(feature = "noop")]
pub use noop::*;
pub use policy::*;
#[cfg(feature = "ssh")]
pub use ssh::*;
