use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    time::Instant,
};

use async_trait::async_trait;
use harness_contracts::{
    now, Event, McpConnectionLostEvent, McpConnectionLostReason, McpConnectionRecoveredEvent,
    SessionId,
};
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};

use crate::{
    ListChangedEvent, McpConnection, McpError, McpPrompt, McpPromptMessages, McpResource,
    McpResourceContents, McpServerScope, McpServerSpec, McpToolDescriptor, McpToolResult,
    McpTransport,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpConnectionState {
    Connecting,
    Ready,
    Reconnecting { attempt: u32, last_error: String },
    Failed { last_error: String },
    Closed,
}

pub trait McpEventSink: Send + Sync + 'static {
    fn emit(&self, event: Event);
}

#[derive(Debug, Default)]
pub struct NoopMcpEventSink;

impl McpEventSink for NoopMcpEventSink {
    fn emit(&self, _event: Event) {}
}

#[derive(Clone)]
pub struct ManagedMcpConnection {
    connection_id: String,
    transport: Arc<dyn McpTransport>,
    spec: McpServerSpec,
    session_id: Option<SessionId>,
    state: Arc<RwLock<McpConnectionState>>,
    connection: Arc<RwLock<Option<Arc<dyn McpConnection>>>>,
    attempts: Arc<AtomicU32>,
    reconnecting: Arc<AtomicBool>,
    downtime_started: Arc<Mutex<Option<Instant>>>,
    event_sink: Arc<dyn McpEventSink>,
}

impl ManagedMcpConnection {
    pub async fn connect(
        transport: Arc<dyn McpTransport>,
        spec: McpServerSpec,
        scope: McpServerScope,
        event_sink: Arc<dyn McpEventSink>,
    ) -> Result<Self, McpError> {
        spec.reconnect.validate()?;
        let session_id = session_id_for_scope(&scope);
        let connection = transport.connect(spec.clone()).await?;
        let managed = Self {
            connection_id: format!("managed:{}", spec.server_id.0),
            transport,
            spec,
            session_id,
            state: Arc::new(RwLock::new(McpConnectionState::Ready)),
            connection: Arc::new(RwLock::new(Some(connection))),
            attempts: Arc::new(AtomicU32::new(0)),
            reconnecting: Arc::new(AtomicBool::new(false)),
            downtime_started: Arc::new(Mutex::new(None)),
            event_sink,
        };
        managed.emit_recovered(true, 0, 0, false);
        Ok(managed)
    }

    pub async fn state(&self) -> McpConnectionState {
        self.state.read().await.clone()
    }

    pub fn attempts_so_far(&self) -> u32 {
        self.attempts.load(Ordering::SeqCst)
    }

    async fn current_connection(&self) -> Result<Arc<dyn McpConnection>, McpError> {
        match &*self.state.read().await {
            McpConnectionState::Ready => {}
            McpConnectionState::Reconnecting { .. } => {
                return Err(McpError::Connection("mcp server reconnecting".into()));
            }
            McpConnectionState::Failed { last_error } => {
                return Err(McpError::Connection(format!(
                    "mcp server failed: {last_error}"
                )));
            }
            McpConnectionState::Closed => {
                return Err(McpError::Connection("mcp server closed".into()));
            }
            McpConnectionState::Connecting => {
                return Err(McpError::Connection("mcp server connecting".into()));
            }
        }

        self.connection
            .read()
            .await
            .clone()
            .ok_or_else(|| McpError::Connection("mcp server missing active connection".into()))
    }

    async fn handle_operation_error(&self, error: McpError) -> McpError {
        if should_reconnect(&error) {
            self.start_reconnect(error.clone()).await;
        }
        error
    }

    async fn start_reconnect(&self, error: McpError) {
        if self.reconnecting.swap(true, Ordering::SeqCst) {
            return;
        }

        let reason = connection_lost_reason(&error);
        let last_error = error.to_string();
        self.attempts.store(0, Ordering::SeqCst);
        *self.downtime_started.lock().await = Some(Instant::now());
        *self.connection.write().await = None;
        *self.state.write().await = McpConnectionState::Reconnecting {
            attempt: 0,
            last_error,
        };
        self.emit_lost(reason, 0, false);

        let this = self.clone();
        tokio::spawn(async move {
            this.reconnect_loop().await;
        });
    }

    async fn reconnect_loop(self) {
        loop {
            if self.state().await == McpConnectionState::Closed {
                return;
            }

            let next_attempt = self.attempts.load(Ordering::SeqCst).saturating_add(1);
            if self
                .spec
                .reconnect
                .is_exhausted(next_attempt.saturating_sub(1))
            {
                self.fail_terminal("reconnect attempts exhausted".to_owned(), next_attempt - 1)
                    .await;
                return;
            }

            tokio::time::sleep(self.spec.reconnect.backoff_for_attempt(next_attempt)).await;
            if self.state().await == McpConnectionState::Closed {
                return;
            }

            match self.transport.connect(self.spec.clone()).await {
                Ok(connection) => {
                    *self.connection.write().await = Some(connection);
                    *self.state.write().await = McpConnectionState::Ready;
                    self.reconnecting.store(false, Ordering::SeqCst);
                    self.attempts.store(next_attempt, Ordering::SeqCst);
                    let downtime_ms = self.take_downtime_ms().await;
                    self.emit_recovered(false, downtime_ms, next_attempt, false);
                    self.spawn_success_reset(next_attempt);
                    return;
                }
                Err(error) => {
                    let attempts_so_far = next_attempt;
                    self.attempts.store(attempts_so_far, Ordering::SeqCst);
                    let last_error = error.to_string();
                    if self.spec.reconnect.is_exhausted(attempts_so_far) {
                        self.fail_terminal(last_error, attempts_so_far).await;
                        return;
                    }
                    *self.state.write().await = McpConnectionState::Reconnecting {
                        attempt: attempts_so_far,
                        last_error,
                    };
                }
            }
        }
    }

    async fn fail_terminal(&self, last_error: String, attempts_so_far: u32) {
        *self.state.write().await = McpConnectionState::Failed {
            last_error: last_error.clone(),
        };
        *self.connection.write().await = None;
        self.reconnecting.store(false, Ordering::SeqCst);
        self.emit_lost(
            McpConnectionLostReason::Other(last_error),
            attempts_so_far,
            true,
        );
    }

    async fn take_downtime_ms(&self) -> u64 {
        self.downtime_started
            .lock()
            .await
            .take()
            .map(|started| started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0)
    }

    fn spawn_success_reset(&self, attempt_snapshot: u32) {
        let attempts = Arc::clone(&self.attempts);
        let state = Arc::clone(&self.state);
        let reset_after = self.spec.reconnect.success_reset_after;
        tokio::spawn(async move {
            tokio::time::sleep(reset_after).await;
            if *state.read().await == McpConnectionState::Ready
                && attempts.load(Ordering::SeqCst) == attempt_snapshot
            {
                attempts.store(0, Ordering::SeqCst);
            }
        });
    }

    fn emit_lost(&self, reason: McpConnectionLostReason, attempts_so_far: u32, terminal: bool) {
        self.event_sink
            .emit(Event::McpConnectionLost(McpConnectionLostEvent {
                session_id: self.session_id,
                server_id: self.spec.server_id.clone(),
                server_source: self.spec.source.clone(),
                reason,
                attempts_so_far,
                terminal,
                at: now(),
            }));
    }

    fn emit_recovered(
        &self,
        was_first: bool,
        total_downtime_ms: u64,
        attempts_used: u32,
        schema_changed: bool,
    ) {
        self.event_sink
            .emit(Event::McpConnectionRecovered(McpConnectionRecoveredEvent {
                session_id: self.session_id,
                server_id: self.spec.server_id.clone(),
                server_source: self.spec.source.clone(),
                was_first,
                total_downtime_ms,
                attempts_used,
                schema_changed,
                at: now(),
            }));
    }
}

#[async_trait]
impl McpConnection for ManagedMcpConnection {
    fn connection_id(&self) -> &str {
        &self.connection_id
    }

    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError> {
        let connection = self.current_connection().await?;
        match connection.list_tools().await {
            Ok(result) => Ok(result),
            Err(error) => Err(self.handle_operation_error(error).await),
        }
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<McpToolResult, McpError> {
        let connection = self.current_connection().await?;
        match connection.call_tool(name, args).await {
            Ok(result) => Ok(result),
            Err(error) => Err(self.handle_operation_error(error).await),
        }
    }

    async fn list_resources(&self) -> Result<Vec<McpResource>, McpError> {
        let connection = self.current_connection().await?;
        match connection.list_resources().await {
            Ok(result) => Ok(result),
            Err(error) => Err(self.handle_operation_error(error).await),
        }
    }

    async fn read_resource(&self, uri: &str) -> Result<McpResourceContents, McpError> {
        let connection = self.current_connection().await?;
        match connection.read_resource(uri).await {
            Ok(result) => Ok(result),
            Err(error) => Err(self.handle_operation_error(error).await),
        }
    }

    async fn list_prompts(&self) -> Result<Vec<McpPrompt>, McpError> {
        let connection = self.current_connection().await?;
        match connection.list_prompts().await {
            Ok(result) => Ok(result),
            Err(error) => Err(self.handle_operation_error(error).await),
        }
    }

    async fn get_prompt(&self, name: &str, args: Value) -> Result<McpPromptMessages, McpError> {
        let connection = self.current_connection().await?;
        match connection.get_prompt(name, args).await {
            Ok(result) => Ok(result),
            Err(error) => Err(self.handle_operation_error(error).await),
        }
    }

    async fn subscribe_changes(&self) -> Result<ListChangedEvent, McpError> {
        self.current_connection().await?.subscribe_changes().await
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        *self.state.write().await = McpConnectionState::Closed;
        self.reconnecting.store(false, Ordering::SeqCst);
        if let Some(connection) = self.connection.write().await.take() {
            connection.shutdown().await?;
        }
        Ok(())
    }
}

fn session_id_for_scope(scope: &McpServerScope) -> Option<SessionId> {
    match scope {
        McpServerScope::Session(session_id) => Some(*session_id),
        McpServerScope::Global | McpServerScope::Agent(_) => None,
    }
}

fn should_reconnect(error: &McpError) -> bool {
    matches!(error, McpError::Connection(_) | McpError::Transport(_))
}

fn connection_lost_reason(error: &McpError) -> McpConnectionLostReason {
    match error {
        McpError::Transport(message) => McpConnectionLostReason::Network(message.clone()),
        McpError::Connection(message) => McpConnectionLostReason::Other(message.clone()),
        _ => McpConnectionLostReason::Other(error.to_string()),
    }
}
