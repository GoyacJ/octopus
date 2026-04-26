//! Error contracts.
//!
//! SPEC: docs/architecture/harness/crates/harness-contracts.md §3.8

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{BudgetKind, TenantId};

pub type Result<T, E = HarnessError> = std::result::Result<T, E>;

macro_rules! define_error_family {
    ($($name:ident),+ $(,)?) => {
        $(
            #[non_exhaustive]
            #[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, thiserror::Error)]
            #[serde(rename_all = "snake_case")]
            pub enum $name {
                #[error("{0}")]
                Message(String),
            }
        )+
    };
}

#[non_exhaustive]
#[derive(
    Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, thiserror::Error,
)]
#[serde(rename_all = "snake_case")]
pub enum ModelError {
    #[error("{0}")]
    Message(String),
    #[error("rate limited: {0}")]
    RateLimited(String),
    #[error("context too long: tokens={tokens}, max={max}")]
    ContextTooLong { tokens: usize, max: usize },
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("all credentials banned")]
    AllCredentialsBanned,
    #[error("aux model not configured")]
    AuxModelNotConfigured,
    #[error("auth expired: {0}")]
    AuthExpired(String),
    #[error("provider unavailable: {0}")]
    ProviderUnavailable(String),
    #[error("unexpected response: {0}")]
    UnexpectedResponse(String),
    #[error("cancelled by caller")]
    Cancelled,
    #[error("deadline exceeded after {0:?}")]
    DeadlineExceeded(std::time::Duration),
    #[error("io: {0}")]
    Io(String),
}

define_error_family! {
    JournalError,
    SandboxError,
    PermissionError,
    MemoryError,
    ToolError,
    SessionError,
    EngineError,
    PluginError,
    McpError,
    HookError,
    ContextError,
}

#[non_exhaustive]
#[derive(
    Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, thiserror::Error,
)]
#[serde(rename_all = "snake_case")]
pub enum HarnessError {
    #[error("prompt cache locked for running session")]
    PromptCacheLocked,
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("invalid tenant: {0:?}")]
    InvalidTenant(TenantId),
    #[error("budget exhausted: {0:?}")]
    BudgetExhausted(BudgetKind),
    #[error("interrupted by user")]
    Interrupted,
    #[error("model: {0}")]
    Model(ModelError),
    #[error("journal: {0}")]
    Journal(JournalError),
    #[error("sandbox: {0}")]
    Sandbox(SandboxError),
    #[error("permission: {0}")]
    Permission(PermissionError),
    #[error("memory: {0}")]
    Memory(MemoryError),
    #[error("tool: {0}")]
    Tool(ToolError),
    #[error("session: {0}")]
    Session(SessionError),
    #[error("engine: {0}")]
    Engine(EngineError),
    #[error("plugin: {0}")]
    Plugin(PluginError),
    #[error("mcp: {0}")]
    Mcp(McpError),
    #[error("hook: {0}")]
    Hook(HookError),
    #[error("context: {0}")]
    Context(ContextError),
    #[error("tenant mismatch")]
    TenantMismatch,
    #[error("internal error: {0}")]
    Internal(String),
    #[error("other: {0}")]
    Other(String),
}

macro_rules! impl_from_family {
    ($($variant:ident($name:ident)),+ $(,)?) => {
        $(
            impl From<$name> for HarnessError {
                fn from(value: $name) -> Self {
                    Self::$variant(value)
                }
            }
        )+
    };
}

impl_from_family! {
    Model(ModelError),
    Journal(JournalError),
    Sandbox(SandboxError),
    Permission(PermissionError),
    Memory(MemoryError),
    Tool(ToolError),
    Session(SessionError),
    Engine(EngineError),
    Plugin(PluginError),
    Mcp(McpError),
    Hook(HookError),
    Context(ContextError),
}
