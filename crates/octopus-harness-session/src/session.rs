use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use harness_contracts::{
    ConfigHash, EndReason, Event, SessionCreatedEvent, SessionEndedEvent, SessionError, SessionId,
    SnapshotId, TenantId, UsageSnapshot,
};
use harness_journal::EventStore;
use tokio::sync::{watch, Mutex};

#[cfg(feature = "steering")]
use crate::SteeringQueue;
use crate::{SessionBuilder, SessionPaths, SessionProjection, SessionTurnRuntime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionOptions {
    pub workspace_root: PathBuf,
    pub tenant_id: TenantId,
    pub session_id: SessionId,
}

impl SessionOptions {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: root.into(),
            tenant_id: TenantId::SINGLE,
            session_id: SessionId::new(),
        }
    }

    #[must_use]
    pub fn with_tenant_id(mut self, tenant_id: TenantId) -> Self {
        self.tenant_id = tenant_id;
        self
    }

    #[must_use]
    pub fn with_session_id(mut self, session_id: SessionId) -> Self {
        self.session_id = session_id;
        self
    }
}

pub struct Session {
    options: SessionOptions,
    paths: SessionPaths,
    event_store: Arc<dyn EventStore>,
    snapshot_tx: watch::Sender<SnapshotId>,
    snapshot_rx: watch::Receiver<SnapshotId>,
    turn_runtime: Option<SessionTurnRuntime>,
    #[cfg(feature = "steering")]
    steering: SteeringQueue,
    state: Mutex<SessionState>,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("options", &self.options)
            .field("paths", &self.paths)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
struct SessionState {
    ended: bool,
    projection: SessionProjection,
}

impl Session {
    pub fn builder() -> SessionBuilder {
        SessionBuilder::default()
    }

    pub(crate) async fn create(
        options: SessionOptions,
        paths: SessionPaths,
        event_store: Arc<dyn EventStore>,
        turn_runtime: Option<SessionTurnRuntime>,
        #[cfg(feature = "steering")] steering_policy: harness_contracts::SteeringPolicy,
    ) -> Result<Self, SessionError> {
        let projection = SessionProjection::empty(options.tenant_id, options.session_id);
        let (snapshot_tx, snapshot_rx) = watch::channel(projection.snapshot_id);
        let session = Self {
            options,
            paths,
            event_store,
            snapshot_tx,
            snapshot_rx,
            turn_runtime,
            #[cfg(feature = "steering")]
            steering: SteeringQueue::new(steering_policy),
            state: Mutex::new(SessionState {
                ended: false,
                projection,
            }),
        };
        session.append_created().await?;
        Ok(session)
    }

    pub(crate) async fn from_projection(
        options: SessionOptions,
        paths: SessionPaths,
        event_store: Arc<dyn EventStore>,
        turn_runtime: Option<SessionTurnRuntime>,
        projection: SessionProjection,
    ) -> Result<Self, SessionError> {
        let (snapshot_tx, snapshot_rx) = watch::channel(projection.snapshot_id);
        Ok(Self {
            options,
            paths,
            event_store,
            snapshot_tx,
            snapshot_rx,
            turn_runtime,
            #[cfg(feature = "steering")]
            steering: SteeringQueue::default(),
            state: Mutex::new(SessionState {
                ended: projection.end_reason.is_some(),
                projection,
            }),
        })
    }

    pub fn paths(&self) -> &SessionPaths {
        &self.paths
    }

    pub(crate) fn options(&self) -> &SessionOptions {
        &self.options
    }

    pub(crate) fn event_store(&self) -> &Arc<dyn EventStore> {
        &self.event_store
    }

    pub(crate) fn turn_runtime(&self) -> Option<SessionTurnRuntime> {
        self.turn_runtime.clone()
    }

    pub(crate) fn tenant_id(&self) -> TenantId {
        self.options.tenant_id
    }

    pub(crate) fn session_id(&self) -> SessionId {
        self.options.session_id
    }

    #[cfg(feature = "steering")]
    pub(crate) fn steering(&self) -> &SteeringQueue {
        &self.steering
    }

    pub async fn run_turn(&self, prompt: impl Into<String>) -> Result<(), SessionError> {
        if self.state.lock().await.ended {
            return Err(SessionError::Message("session already ended".to_owned()));
        }
        let runtime = self
            .turn_runtime()
            .ok_or_else(|| SessionError::Message("turn runtime missing".to_owned()))?;
        crate::turn::run_turn(self, runtime, prompt.into()).await
    }

    pub async fn interrupt(&self) -> Result<(), SessionError> {
        if self.state.lock().await.ended {
            return Err(SessionError::Message("session already ended".to_owned()));
        }
        Ok(())
    }

    pub async fn end(&self, reason: EndReason) -> Result<(), SessionError> {
        let snapshot_id;
        {
            let mut state = self.state.lock().await;
            if state.ended {
                return Ok(());
            }
            state.ended = true;
            state.projection.end_reason = Some(reason.clone());
            state.projection.refresh_snapshot_id();
            snapshot_id = state.projection.snapshot_id;
        }
        self.snapshot_tx.send_replace(snapshot_id);
        #[cfg(feature = "steering")]
        self.drop_steering_for_session_end().await?;

        self.event_store
            .append(
                self.options.tenant_id,
                self.options.session_id,
                &[Event::SessionEnded(SessionEndedEvent {
                    session_id: self.options.session_id,
                    tenant_id: self.options.tenant_id,
                    reason,
                    final_usage: UsageSnapshot::default(),
                    at: harness_contracts::now(),
                })],
            )
            .await
            .map_err(session_error)?;
        Ok(())
    }

    pub async fn projection(&self) -> SessionProjection {
        self.state.lock().await.projection.clone()
    }

    pub fn snapshot_id(&self) -> SnapshotId {
        *self.snapshot_rx.borrow()
    }

    async fn append_created(&self) -> Result<(), SessionError> {
        let snapshot_id = self.state.lock().await.projection.snapshot_id;
        self.event_store
            .append(
                self.options.tenant_id,
                self.options.session_id,
                &[Event::SessionCreated(SessionCreatedEvent {
                    session_id: self.options.session_id,
                    tenant_id: self.options.tenant_id,
                    options_hash: [0; 32],
                    snapshot_id,
                    effective_config_hash: ConfigHash([0; 32]),
                    created_at: harness_contracts::now(),
                })],
            )
            .await
            .map_err(session_error)?;
        Ok(())
    }

    pub(crate) async fn append_events(&self, events: &[Event]) -> Result<(), SessionError> {
        self.event_store
            .append(self.options.tenant_id, self.options.session_id, events)
            .await
            .map_err(session_error)?;
        Ok(())
    }

    pub(crate) async fn apply_projection_events(&self, events: &[Event]) {
        let snapshot_id = {
            let mut state = self.state.lock().await;
            state.projection.apply_events(events);
            state.projection.snapshot_id
        };
        self.snapshot_tx.send_replace(snapshot_id);
    }
}

pub(crate) fn session_error(error: impl std::fmt::Display) -> SessionError {
    SessionError::Message(error.to_string())
}
