//! `octopus-harness-contracts`
//!
//! Shared contract types, events, errors, and schemas.
//!
//! SPEC: docs/architecture/harness/crates/harness-contracts.md

#![forbid(unsafe_code)]

pub mod blob;
pub mod capability;
pub mod dangerous_patterns;
pub mod enums;
pub mod errors;
pub mod events;
pub mod ids;
pub mod messages;
pub mod redactor;
pub mod schema_export;
pub mod tool;

pub use blob::*;
pub use capability::*;
pub use dangerous_patterns::*;
pub use enums::*;
pub use errors::*;
pub use events::*;
pub use ids::*;
pub use messages::*;
pub use redactor::*;
pub use schema_export::*;
pub use tool::*;
