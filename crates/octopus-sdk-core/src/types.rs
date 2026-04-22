use std::path::PathBuf;

use octopus_sdk_contracts::{Message, PermissionMode, RunId, SessionId};
use octopus_sdk_model::{ModelError, ModelId};
use octopus_sdk_plugin::PluginError;
use octopus_sdk_sandbox::SandboxError;
use octopus_sdk_session::SessionError;
use octopus_sdk_tools::{RegistryError, ToolError};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct StartSessionInput {
    pub session_id: Option<SessionId>,
    pub working_dir: PathBuf,
    pub permission_mode: PermissionMode,
    pub model: ModelId,
    pub config_snapshot_id: String,
    pub effective_config_hash: String,
    pub token_budget: u32,
}

#[derive(Debug, Clone)]
pub struct SubmitTurnInput {
    pub session_id: SessionId,
    pub message: Message,
}

#[derive(Debug, Clone)]
pub struct SessionHandle {
    pub session_id: SessionId,
    pub working_dir: PathBuf,
    pub permission_mode: PermissionMode,
    pub model: ModelId,
    pub config_snapshot_id: String,
    pub effective_config_hash: String,
    pub token_budget: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunHandle {
    pub run_id: RunId,
    pub session_id: SessionId,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("runtime builder is missing required dependency `{field}`")]
    MissingBuilderField { field: &'static str },
    #[error("session `{session_id}` is not active in the current process")]
    SessionStateMissing { session_id: String },
    #[error("builder plugin snapshot does not match supplied registry")]
    PluginSnapshotMismatch,
    #[error("runtime run was cancelled")]
    Cancelled,
    #[error("tool `{name}` is not registered")]
    ToolNotFound { name: String },
    #[error("permission denied for tool `{name}`: {reason}")]
    PermissionDenied { name: String, reason: String },
    #[error("tool `{name}` requires unresolved ask/auth prompt kind `{kind}`")]
    UnresolvedPrompt { name: String, kind: String },
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Tool(#[from] ToolError),
    #[error(transparent)]
    Registry(#[from] RegistryError),
    #[error(transparent)]
    Sandbox(#[from] SandboxError),
    #[error(transparent)]
    Plugin(#[from] PluginError),
    #[error("hook execution failed: {0}")]
    Hook(String),
    #[error("serialization failed: {0}")]
    Serde(#[from] serde_json::Error),
}
