#![cfg(all(feature = "local", feature = "noop", unix))]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use futures::{stream::BoxStream, StreamExt};
use harness_contracts::{Event, SandboxError, SandboxExitStatus};
use harness_sandbox::{
    EventSink, ExecContext, ExecOutcome, ExecSpec, LocalSandbox, NoopSandbox, SandboxBackend,
    SessionSnapshotFile, SnapshotSpec, StdioSpec,
};

#[derive(Default)]
struct RecordingSink {
    events: Mutex<Vec<Event>>,
}

impl RecordingSink {
    fn events(&self) -> Vec<Event> {
        self.events.lock().expect("events lock should work").clone()
    }
}

impl EventSink for RecordingSink {
    fn emit(&self, event: Event) -> Result<(), SandboxError> {
        self.events
            .lock()
            .expect("events lock should work")
            .push(event);
        Ok(())
    }
}

fn temp_root(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "octopus-harness-sandbox-contract-{name}-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("temp root should be created");
    root
}

fn shell_spec(script: &str) -> ExecSpec {
    ExecSpec {
        command: "/bin/sh".to_owned(),
        args: vec!["-c".to_owned(), script.to_owned()],
        stdin: StdioSpec::Null,
        stdout: StdioSpec::Piped,
        stderr: StdioSpec::Piped,
        ..ExecSpec::default()
    }
}

async fn collect_stdout(mut stdout: BoxStream<'static, Bytes>) -> String {
    let mut bytes = Vec::new();
    while let Some(chunk) = stdout.next().await {
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).expect("stdout should be utf8")
}

fn expect_execute_error<T>(result: Result<T, SandboxError>, message: &str) -> SandboxError {
    match result {
        Ok(_) => panic!("{message}"),
        Err(error) => error,
    }
}

#[tokio::test]
async fn contract_backends_expose_sane_identity_and_capabilities() {
    let local: Arc<dyn SandboxBackend> = Arc::new(LocalSandbox::new(temp_root("identity-local")));
    let noop: Arc<dyn SandboxBackend> = Arc::new(NoopSandbox::new());

    assert_eq!(local.backend_id(), "local");
    assert_eq!(noop.backend_id(), "noop");

    let local_caps = local.capabilities();
    assert!(local_caps.default_timeout > Duration::ZERO);
    assert!(local_caps.supports_streaming);
    assert!(local_caps.supports_stdin);
    assert!(local_caps.supports_activity_heartbeat);

    let noop_caps = noop.capabilities();
    assert!(noop_caps.default_timeout > Duration::ZERO);
    assert!(!noop_caps.supports_streaming);
    assert!(!noop_caps.supports_stdin);
    assert!(!noop_caps.supports_activity_heartbeat);
}

#[tokio::test]
async fn contract_lifecycle_hooks_and_shutdown_are_safe() {
    let backends: Vec<Arc<dyn SandboxBackend>> = vec![
        Arc::new(LocalSandbox::new(temp_root("lifecycle-local"))),
        Arc::new(NoopSandbox::new()),
    ];
    let spec = shell_spec("printf ignored");
    let ctx = ExecContext::for_test(Arc::new(RecordingSink::default()));
    let outcome = ExecOutcome::default();
    let snapshot_spec = SnapshotSpec::default();
    let snapshot_file = SessionSnapshotFile::default();

    for backend in backends {
        backend
            .before_execute(&spec, &ctx)
            .await
            .expect("before_execute should be safe");
        backend
            .after_execute(&outcome, &ctx)
            .await
            .expect("after_execute should be safe");
        backend.shutdown().await.expect("shutdown should be safe");

        let snapshot_error = backend
            .snapshot_session(&snapshot_spec)
            .await
            .expect_err("snapshot should be an explicit stub");
        assert!(matches!(snapshot_error, SandboxError::Message(_)));

        let restore_error = backend
            .restore_session(&snapshot_file)
            .await
            .expect_err("restore should be an explicit stub");
        assert!(matches!(restore_error, SandboxError::Message(_)));
    }
}

#[tokio::test]
async fn contract_execute_semantics_are_deterministic() {
    let sink = Arc::new(RecordingSink::default());
    let local: Arc<dyn SandboxBackend> = Arc::new(LocalSandbox::new(temp_root("execute-local")));
    let mut handle = local
        .execute(
            shell_spec("printf contract"),
            ExecContext::for_test(sink.clone()),
        )
        .await
        .expect("local execute should spawn process");
    let stdout = handle.stdout.take().expect("stdout should be piped");
    let output = collect_stdout(stdout).await;
    let outcome = handle.activity.wait().await.expect("wait should succeed");

    assert_eq!(output, "contract");
    assert_eq!(outcome.exit_status, SandboxExitStatus::Code(0));

    let events = sink.events();
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SandboxExecutionStarted(started) if started.backend_id == "local"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SandboxExecutionCompleted(completed)
            if completed.backend_id == "local"
                && completed.exit_status == SandboxExitStatus::Code(0)
    )));

    let noop = Arc::new(NoopSandbox::new());
    let noop_backend: Arc<dyn SandboxBackend> = noop.clone();
    let spec = shell_spec("printf contract");
    let error = expect_execute_error(
        noop_backend
            .execute(
                spec.clone(),
                ExecContext::for_test(Arc::new(RecordingSink::default())),
            )
            .await,
        "noop execute should reject",
    );

    assert!(error.to_string().contains("noop sandbox rejects exec"));
    assert_eq!(noop.recorded_execs(), vec![spec]);
}

#[tokio::test]
async fn contract_rejects_or_records_workspace_escape_consistently() {
    let mut spec = shell_spec("printf escaped");
    spec.cwd = Some(PathBuf::from("../"));

    let local = LocalSandbox::new(temp_root("escape-local"));
    let local_error = expect_execute_error(
        local
            .execute(
                spec.clone(),
                ExecContext::for_test(Arc::new(RecordingSink::default())),
            )
            .await,
        "local should reject workspace escape",
    );
    assert!(local_error.to_string().contains("workspace path denied"));

    let noop = NoopSandbox::new();
    let noop_error = expect_execute_error(
        noop.execute(
            spec.clone(),
            ExecContext::for_test(Arc::new(RecordingSink::default())),
        )
        .await,
        "noop execute should reject",
    );

    assert!(noop_error.to_string().contains("noop sandbox rejects exec"));
    assert_eq!(noop.recorded_execs(), vec![spec]);
}
