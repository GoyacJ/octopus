use std::collections::BTreeSet;
use std::future::Future;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
#[cfg(unix)]
use command_fds::{CommandFdExt, FdMapping};
use futures::StreamExt;
use harness_contracts::{
    BlobRef, Event, ExecFingerprint, KillScope, SandboxActivityHeartbeatEvent,
    SandboxActivityTimeoutFiredEvent, SandboxBackpressureAppliedEvent, SandboxError,
    SandboxExecutionCompletedEvent, SandboxExecutionStartedEvent, SandboxExitStatus,
    SandboxOutputSpilledEvent, SandboxOutputStream, SandboxOverflowSummary, SandboxPolicySummary,
    SandboxSnapshotCreatedEvent, SessionSnapshotKind,
};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex as AsyncMutex};
use tokio_util::io::ReaderStream;

use super::LocalSandbox;
use crate::cwd::CwdMarkerLine;
use crate::{
    backend::lexical_normalize_path, ActivityHandle, ExecContext, ExecOutcome, ExecSpec,
    OutputOverflow, OutputOverflowPolicy, OutputStream, ProcessHandle, ResourceLimitSupport,
    SandboxBackend, SandboxCapabilities, SessionSnapshotFile, Signal, SnapshotMetadata,
    SnapshotSpec, StdioSpec,
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
            supports_cwd_tracking: cfg!(unix),
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
        let (mut command, cwd_marker) = command_with_cwd_marker(&spec)?;
        command
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
            cwd_marker,
            activity,
        })
    }

    async fn snapshot_session(
        &self,
        spec: &SnapshotSpec,
    ) -> Result<SessionSnapshotFile, SandboxError> {
        let snapshot = create_filesystem_snapshot(&self.root, spec)?;
        if let Some(event_sink) = &self.snapshot_event_sink {
            event_sink.emit(Event::SandboxSnapshotCreated(SandboxSnapshotCreatedEvent {
                session_id: snapshot.session_id,
                backend_id: BACKEND_ID.to_owned(),
                kind: snapshot.kind,
                size_bytes: snapshot.metadata.size_bytes,
                content_hash: snapshot.metadata.content_hash,
                at: Utc::now(),
            }))?;
        }
        Ok(snapshot)
    }

    async fn restore_session(&self, snapshot: &SessionSnapshotFile) -> Result<(), SandboxError> {
        restore_filesystem_snapshot(&self.root, snapshot)
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
    overflow: AsyncMutex<Option<OutputOverflow>>,
    spill: AsyncMutex<Option<SpillState>>,
    killed_signal: AtomicI32,
}

struct SpillState {
    stream: OutputStream,
    path: PathBuf,
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
            overflow: AsyncMutex::new(None),
            spill: AsyncMutex::new(None),
            killed_signal: AtomicI32::new(NO_CACHED_SIGNAL),
        }
    }

    async fn process_output(&self, stream: OutputStream, bytes: Bytes) -> Option<Bytes> {
        let previous = match stream {
            OutputStream::Stdout => self
                .stdout_bytes
                .fetch_add(bytes.len() as u64, Ordering::Relaxed),
            OutputStream::Stderr => self
                .stderr_bytes
                .fetch_add(bytes.len() as u64, Ordering::Relaxed),
            OutputStream::Combined => 0,
        };
        self.touch();

        let limit = self.spec.output_policy.max_inline_bytes;
        let observed = previous + bytes.len() as u64;
        if observed <= limit {
            return Some(bytes);
        }

        let new_overflow = self.record_overflow(stream, observed, limit, None).await;
        if new_overflow {
            self.emit_backpressure(observed.saturating_sub(limit), Duration::ZERO);
        }

        match self.spec.output_policy.overflow {
            OutputOverflowPolicy::Truncate => prefix_within_limit(bytes, previous, limit),
            OutputOverflowPolicy::SpillToBlob => {
                let overflow_bytes = bytes_after_limit(&bytes, previous, limit);
                if !overflow_bytes.is_empty() {
                    let _ = self.append_spill(stream, &overflow_bytes).await;
                }
                prefix_within_limit(bytes, previous, limit)
            }
            OutputOverflowPolicy::AbortExec => {
                if let Some(child) = self.child.lock().await.as_mut() {
                    let _ = child.start_kill();
                }
                None
            }
        }
    }

    async fn record_overflow(
        &self,
        stream: OutputStream,
        original_bytes: u64,
        effective_limit: u64,
        blob_ref: Option<BlobRef>,
    ) -> bool {
        let mut overflow = self.overflow.lock().await;
        if overflow.is_some() {
            return false;
        }
        *overflow = Some(OutputOverflow {
            stream,
            original_bytes,
            effective_limit,
            blob_ref,
        });
        true
    }

    async fn append_spill(&self, stream: OutputStream, bytes: &[u8]) -> Result<(), SandboxError> {
        let path = {
            let mut spill = self.spill.lock().await;
            if spill.is_none() {
                let blob_id = harness_contracts::BlobId::new();
                let dir = self
                    .ctx
                    .workspace_root
                    .join(".octopus")
                    .join("sandbox-output");
                std::fs::create_dir_all(&dir).map_err(sandbox_error)?;
                *spill = Some(SpillState {
                    stream,
                    path: dir.join(format!("{blob_id}.bin")),
                });
            }
            spill.as_ref().expect("spill initialized").path.clone()
        };

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(sandbox_error)?;
        file.write_all(bytes).map_err(sandbox_error)
    }

    async fn finalize_overflow(&self) -> Result<Option<OutputOverflow>, SandboxError> {
        let mut overflow = self.overflow.lock().await;
        let Some(mut overflow_value) = overflow.clone() else {
            return Ok(None);
        };
        overflow_value.original_bytes = match overflow_value.stream {
            OutputStream::Stdout => self.stdout_bytes.load(Ordering::Relaxed),
            OutputStream::Stderr => self.stderr_bytes.load(Ordering::Relaxed),
            OutputStream::Combined => {
                self.stdout_bytes.load(Ordering::Relaxed)
                    + self.stderr_bytes.load(Ordering::Relaxed)
            }
        };

        if let Some(spill) = self.spill.lock().await.as_ref() {
            let blob_ref = blob_ref_for_file(&spill.path)?;
            self.ctx
                .event_sink
                .emit(Event::SandboxOutputSpilled(SandboxOutputSpilledEvent {
                    session_id: self.ctx.session_id,
                    run_id: self.ctx.run_id,
                    tool_use_id: self.ctx.tool_use_id,
                    stream: sandbox_output_stream(spill.stream),
                    blob_ref: blob_ref.clone(),
                    head_bytes: self
                        .spec
                        .output_policy
                        .max_inline_bytes
                        .min(u64::from(u32::MAX)) as u32,
                    tail_bytes: blob_ref.size.min(u64::from(u32::MAX)) as u32,
                    original_bytes: overflow_value.original_bytes,
                    at: Utc::now(),
                }))?;
            overflow_value.blob_ref = Some(blob_ref);
        }

        *overflow = Some(overflow_value.clone());
        Ok(Some(overflow_value))
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

    fn emit_backpressure(&self, queued_bytes: u64, paused_for: Duration) {
        let _ = self.ctx.event_sink.emit(Event::SandboxBackpressureApplied(
            SandboxBackpressureAppliedEvent {
                session_id: self.ctx.session_id,
                run_id: self.ctx.run_id,
                tool_use_id: self.ctx.tool_use_id,
                queued_bytes,
                paused_for_ms: paused_for.as_millis() as u64,
                at: Utc::now(),
            },
        ));
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
        let overflow = self.finalize_overflow().await?;
        let budget_exceeded = self.spec.output_policy.overflow == OutputOverflowPolicy::AbortExec
            && overflow.is_some();
        let outcome = ExecOutcome {
            exit_status,
            started_at: self.started_at,
            finished_at: Utc::now(),
            stdout_bytes_observed: self.stdout_bytes.load(Ordering::Relaxed),
            stderr_bytes_observed: self.stderr_bytes.load(Ordering::Relaxed),
            overflow: overflow.clone(),
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
                overflow: overflow.map(sandbox_overflow_summary),
                at: Utc::now(),
            },
        ))?;

        *self.outcome.lock().await = Some(outcome.clone());
        if budget_exceeded {
            return Err(SandboxError::Message(
                "output budget exceeded; process aborted".to_owned(),
            ));
        }
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
        let (tx, rx) = mpsc::channel(1);
        tokio::spawn(async move {
            let reader = ReaderStream::new(reader);
            futures::pin_mut!(reader);
            while let Some(chunk) = reader.next().await {
                let bytes = match chunk {
                    Ok(bytes) => bytes,
                    Err(_) => break,
                };
                let Some(bytes) = activity.process_output(stream, bytes).await else {
                    continue;
                };
                match tx.try_send(bytes) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(bytes)) => {
                        let started = Instant::now();
                        if tx.send(bytes).await.is_err() {
                            break;
                        }
                        activity.emit_backpressure(1, started.elapsed());
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => break,
                }
            }
        });
        futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|bytes| (bytes, rx))
        })
        .boxed()
    })
}

fn command_with_cwd_marker(
    spec: &ExecSpec,
) -> Result<
    (
        Command,
        Option<futures::stream::BoxStream<'static, CwdMarkerLine>>,
    ),
    SandboxError,
> {
    #[cfg(unix)]
    {
        if let Some(script) = shell_script(spec) {
            let (reader, writer) = os_pipe::pipe().map_err(sandbox_error)?;
            let mut command = Command::new(&spec.command);
            command.arg("-c").arg(wrap_shell_script_for_cwd(script));
            command
                .fd_mappings(vec![FdMapping {
                    parent_fd: writer.into(),
                    child_fd: 3,
                }])
                .map_err(|error| SandboxError::Message(error.to_string()))?;
            return Ok((command, Some(cwd_marker_stream(reader))));
        }
    }

    let mut command = Command::new(&spec.command);
    command.args(&spec.args);
    Ok((command, None))
}

#[cfg(unix)]
fn shell_script(spec: &ExecSpec) -> Option<&str> {
    let command = Path::new(&spec.command).file_name()?.to_str()?;
    if !matches!(command, "sh" | "bash" | "zsh") {
        return None;
    }
    if spec.args.first().map(String::as_str) != Some("-c") {
        return None;
    }
    spec.args.get(1).map(String::as_str)
}

#[cfg(unix)]
fn wrap_shell_script_for_cwd(script: &str) -> String {
    format!(
        "{script}\n__octopus_status=$?\nprintf '1\\t%s\\n' \"$PWD\" >&3\nexit $__octopus_status"
    )
}

#[cfg(unix)]
fn cwd_marker_stream(
    reader: os_pipe::PipeReader,
) -> futures::stream::BoxStream<'static, CwdMarkerLine> {
    futures::stream::once(async move {
        let line = tokio::task::spawn_blocking(move || {
            let mut reader = reader;
            let mut line = String::new();
            reader.read_to_string(&mut line).map(|_| line)
        })
        .await
        .ok()?
        .ok()?;
        parse_cwd_marker_line(line.lines().next().unwrap_or_default())
    })
    .filter_map(|line| async move { line })
    .boxed()
}

#[cfg(unix)]
fn parse_cwd_marker_line(line: &str) -> Option<CwdMarkerLine> {
    let (sequence, cwd) = line.trim_end().split_once('\t')?;
    Some(CwdMarkerLine {
        sequence: sequence.parse().ok()?,
        cwd: PathBuf::from(cwd),
        at: Utc::now(),
    })
}

fn prefix_within_limit(bytes: Bytes, previous: u64, limit: u64) -> Option<Bytes> {
    if previous >= limit {
        return None;
    }
    let allowed = (limit - previous).min(bytes.len() as u64) as usize;
    Some(bytes.slice(..allowed))
}

fn bytes_after_limit(bytes: &Bytes, previous: u64, limit: u64) -> Bytes {
    if previous + bytes.len() as u64 <= limit {
        return Bytes::new();
    }
    let start = limit.saturating_sub(previous).min(bytes.len() as u64) as usize;
    bytes.slice(start..)
}

fn blob_ref_for_file(path: &Path) -> Result<BlobRef, SandboxError> {
    let bytes = std::fs::read(path).map_err(sandbox_error)?;
    let hash = blake3::hash(&bytes);
    Ok(BlobRef {
        id: harness_contracts::BlobId::new(),
        size: bytes.len() as u64,
        content_hash: *hash.as_bytes(),
        content_type: Some("application/octet-stream".to_owned()),
    })
}

fn sandbox_output_stream(stream: OutputStream) -> SandboxOutputStream {
    match stream {
        OutputStream::Stdout => SandboxOutputStream::Stdout,
        OutputStream::Stderr => SandboxOutputStream::Stderr,
        OutputStream::Combined => SandboxOutputStream::Combined,
    }
}

fn sandbox_overflow_summary(overflow: OutputOverflow) -> SandboxOverflowSummary {
    SandboxOverflowSummary {
        stream: sandbox_output_stream(overflow.stream),
        original_bytes: overflow.original_bytes,
        effective_limit: overflow.effective_limit,
        blob_ref: overflow.blob_ref,
    }
}

fn filtered_env<'a>(
    allowed: &'a BTreeSet<String>,
    spec: &'a ExecSpec,
) -> impl Iterator<Item = (&'a String, &'a String)> + 'a {
    spec.env
        .iter()
        .filter(|(key, _)| allowed.contains(key.as_str()))
}

fn create_filesystem_snapshot(
    root: &Path,
    spec: &SnapshotSpec,
) -> Result<SessionSnapshotFile, SandboxError> {
    if spec.kind != SessionSnapshotKind::FilesystemImage {
        return Err(SandboxError::Message(format!(
            "local snapshot kind is unsupported: {:?}",
            spec.kind
        )));
    }

    let target_path = spec.target_path.clone().unwrap_or_else(|| {
        root.join(".octopus")
            .join("snapshots")
            .join(format!("{}.tar", spec.session_id))
    });
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).map_err(sandbox_error)?;
    }

    let file = std::fs::File::create(&target_path).map_err(sandbox_error)?;
    let mut builder = tar::Builder::new(file);
    append_snapshot_entries(&mut builder, root, root, &target_path)?;
    builder.finish().map_err(sandbox_error)?;

    let metadata = snapshot_metadata(&target_path)?;
    Ok(SessionSnapshotFile {
        session_id: spec.session_id,
        kind: spec.kind,
        path: target_path,
        metadata,
    })
}

fn append_snapshot_entries(
    builder: &mut tar::Builder<std::fs::File>,
    root: &Path,
    current: &Path,
    target_path: &Path,
) -> Result<(), SandboxError> {
    for entry in std::fs::read_dir(current).map_err(sandbox_error)? {
        let entry = entry.map_err(sandbox_error)?;
        let path = entry.path();
        if path == target_path || path.starts_with(root.join(".octopus").join("snapshots")) {
            continue;
        }
        let relative = path.strip_prefix(root).map_err(|error| {
            SandboxError::Message(format!("snapshot path escaped root: {error}"))
        })?;
        if path.is_dir() {
            builder.append_dir(relative, &path).map_err(sandbox_error)?;
            append_snapshot_entries(builder, root, &path, target_path)?;
        } else if path.is_file() {
            builder
                .append_path_with_name(&path, relative)
                .map_err(sandbox_error)?;
        }
    }
    Ok(())
}

fn restore_filesystem_snapshot(
    root: &Path,
    snapshot: &SessionSnapshotFile,
) -> Result<(), SandboxError> {
    if snapshot.kind != SessionSnapshotKind::FilesystemImage {
        return Err(SandboxError::Message(format!(
            "local restore kind is unsupported: {:?}",
            snapshot.kind
        )));
    }

    validate_snapshot_archive(&snapshot.path)?;
    clear_root_for_restore(root, &snapshot.path)?;

    let file = std::fs::File::open(&snapshot.path).map_err(sandbox_error)?;
    let mut archive = tar::Archive::new(file);
    for entry in archive.entries().map_err(sandbox_error)? {
        let mut entry = entry.map_err(sandbox_error)?;
        let path = entry.path().map_err(sandbox_error)?;
        ensure_relative_archive_path(&path)?;
        entry.unpack_in(root).map_err(sandbox_error)?;
    }
    Ok(())
}

fn validate_snapshot_archive(path: &Path) -> Result<(), SandboxError> {
    let file = std::fs::File::open(path).map_err(sandbox_error)?;
    let mut archive = tar::Archive::new(file);
    for entry in archive.entries().map_err(sandbox_error)? {
        let entry = entry.map_err(sandbox_error)?;
        let path = entry.path().map_err(sandbox_error)?;
        ensure_relative_archive_path(&path)?;
    }
    Ok(())
}

fn ensure_relative_archive_path(path: &Path) -> Result<(), SandboxError> {
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(SandboxError::Message(format!(
            "snapshot path escapes sandbox root: {}",
            path.display()
        )));
    }
    Ok(())
}

fn clear_root_for_restore(root: &Path, snapshot_path: &Path) -> Result<(), SandboxError> {
    std::fs::create_dir_all(root).map_err(sandbox_error)?;
    for entry in std::fs::read_dir(root).map_err(sandbox_error)? {
        let entry = entry.map_err(sandbox_error)?;
        let path = entry.path();
        if path == snapshot_path || snapshot_path.starts_with(&path) {
            continue;
        }
        if path.is_dir() {
            std::fs::remove_dir_all(path).map_err(sandbox_error)?;
        } else {
            std::fs::remove_file(path).map_err(sandbox_error)?;
        }
    }
    Ok(())
}

fn snapshot_metadata(path: &Path) -> Result<SnapshotMetadata, SandboxError> {
    let mut file = std::fs::File::open(path).map_err(sandbox_error)?;
    let mut hasher = blake3::Hasher::new();
    let mut size = 0;
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer).map_err(sandbox_error)?;
        if read == 0 {
            break;
        }
        size += read as u64;
        hasher.update(&buffer[..read]);
    }
    Ok(SnapshotMetadata {
        size_bytes: size,
        content_hash: *hasher.finalize().as_bytes(),
        created_at: Utc::now(),
    })
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
