//! Shared types and constants for the octopus Rust workspace.

pub const API_VERSION: &str = "v1";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceId(pub String);
