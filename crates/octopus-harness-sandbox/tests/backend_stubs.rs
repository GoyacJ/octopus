#![cfg(any(feature = "docker", feature = "ssh"))]

use std::sync::Arc;

use harness_contracts::{Event, SandboxError};
use harness_sandbox::{EventSink, ExecContext, ExecSpec, SandboxBackend};

#[derive(Default)]
struct NullSink;

impl EventSink for NullSink {
    fn emit(&self, _event: Event) -> Result<(), SandboxError> {
        Ok(())
    }
}

#[cfg(feature = "docker")]
#[tokio::test]
async fn docker_sandbox_stub_is_object_safe() {
    let backend: Arc<dyn SandboxBackend> = Arc::new(harness_sandbox::DockerSandbox);
    assert_eq!(backend.backend_id(), "docker");
    assert!(backend
        .execute(
            ExecSpec::default(),
            ExecContext::for_test(Arc::new(NullSink)),
        )
        .await
        .err()
        .is_some_and(|error| error.to_string().contains("not implemented")));
    backend.shutdown().await.expect("shutdown should succeed");
}

#[cfg(feature = "ssh")]
#[tokio::test]
async fn ssh_sandbox_stub_is_object_safe() {
    let backend: Arc<dyn SandboxBackend> = Arc::new(harness_sandbox::SshSandbox);
    assert_eq!(backend.backend_id(), "ssh");
    assert!(backend
        .execute(
            ExecSpec::default(),
            ExecContext::for_test(Arc::new(NullSink)),
        )
        .await
        .err()
        .is_some_and(|error| error.to_string().contains("not implemented")));
    backend.shutdown().await.expect("shutdown should succeed");
}
