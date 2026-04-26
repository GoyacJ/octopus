use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use harness_contracts::{
    ConfigHash, EndReason, Event, SessionCreatedEvent, SessionEndedEvent, SessionError, SessionId,
    SnapshotId, TenantId, UsageSnapshot,
};
use harness_journal::EventStore;
use tokio::sync::Mutex;

use crate::{SessionBuilder, SessionPaths};

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

#[derive(Debug, Clone, PartialEq)]
pub struct SessionProjection {
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub end_reason: Option<EndReason>,
    pub snapshot_id: SnapshotId,
}

pub struct Session {
    options: SessionOptions,
    paths: SessionPaths,
    event_store: Arc<dyn EventStore>,
    snapshot_id: SnapshotId,
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
    ) -> Result<Self, SessionError> {
        let snapshot_id = SnapshotId::new();
        let projection = SessionProjection {
            session_id: options.session_id,
            tenant_id: options.tenant_id,
            end_reason: None,
            snapshot_id,
        };
        let session = Self {
            options,
            paths,
            event_store,
            snapshot_id,
            state: Mutex::new(SessionState {
                ended: false,
                projection,
            }),
        };
        session.append_created().await?;
        Ok(session)
    }

    pub fn paths(&self) -> &SessionPaths {
        &self.paths
    }

    pub async fn run_turn(&self, _prompt: impl Into<String>) -> Result<(), SessionError> {
        if self.state.lock().await.ended {
            return Err(SessionError::Message("session already ended".to_owned()));
        }
        Ok(())
    }

    pub async fn interrupt(&self) -> Result<(), SessionError> {
        if self.state.lock().await.ended {
            return Err(SessionError::Message("session already ended".to_owned()));
        }
        Ok(())
    }

    pub async fn end(&self, reason: EndReason) -> Result<(), SessionError> {
        let mut state = self.state.lock().await;
        if state.ended {
            return Ok(());
        }
        state.ended = true;
        state.projection.end_reason = Some(reason.clone());
        drop(state);

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
        self.snapshot_id
    }

    async fn append_created(&self) -> Result<(), SessionError> {
        self.event_store
            .append(
                self.options.tenant_id,
                self.options.session_id,
                &[Event::SessionCreated(SessionCreatedEvent {
                    session_id: self.options.session_id,
                    tenant_id: self.options.tenant_id,
                    options_hash: [0; 32],
                    snapshot_id: self.state.lock().await.projection.snapshot_id,
                    effective_config_hash: ConfigHash([0; 32]),
                    created_at: harness_contracts::now(),
                })],
            )
            .await
            .map_err(session_error)?;
        Ok(())
    }
}

pub(crate) fn session_error(error: impl std::fmt::Display) -> SessionError {
    SessionError::Message(error.to_string())
}
