use std::sync::Arc;

use futures::stream;
use harness_contracts::{
    EndReason, Event, EventId, ForkReason, JournalError, JournalOffset, SessionError, SessionId,
    TenantId, ToolSearchMode,
};
use harness_journal::{
    EventEnvelope, EventStore, PrunePolicy, PruneReport, ReplayCursor, SchemaVersion,
    SessionFilter, SessionSnapshot, SessionSummary,
};
use harness_session::{Session, SessionOptions};
use tokio::sync::Mutex;

#[tokio::test]
async fn builder_rejects_missing_or_invalid_workspace_root() {
    let store = Arc::new(RecordingEventStore::default());
    let missing = std::env::temp_dir().join(format!("octopus-missing-{}", SessionId::new()));

    let error = Session::builder()
        .with_options(SessionOptions::new(missing))
        .with_event_store(store)
        .build()
        .await
        .unwrap_err();

    assert!(matches!(error, SessionError::Message(message) if message.contains("workspace_root")));
}

#[tokio::test]
async fn create_and_end_write_lifecycle_events() {
    let root = tempfile::tempdir().unwrap();
    let store = Arc::new(RecordingEventStore::default());

    let session = Session::builder()
        .with_options(SessionOptions::new(root.path()))
        .with_event_store(store.clone())
        .build()
        .await
        .unwrap();
    session.end(EndReason::Completed).await.unwrap();

    let events = store.events().await;
    assert!(matches!(events[0], Event::SessionCreated(_)));
    assert!(matches!(events[1], Event::SessionEnded(_)));
    assert_eq!(
        session.projection().await.end_reason,
        Some(EndReason::Completed)
    );
    assert_eq!(
        session.snapshot_id(),
        session.projection().await.snapshot_id
    );
}

#[tokio::test]
async fn ended_session_rejects_run_turn() {
    let root = tempfile::tempdir().unwrap();
    let session = Session::builder()
        .with_options(SessionOptions::new(root.path()))
        .with_event_store(Arc::new(RecordingEventStore::default()))
        .build()
        .await
        .unwrap();
    session.end(EndReason::Completed).await.unwrap();

    let error = session.run_turn("hello").await.unwrap_err();

    assert!(matches!(error, SessionError::Message(message) if message.contains("ended")));
}

#[tokio::test]
async fn session_options_exposes_creation_time_tool_search_mode() {
    let default_options = SessionOptions::new(tempfile::tempdir().unwrap().path());
    assert_eq!(default_options.tool_search, ToolSearchMode::default());

    let root = tempfile::tempdir().unwrap();
    let options = SessionOptions::new(root.path()).with_tool_search_mode(ToolSearchMode::Always);
    assert_eq!(options.tool_search, ToolSearchMode::Always);

    Session::builder()
        .with_options(options)
        .with_event_store(Arc::new(RecordingEventStore::default()))
        .build()
        .await
        .unwrap();
}

#[derive(Default)]
struct RecordingEventStore {
    events: Mutex<Vec<Event>>,
}

impl RecordingEventStore {
    async fn events(&self) -> Vec<Event> {
        self.events.lock().await.clone()
    }
}

#[async_trait::async_trait]
impl EventStore for RecordingEventStore {
    async fn append(
        &self,
        _tenant: TenantId,
        _session_id: SessionId,
        events: &[Event],
    ) -> Result<JournalOffset, JournalError> {
        let mut guard = self.events.lock().await;
        guard.extend_from_slice(events);
        Ok(JournalOffset(guard.len().saturating_sub(1) as u64))
    }

    async fn read_envelopes(
        &self,
        tenant: TenantId,
        session_id: SessionId,
        _cursor: ReplayCursor,
    ) -> Result<futures::stream::BoxStream<'static, EventEnvelope>, JournalError> {
        let envelopes = self
            .events
            .lock()
            .await
            .clone()
            .into_iter()
            .enumerate()
            .map(move |(offset, payload)| EventEnvelope {
                offset: JournalOffset(offset as u64),
                event_id: EventId::new(),
                session_id,
                tenant_id: tenant,
                run_id: None,
                correlation_id: harness_contracts::CorrelationId::new(),
                causation_id: None,
                schema_version: SchemaVersion::CURRENT,
                recorded_at: harness_contracts::now(),
                payload,
            })
            .collect::<Vec<_>>();
        Ok(Box::pin(stream::iter(envelopes)))
    }

    async fn query_after(
        &self,
        _tenant: TenantId,
        _after: Option<EventId>,
        _limit: usize,
    ) -> Result<Vec<EventEnvelope>, JournalError> {
        Ok(Vec::new())
    }

    async fn snapshot(
        &self,
        _tenant: TenantId,
        _session_id: SessionId,
    ) -> Result<Option<SessionSnapshot>, JournalError> {
        Ok(None)
    }

    async fn save_snapshot(
        &self,
        _tenant: TenantId,
        _snapshot: SessionSnapshot,
    ) -> Result<(), JournalError> {
        Ok(())
    }

    async fn compact_link(
        &self,
        _parent: SessionId,
        _child: SessionId,
        _reason: ForkReason,
    ) -> Result<(), JournalError> {
        Ok(())
    }

    async fn list_sessions(
        &self,
        _tenant: TenantId,
        _filter: SessionFilter,
    ) -> Result<Vec<SessionSummary>, JournalError> {
        Ok(Vec::new())
    }

    async fn prune(
        &self,
        _tenant: TenantId,
        _policy: PrunePolicy,
    ) -> Result<PruneReport, JournalError> {
        Ok(PruneReport {
            events_removed: 0,
            snapshots_removed: 0,
            bytes_freed: 0,
        })
    }
}
