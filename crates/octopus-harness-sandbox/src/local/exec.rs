use std::collections::BTreeSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use futures::StreamExt;
use harness_contracts::{
    Event, ExecFingerprint, KillScope, SandboxActivityHeartbeatEvent,
    SandboxActivityTimeoutFiredEvent, SandboxError, SandboxExecutionCompletedEvent,
    SandboxExecutionStartedEvent, SandboxExitStatus, SandboxPolicySummary, SessionSnapshotKind,
};
use tokio::process::{Child, Command};
use tokio::sync::Mutex as AsyncMutex;
use tokio_util::io::ReaderStream;

use super::LocalSandbox;
use crate::{
    backend::lexical_normalize_path, ActivityHandle, ExecContext, ExecOutcome, ExecSpec,
    OutputStream, ProcessHandle, ResourceLimitSupport, SandboxBackend, SandboxCapabilities,
    SessionSnapshotFile, Signal, SnapshotSpec, StdioSpec,
};

const BACKEND_ID: &str = "local";
const NO_CACHED_SIGNAL: i32 = i32::MIN;

#[async_trait]
impl SandboxBackend for LocalSandbox {
    fn backend_id(&self) -> &str {
        BACKEND_ID
    }

    fn capabilities(&self) -> SandboxCapabilities {
        SandboxCapabilities {
            supports_streaming: true,
            supports_stdin: true,
            supports_cwd_tracking: false,
            supports_activity_heartbeat: true,
            supports_kill_scope: vec![KillScope::Process],
            snapshot_kinds: BTreeSet::from([SessionSnapshotKind::FilesystemImage]),
            resource_limit_support: ResourceLimitSupport {
                wall_clock: true,
                ..ResourceLimitSupport::default()
            },
            default_timeout: Duration::from_secs(300),
        }
    }

    async fn execute(
        &self,
        spec: ExecSpec,
        ctx: ExecContext,
    ) -> Result<ProcessHandle, SandboxError> {
        let cwd = resolve_cwd(&self.root, spec.cwd.as_deref())?;
        let mut command = Command::new(&spec.command);
        command
            .args(&spec.args)
            .current_dir(cwd)
            .stdin(stdio(&spec.stdin)?)
            .stdout(stdio(&spec.stdout)?)
            .stderr(stdio(&spec.stderr)?)
            .env_clear()
            .envs(filtered_env(&self.base.passthrough_env_keys, &spec));

        let mut child = command.spawn().map_err(sandbox_error)?;
        let pid = child.id();
        let stdin = child
            .stdin
            .take()
            .map(|stdin| Box::pin(stdin) as crate::BoxStdin);
        let stdout_reader = child.stdout.take();
        let stderr_reader = child.stderr.take();
        let fingerprint = spec.canonical_fingerprint(&self.base);

        let activity = Arc::new(LocalActivity::new(
            child,
            spec.clone(),
            ctx.clone(),
            fingerprint,
        ));
        let stdout = child_stream(stdout_reader, Arc::clone(&activity), OutputStream::Stdout);
        let stderr = child_stream(stderr_reader, Arc::clone(&activity), OutputStream::Stderr);

        ctx.event_sink.emit(Event::SandboxExecutionStarted(
            SandboxExecutionStartedEvent {
                session_id: ctx.session_id,
                run_id: ctx.run_id,
                tool_use_id: ctx.tool_use_id,
                backend_id: BACKEND_ID.to_owned(),
                fingerprint,
                policy: SandboxPolicySummary {
                    mode: spec.policy.mode.clone(),
                    scope: spec.policy.scope.clone(),
                    network: spec.policy.network.clone(),
                    resource_limits: spec.policy.resource_limits.clone(),
                },
                at: Utc::now(),
            },
        ))?;

        Ok(ProcessHandle {
            pid,
            stdout,
            stderr,
            stdin,
            cwd_marker: None,
            activity,
        })
    }

    async fn snapshot_session(
        &self,
        _spec: &SnapshotSpec,
    ) -> Result<SessionSnapshotFile, SandboxError> {
        Err(SandboxError::Message(
            "local snapshot is not implemented in M2-T12".to_owned(),
        ))
    }

    async fn restore_session(&self, _snapshot: &SessionSnapshotFile) -> Result<(), SandboxError> {
        Err(SandboxError::Message(
            "local restore is not implemented in M2-T12".to_owned(),
        ))
    }

    async fn shutdown(&self) -> Result<(), SandboxError> {
        Ok(())
    }
}

pub struct LocalActivity {
    pub(crate) child: AsyncMutex<Option<Child>>,
    spec: ExecSpec,
    ctx: ExecContext,
    started_at: chrono::DateTime<Utc>,
    started_instant: Instant,
    fingerprint: ExecFingerprint,
    last_activity_ms: AtomicU64,
    stdout_bytes: AtomicU64,
    stderr_bytes: AtomicU64,
    outcome: AsyncMutex<Option<ExecOutcome>>,
    killed_signal: AtomicI32,
}

impl LocalActivity {
    fn new(child: Child, spec: ExecSpec, ctx: ExecContext, fingerprint: ExecFingerprint) -> Self {
        Self {
            child: AsyncMutex::new(Some(child)),
            spec,
            ctx,
            started_at: Utc::now(),
            started_instant: Instant::now(),
            fingerprint,
            last_activity_ms: AtomicU64::new(0),
            stdout_bytes: AtomicU64::new(0),
            stderr_bytes: AtomicU64::new(0),
            outcome: AsyncMutex::new(None),
            killed_signal: AtomicI32::new(NO_CACHED_SIGNAL),
        }
    }

    fn observe_output(&self, stream: OutputStream, bytes: u64) {
        match stream {
            OutputStream::Stdout => {
                self.stdout_bytes.fetch_add(bytes, Ordering::Relaxed);
            }
            OutputStream::Stderr => {
                self.stderr_bytes.fetch_add(bytes, Ordering::Relaxed);
            }
            OutputStream::Combined => {}
        }
        self.touch();
    }

    fn cached_signal(&self) -> Option<Signal> {
        match self.killed_signal.load(Ordering::Relaxed) {
            NO_CACHED_SIGNAL => None,
            signal => Some(signal),
        }
    }

    fn elapsed_since_start_ms(&self) -> u64 {
        self.started_instant.elapsed().as_millis() as u64
    }
}

#[async_trait]
impl ActivityHandle for LocalActivity {
    async fn wait(&self) -> Result<ExecOutcome, SandboxError> {
        if let Some(outcome) = self.outcome.lock().await.clone() {
            return Ok(outcome);
        }

        let mut child = self
            .child
            .lock()
            .await
            .take()
            .ok_or_else(|| SandboxError::Message("local process already claimed".to_owned()))?;

        let exit_status = self.wait_child(&mut child).await?;
        let outcome = ExecOutcome {
            exit_status,
            started_at: self.started_at,
            finished_at: Utc::now(),
            stdout_bytes_observed: self.stdout_bytes.load(Ordering::Relaxed),
            stderr_bytes_observed: self.stderr_bytes.load(Ordering::Relaxed),
            overflow: None,
        };

        self.ctx.event_sink.emit(Event::SandboxExecutionCompleted(
            SandboxExecutionCompletedEvent {
                session_id: self.ctx.session_id,
                run_id: self.ctx.run_id,
                tool_use_id: self.ctx.tool_use_id,
                backend_id: BACKEND_ID.to_owned(),
                fingerprint: self.fingerprint,
                exit_status: outcome.exit_status.clone(),
                stdout_bytes_observed: outcome.stdout_bytes_observed,
                stderr_bytes_observed: outcome.stderr_bytes_observed,
                duration_ms: self.started_instant.elapsed().as_millis() as u64,
                overflow: None,
                at: Utc::now(),
            },
        ))?;

        *self.outcome.lock().await = Some(outcome.clone());
        Ok(outcome)
    }

    async fn kill(&self, signal: Signal, scope: KillScope) -> Result<(), SandboxError> {
        if scope != KillScope::Process {
            return Err(SandboxError::Message(format!(
                "unsupported kill scope for local T12: {scope:?}"
            )));
        }

        self.killed_signal.store(signal, Ordering::Relaxed);
        if let Some(child) = self.child.lock().await.as_mut() {
            child.start_kill().map_err(sandbox_error)?;
        }
        Ok(())
    }

    fn touch(&self) {
        let previous = self
            .last_activity_ms
            .swap(self.elapsed_since_start_ms(), Ordering::Relaxed);
        let _ = self.ctx.event_sink.emit(Event::SandboxActivityHeartbeat(
            SandboxActivityHeartbeatEvent {
                session_id: self.ctx.session_id,
                run_id: self.ctx.run_id,
                tool_use_id: self.ctx.tool_use_id,
                backend_id: BACKEND_ID.to_owned(),
                since_last_io_ms: self.elapsed_since_start_ms().saturating_sub(previous),
                at: Utc::now(),
            },
        ));
    }

    fn last_activity(&self) -> Instant {
        let elapsed = Duration::from_millis(self.last_activity_ms.load(Ordering::Relaxed));
        self.started_instant + elapsed
    }
}

impl LocalActivity {
    async fn wait_child(&self, child: &mut Child) -> Result<SandboxExitStatus, SandboxError> {
        let timeout = timeout_future(self.spec.timeout, self.started_instant);
        let activity_timeout = activity_timeout_future(self.spec.activity_timeout, self);

        tokio::select! {
            result = child.wait() => {
                match result {
                    Ok(status) => {
                        if let Some(signal) = self.cached_signal() {
                            Ok(SandboxExitStatus::Signal(signal))
                        } else if let Some(code) = status.code() {
                            Ok(SandboxExitStatus::Code(code))
                        } else {
                            Ok(SandboxExitStatus::BackendError)
                        }
                    }
                    Err(error) => Err(sandbox_error(error)),
                }
            }
            interrupt = timeout => {
                match interrupt {
                    WaitInterrupt::Timeout => {
                        child.start_kill().map_err(sandbox_error)?;
                        let _ = child.wait().await;
                        Ok(SandboxExitStatus::Timeout)
                    }
                    WaitInterrupt::InactivityTimeout => unreachable!("timeout future cannot return inactivity"),
                }
            }
            interrupt = activity_timeout => {
                match interrupt {
                    WaitInterrupt::InactivityTimeout => {
                        child.start_kill().map_err(sandbox_error)?;
                        let _ = child.wait().await;
                        self.ctx.event_sink.emit(Event::SandboxActivityTimeoutFired(
                            SandboxActivityTimeoutFiredEvent {
                                session_id: self.ctx.session_id,
                                run_id: self.ctx.run_id,
                                tool_use_id: self.ctx.tool_use_id,
                                backend_id: BACKEND_ID.to_owned(),
                                configured_timeout: self.spec.activity_timeout.unwrap_or_default(),
                                kill_scope: KillScope::Process,
                                at: Utc::now(),
                            },
                        ))?;
                        Ok(SandboxExitStatus::InactivityTimeout)
                    }
                    WaitInterrupt::Timeout => unreachable!("activity timeout future cannot return timeout"),
                }
            }
        }
    }
}

enum WaitInterrupt {
    Timeout,
    InactivityTimeout,
}

fn timeout_future(
    timeout: Option<Duration>,
    started: Instant,
) -> Pin<Box<dyn Future<Output = WaitInterrupt> + Send>> {
    Box::pin(async move {
        match timeout {
            Some(timeout) => {
                let deadline = started + timeout;
                tokio::time::sleep_until(deadline.into()).await;
                WaitInterrupt::Timeout
            }
            None => std::future::pending().await,
        }
    })
}

fn activity_timeout_future(
    timeout: Option<Duration>,
    activity: &LocalActivity,
) -> Pin<Box<dyn Future<Output = WaitInterrupt> + Send + '_>> {
    Box::pin(async move {
        match timeout {
            Some(timeout) => loop {
                let elapsed = activity.last_activity().elapsed();
                if elapsed >= timeout {
                    break WaitInterrupt::InactivityTimeout;
                }
                tokio::time::sleep(timeout.saturating_sub(elapsed)).await;
            },
            None => std::future::pending().await,
        }
    })
}

fn child_stream(
    reader: Option<impl tokio::io::AsyncRead + Send + 'static>,
    activity: Arc<LocalActivity>,
    stream: OutputStream,
) -> Option<futures::stream::BoxStream<'static, Bytes>> {
    reader.map(|reader| {
        ReaderStream::new(reader)
            .filter_map(move |chunk| {
                let activity = Arc::clone(&activity);
                async move {
                    match chunk {
                        Ok(bytes) => {
                            activity.observe_output(stream, bytes.len() as u64);
                            Some(bytes)
                        }
                        Err(_) => None,
                    }
                }
            })
            .boxed()
    })
}

fn filtered_env<'a>(
    allowed: &'a BTreeSet<String>,
    spec: &'a ExecSpec,
) -> impl Iterator<Item = (&'a String, &'a String)> + 'a {
    spec.env
        .iter()
        .filter(|(key, _)| allowed.contains(key.as_str()))
}

fn stdio(spec: &StdioSpec) -> Result<Stdio, SandboxError> {
    match spec {
        StdioSpec::Null => Ok(Stdio::null()),
        StdioSpec::Piped => Ok(Stdio::piped()),
        StdioSpec::Inherit => Ok(Stdio::inherit()),
        StdioSpec::File(path) => {
            let file = std::fs::File::create(path).map_err(sandbox_error)?;
            Ok(Stdio::from(file))
        }
    }
}

fn resolve_cwd(root: &Path, cwd: Option<&Path>) -> Result<PathBuf, SandboxError> {
    let relative = cwd.map_or_else(PathBuf::new, lexical_normalize_path);
    if relative.is_absolute() || relative.starts_with("..") {
        return Err(SandboxError::Message(format!(
            "workspace path denied: {}",
            relative.display()
        )));
    }
    Ok(root.join(relative))
}

fn sandbox_error(error: std::io::Error) -> SandboxError {
    SandboxError::Message(error.to_string())
}
