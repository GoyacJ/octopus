use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use harness_contracts::{Event, KillScope, SandboxError};
use harness_sandbox::{
    ActivityHandle, EventSink, ExecContext, ExecOutcome, ExecSpec, ProcessHandle, SandboxBackend,
    SandboxCapabilities, SessionSnapshotFile, SnapshotSpec,
};

#[derive(Default)]
struct NullSink;

impl EventSink for NullSink {
    fn emit(&self, _event: Event) -> Result<(), SandboxError> {
        Ok(())
    }
}

struct TestActivity;

#[async_trait]
impl ActivityHandle for TestActivity {
    async fn wait(&self) -> Result<ExecOutcome, SandboxError> {
        Ok(ExecOutcome::default())
    }

    async fn kill(&self, _signal: i32, _scope: KillScope) -> Result<(), SandboxError> {
        Ok(())
    }

    fn touch(&self) {}

    fn last_activity(&self) -> Instant {
        Instant::now()
    }
}

struct TestBackend {
    id: String,
}

#[async_trait]
impl SandboxBackend for TestBackend {
    fn backend_id(&self) -> &str {
        &self.id
    }

    fn capabilities(&self) -> SandboxCapabilities {
        SandboxCapabilities {
            snapshot_kinds: BTreeSet::default(),
            ..SandboxCapabilities::default()
        }
    }

    async fn execute(
        &self,
        _spec: ExecSpec,
        _ctx: ExecContext,
    ) -> Result<ProcessHandle, SandboxError> {
        Ok(ProcessHandle {
            pid: Some(42),
            stdout: None,
            stderr: None,
            stdin: None,
            cwd_marker: None,
            activity: Arc::new(TestActivity),
        })
    }

    async fn snapshot_session(
        &self,
        _spec: &SnapshotSpec,
    ) -> Result<SessionSnapshotFile, SandboxError> {
        Ok(SessionSnapshotFile::default())
    }

    async fn restore_session(&self, _snapshot: &SessionSnapshotFile) -> Result<(), SandboxError> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), SandboxError> {
        Ok(())
    }
}

#[tokio::test]
async fn sandbox_backend_is_object_safe_and_has_noop_hooks() {
    let backend: Arc<dyn SandboxBackend> = Arc::new(TestBackend {
        id: "test".to_owned(),
    });
    let spec = ExecSpec::default();
    let ctx = ExecContext::for_test(Arc::new(NullSink));

    backend.before_execute(&spec, &ctx).await.unwrap();
    let handle = backend.execute(spec, ctx.clone()).await.unwrap();
    let outcome = handle.activity.wait().await.unwrap();
    backend.after_execute(&outcome, &ctx).await.unwrap();

    assert_eq!(handle.pid, Some(42));
    assert_eq!(outcome.stdout_bytes_observed, 0);
}
