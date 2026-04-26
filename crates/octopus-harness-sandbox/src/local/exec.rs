use std::collections::BTreeSet;
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
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
    ActivityHandle, ExecContext, ExecOutcome, ExecSpec, OutputStream, ProcessHandle,
    ResourceLimitSupport, SandboxBackend, SandboxCapabilities, SessionSnapshotFile, Signal,
    SnapshotSpec, StdioSpec,
};

const BACKEND_ID: &str = "local";

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
    last_activity: Mutex<Instant>,
    stdout_bytes: AtomicU64,
    stderr_bytes: AtomicU64,
    outcome: AsyncMutex<Option<ExecOutcome>>,
    killed_signal: Mutex<Option<Signal>>,
}

impl LocalActivity {
    fn new(child: Child, spec: ExecSpec, ctx: ExecContext, fingerprint: ExecFingerprint) -> Self {
        let now = Instant::now();
        Self {
            child: AsyncMutex::new(Some(child)),
            spec,
            ctx,
            started_at: Utc::now(),
            started_instant: now,
            fingerprint,
            last_activity: Mutex::new(now),
            stdout_bytes: AtomicU64::new(0),
            stderr_bytes: AtomicU64::new(0),
            outcome: AsyncMutex::new(None),
            killed_signal: Mutex::new(None),
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
        *self
            .killed_signal
            .lock()
            .expect("killed signal lock should work")
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

        *self
            .killed_signal
            .lock()
            .expect("killed signal lock should work") = Some(signal);

        if let Some(child) = self.child.lock().await.as_mut() {
            child.start_kill().map_err(sandbox_error)?;
        }
        Ok(())
    }

    fn touch(&self) {
        let previous = {
            let mut last_activity = self
                .last_activity
                .lock()
                .expect("last activity lock should work");
            let previous = *last_activity;
            *last_activity = Instant::now();
            previous
        };

        let _ = self.ctx.event_sink.emit(Event::SandboxActivityHeartbeat(
            SandboxActivityHeartbeatEvent {
                session_id: self.ctx.session_id,
                run_id: self.ctx.run_id,
                tool_use_id: self.ctx.tool_use_id,
                backend_id: BACKEND_ID.to_owned(),
                since_last_io_ms: previous.elapsed().as_millis() as u64,
                at: Utc::now(),
            },
        ));
    }

    fn last_activity(&self) -> Instant {
        *self
            .last_activity
            .lock()
            .expect("last activity lock should work")
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
                        self.ctx
                            .event_sink
                            .emit(Event::SandboxActivityTimeoutFired(
                                SandboxActivityTimeoutFiredEvent {
                                    session_id: self.ctx.session_id,
                                    run_id: self.ctx.run_id,
                                    tool_use_id: self.ctx.tool_use_id,
                                    backend_id: BACKEND_ID.to_owned(),
                                    configured_timeout: self
                                        .spec
                                        .activity_timeout
                                        .expect("activity timeout branch requires configured timeout"),
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

fn timeout_future(
    timeout: Option<Duration>,
    started: Instant,
) -> Pin<Box<dyn Future<Output = WaitInterrupt> + Send>> {
    Box::pin(async move {
        match timeout {
            Some(duration) => {
                tokio::time::sleep_until((started + duration).into()).await;
                WaitInterrupt::Timeout
            }
            None => std::future::pending().await,
        }
    })
}

fn activity_timeout_future<'a>(
    timeout: Option<Duration>,
    activity: &'a LocalActivity,
) -> Pin<Box<dyn Future<Output = WaitInterrupt> + Send + 'a>> {
    Box::pin(async move {
        match timeout {
            Some(duration) => loop {
                let elapsed = activity.last_activity().elapsed();
                if elapsed >= duration {
                    break WaitInterrupt::InactivityTimeout;
                }
                tokio::time::sleep(duration.saturating_sub(elapsed)).await;
            },
            None => std::future::pending().await,
        }
    })
}

enum WaitInterrupt {
    Timeout,
    InactivityTimeout,
}

fn child_stream<R>(
    reader: Option<R>,
    activity: Arc<LocalActivity>,
    stream: OutputStream,
) -> Option<futures::stream::BoxStream<'static, Bytes>>
where
    R: tokio::io::AsyncRead + Send + 'static,
{
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

fn stdio(spec: &StdioSpec) -> Result<Stdio, SandboxError> {
    match spec {
        StdioSpec::Null => Ok(Stdio::null()),
        StdioSpec::Piped => Ok(Stdio::piped()),
        StdioSpec::Inherit => Ok(Stdio::inherit()),
        StdioSpec::File(path) => Err(SandboxError::Message(format!(
            "stdio file endpoint is not implemented in M2-T12: {}",
            path.display()
        ))),
    }
}

fn filtered_env(
    allowed_keys: &BTreeSet<String>,
    spec: &ExecSpec,
) -> impl Iterator<Item = (String, String)> {
    let mut env = std::env::vars()
        .filter(|(key, _)| allowed_keys.contains(key))
        .collect::<std::collections::BTreeMap<_, _>>();
    for (key, value) in &spec.env {
        if allowed_keys.contains(key) {
            env.insert(key.clone(), value.clone());
        }
    }
    env.into_iter()
}

fn resolve_cwd(root: &Path, cwd: Option<&Path>) -> Result<PathBuf, SandboxError> {
    let root = lexical_normalize(root);
    let joined = match cwd {
        Some(cwd) if cwd.is_absolute() => lexical_normalize(cwd),
        Some(cwd) => lexical_normalize(&root.join(cwd)),
        None => root.clone(),
    };

    if joined == root || joined.starts_with(&root) {
        Ok(joined)
    } else {
        Err(SandboxError::Message(format!(
            "workspace path denied: {}",
            joined.display()
        )))
    }
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push("..");
                }
            }
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

fn sandbox_error(error: std::io::Error) -> SandboxError {
    SandboxError::Message(error.to_string())
}
