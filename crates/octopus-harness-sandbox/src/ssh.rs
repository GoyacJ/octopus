//! SSH sandbox backend placeholder.

use async_trait::async_trait;
use harness_contracts::SandboxError;

use crate::{
    ExecContext, ExecSpec, ProcessHandle, SandboxBackend, SandboxCapabilities, SessionSnapshotFile,
    SnapshotSpec,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct SshSandbox;

impl SshSandbox {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SandboxBackend for SshSandbox {
    fn backend_id(&self) -> &'static str {
        "ssh"
    }

    fn capabilities(&self) -> SandboxCapabilities {
        SandboxCapabilities::default()
    }

    async fn execute(
        &self,
        _spec: ExecSpec,
        _ctx: ExecContext,
    ) -> Result<ProcessHandle, SandboxError> {
        Err(not_implemented())
    }

    async fn snapshot_session(
        &self,
        _spec: &SnapshotSpec,
    ) -> Result<SessionSnapshotFile, SandboxError> {
        Err(not_implemented())
    }

    async fn restore_session(&self, _snapshot: &SessionSnapshotFile) -> Result<(), SandboxError> {
        Err(not_implemented())
    }

    async fn shutdown(&self) -> Result<(), SandboxError> {
        Ok(())
    }
}

fn not_implemented() -> SandboxError {
    SandboxError::Message("ssh sandbox is not implemented in M2-T13".to_owned())
}
