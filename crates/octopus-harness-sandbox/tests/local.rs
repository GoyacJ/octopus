#![cfg(all(feature = "local", unix))]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::StreamExt;
use harness_contracts::{Event, KillScope, SandboxError, SandboxExitStatus};
use harness_sandbox::{
    EventSink, ExecContext, ExecSpec, LocalSandbox, SandboxBackend, SandboxBaseConfig, StdioSpec,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

struct RecordingSink {
    tx: UnboundedSender<Event>,
}

struct NullSink;

impl EventSink for NullSink {
    fn emit(&self, _event: Event) -> Result<(), SandboxError> {
        Ok(())
    }
}

impl EventSink for RecordingSink {
    fn emit(&self, event: Event) -> Result<(), SandboxError> {
        self.tx
            .send(event)
            .map_err(|error| SandboxError::Message(error.to_string()))
    }
}

fn recording_sink() -> (Arc<RecordingSink>, UnboundedReceiver<Event>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (Arc::new(RecordingSink { tx }), rx)
}

fn drain_events(rx: &mut UnboundedReceiver<Event>) -> Vec<Event> {
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    events
}

fn temp_root(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "octopus-harness-sandbox-{name}-{}-{unique}",
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

async fn collect_stdout(mut stdout: futures::stream::BoxStream<'static, bytes::Bytes>) -> String {
    let mut bytes = Vec::new();
    while let Some(chunk) = stdout.next().await {
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).expect("stdout should be utf8")
}

#[tokio::test]
async fn local_sandbox_is_object_safe_and_streams_stdout() {
    let root = temp_root("echo");
    let (sink, mut rx) = recording_sink();
    let ctx = ExecContext::for_test(sink);
    let sandbox: Arc<dyn SandboxBackend> = Arc::new(LocalSandbox::new(&root));

    let mut handle = sandbox
        .execute(shell_spec("printf hello"), ctx)
        .await
        .expect("execute should spawn process");
    let stdout = handle.stdout.take().expect("stdout should be piped");
    let output = collect_stdout(stdout).await;
    let outcome = handle.activity.wait().await.expect("wait should succeed");

    assert_eq!(output, "hello");
    assert_eq!(outcome.exit_status, SandboxExitStatus::Code(0));
    assert_eq!(outcome.stdout_bytes_observed, 5);

    let events = drain_events(&mut rx);
    assert!(events
        .iter()
        .any(|event| matches!(event, Event::SandboxExecutionStarted(_))));
    assert!(events
        .iter()
        .any(|event| matches!(event, Event::SandboxExecutionCompleted(_))));
}

#[tokio::test]
async fn local_sandbox_emits_activity_heartbeat_when_output_is_observed() {
    let root = temp_root("heartbeat");
    let (sink, mut rx) = recording_sink();
    let sandbox = LocalSandbox::new(&root);

    let mut handle = sandbox
        .execute(shell_spec("printf hello"), ExecContext::for_test(sink))
        .await
        .expect("execute should spawn process");
    let stdout = handle.stdout.take().expect("stdout should be piped");
    let output = collect_stdout(stdout).await;
    let outcome = handle.activity.wait().await.expect("wait should succeed");

    assert_eq!(output, "hello");
    assert_eq!(outcome.exit_status, SandboxExitStatus::Code(0));
    assert!(drain_events(&mut rx).iter().any(|event| matches!(
        event,
        Event::SandboxActivityHeartbeat(heartbeat)
            if heartbeat.backend_id == "local" && heartbeat.since_last_io_ms <= 5_000
    )));
}

#[tokio::test]
async fn local_sandbox_applies_relative_cwd_inside_root_and_rejects_escape() {
    let root = temp_root("cwd");
    std::fs::create_dir_all(root.join("child")).expect("child dir should be created");
    let sandbox = LocalSandbox::new(&root);
    let ctx = ExecContext::for_test(Arc::new(NullSink));

    let mut spec = shell_spec("printf '%s' \"$(basename \"$PWD\")\"");
    spec.cwd = Some(PathBuf::from("./child/../child"));
    let mut handle = sandbox
        .execute(spec, ctx.clone())
        .await
        .expect("cwd inside root should spawn");
    let output = collect_stdout(handle.stdout.take().expect("stdout should be piped")).await;
    let outcome = handle.activity.wait().await.expect("wait should succeed");
    assert_eq!(output, "child");
    assert_eq!(outcome.exit_status, SandboxExitStatus::Code(0));

    let mut escaping = shell_spec("printf nope");
    escaping.cwd = Some(PathBuf::from("../"));
    let error = match sandbox.execute(escaping, ctx).await {
        Ok(_) => panic!("cwd escape should be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("workspace path denied"));
}

#[tokio::test]
async fn local_sandbox_filters_environment_with_passthrough_keys() {
    let root = temp_root("env");
    let sandbox = LocalSandbox::with_base(
        &root,
        SandboxBaseConfig {
            passthrough_env_keys: BTreeSet::from(["VISIBLE".to_owned()]),
            ..SandboxBaseConfig::default()
        },
    );

    let mut spec = shell_spec("printf '%s:%s' \"${VISIBLE:-missing}\" \"${HIDDEN:-missing}\"");
    spec.env.insert("VISIBLE".to_owned(), "yes".to_owned());
    spec.env.insert("HIDDEN".to_owned(), "no".to_owned());

    let mut handle = sandbox
        .execute(spec, ExecContext::for_test(Arc::new(NullSink)))
        .await
        .expect("execute should spawn process");
    let output = collect_stdout(handle.stdout.take().expect("stdout should be piped")).await;
    let outcome = handle.activity.wait().await.expect("wait should succeed");

    assert_eq!(output, "yes:missing");
    assert_eq!(outcome.exit_status, SandboxExitStatus::Code(0));
}

#[tokio::test]
async fn local_sandbox_timeout_and_activity_timeout_kill_processes() {
    let root = temp_root("timeouts");
    let (sink, mut rx) = recording_sink();
    let sandbox = LocalSandbox::new(&root);

    let mut timed = shell_spec("sleep 5");
    timed.timeout = Some(Duration::from_millis(50));
    let handle = sandbox
        .execute(timed, ExecContext::for_test(sink.clone()))
        .await
        .expect("execute should spawn timed process");
    let outcome = handle.activity.wait().await.expect("wait should succeed");
    assert_eq!(outcome.exit_status, SandboxExitStatus::Timeout);

    let mut inactive = shell_spec("sleep 5");
    inactive.activity_timeout = Some(Duration::from_millis(50));
    let handle = sandbox
        .execute(inactive, ExecContext::for_test(sink))
        .await
        .expect("execute should spawn inactive process");
    let outcome = handle.activity.wait().await.expect("wait should succeed");
    assert_eq!(outcome.exit_status, SandboxExitStatus::InactivityTimeout);

    assert!(drain_events(&mut rx)
        .iter()
        .any(|event| matches!(event, Event::SandboxActivityTimeoutFired(_))));
}

#[tokio::test]
async fn local_sandbox_only_supports_process_kill_scope_in_t12() {
    let root = temp_root("kill-scope");
    let sandbox = LocalSandbox::new(&root);
    let mut spec = shell_spec("sleep 5");
    spec.timeout = Some(Duration::from_secs(5));
    let handle = sandbox
        .execute(spec, ExecContext::for_test(Arc::new(NullSink)))
        .await
        .expect("execute should spawn process");

    let error = handle
        .activity
        .kill(15, KillScope::ProcessGroup)
        .await
        .expect_err("process group kill is not implemented in T12");
    assert!(error.to_string().contains("unsupported kill scope"));

    handle
        .activity
        .kill(15, KillScope::Process)
        .await
        .expect("process kill should be supported");
    let outcome = handle.activity.wait().await.expect("wait should succeed");
    assert_eq!(outcome.exit_status, SandboxExitStatus::Signal(15));
}
