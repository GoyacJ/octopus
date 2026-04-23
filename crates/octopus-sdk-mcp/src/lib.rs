//! Shared SDK MCP crate.
//!
//! W3 Task 1 keeps this crate as a compileable scaffold.

mod client;
mod directory;
mod discovery;
mod error;
mod jsonrpc;
mod lifecycle;
mod manager;
mod transport;
mod types;

pub use client::*;
pub use directory::*;
pub use discovery::*;
pub use error::*;
pub use jsonrpc::*;
pub use lifecycle::*;
pub use manager::*;
pub use transport::*;
pub use types::*;
