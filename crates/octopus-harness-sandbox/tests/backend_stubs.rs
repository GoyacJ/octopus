#![cfg(any(feature = "docker", feature = "ssh"))]

use std::sync::Arc;

use harness_contracts::{Event, SandboxError};
use harness_sandbox::{EventSink, ExecContext, ExecSpec, SandboxBackend};

#[derive(Default)]
struct RecordingSink;

impl EventSink for RecordingSink {
    fn emit(&self, _event: Event) -> Result<(), SandboxError> {
        Ok(())
    }
}

async fn assert_stub_rejects_execute(backend: Arc<dyn SandboxBackend>, backend_id: &str) {
    assert_eq!(backend.backend_id(), backend_id);
    let error = match backend
        .execute(
            ExecSpec::default(),
            ExecContext::for_test(Arc::new(RecordingSink)),
        )
        .await
    {
        Ok(_) => panic!("{backend_id} execute should reject"),
        Err(error) => error,
    };
    assert!(error
        .to_string()
        .contains(&format!("{backend_id} sandbox is not implemented")));
    backend.shutdown().await.expect("shutdown should succeed");
}

#[cfg(feature = "docker")]
#[tokio::test]
async fn docker_sandbox_stub_is_object_safe() {
    assert_stub_rejects_execute(Arc::new(harness_sandbox::DockerSandbox::new()), "docker").await;
}

#[cfg(feature = "ssh")]
#[tokio::test]
async fn ssh_sandbox_stub_is_object_safe() {
    assert_stub_rejects_execute(Arc::new(harness_sandbox::SshSandbox::new()), "ssh").await;
}
