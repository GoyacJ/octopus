use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::SandboxHandle;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkProxy {
    pub endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SandboxSpec {
    pub fs_whitelist: Vec<PathBuf>,
    #[serde(default)]
    pub network_allowlist: Vec<String>,
    pub network_proxy: Option<NetworkProxy>,
    pub env_allowlist: Vec<String>,
    pub cpu_time_limit_ms: Option<u64>,
    pub wall_time_limit_ms: Option<u64>,
    pub memory_limit_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxCommand {
    pub cmd: String,
    pub args: Vec<String>,
    pub stdin: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SandboxOutput {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub truncated: bool,
    pub timed_out: bool,
}

#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("failed to provision sandbox: {reason}")]
    Provision { reason: String },
    #[error("failed to execute sandbox command: {reason}")]
    Execute { reason: String },
    #[error("failed to terminate sandbox: {reason}")]
    Terminate { reason: String },
    #[error("sandbox backend is unsupported on the current platform")]
    UnsupportedPlatform,
    #[error("sandbox resource exhausted: {kind}")]
    ResourceExhausted { kind: String },
    #[error("sandbox command timed out")]
    Timeout,
}

#[async_trait]
pub trait SandboxBackend: Send + Sync {
    async fn provision(&self, spec: SandboxSpec) -> Result<SandboxHandle, SandboxError>;

    async fn execute(
        &self,
        handle: &SandboxHandle,
        cmd: SandboxCommand,
    ) -> Result<SandboxOutput, SandboxError>;

    async fn terminate(&self, handle: SandboxHandle) -> Result<(), SandboxError>;
}
