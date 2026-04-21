//! Model catalog, routing, and protocol adapters for the Octopus SDK.
//!
//! W2 Task 1 intentionally keeps this crate as a minimal scaffold.

mod adapter;
mod catalog;
mod enums;
mod error;
mod fallback;
mod id;
mod provider;
mod provider_impl;
mod request;
mod role_router;

pub use adapter::*;
pub use catalog::*;
pub use enums::*;
pub use error::*;
pub use fallback::*;
pub use id::*;
pub use provider::*;
pub use provider_impl::*;
pub use request::*;
pub use role_router::*;
