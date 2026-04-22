use std::path::PathBuf;
use std::pin::Pin;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use octopus_sdk_contracts::{
    EventId, PermissionMode, PluginsSnapshot, SessionEvent, SessionId, SubagentSpec,
};

use crate::{SessionError, SessionSnapshot};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventRange {
    pub after: Option<EventId>,
    pub limit: Option<usize>,
}

pub type EventStream = Pin<Box<dyn Stream<Item = Result<SessionEvent, SessionError>> + Send>>;
pub type EventRecordStream =
    Pin<Box<dyn Stream<Item = Result<SessionRecord, SessionError>> + Send>>;

#[derive(Debug, Clone, PartialEq)]
pub struct SessionRecord {
    pub event_id: EventId,
    pub event: SessionEvent,
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn append(&self, id: &SessionId, event: SessionEvent) -> Result<EventId, SessionError>;
    async fn append_session_started(
        &self,
        id: &SessionId,
        working_dir: PathBuf,
        permission_mode: PermissionMode,
        model: String,
        config_snapshot_id: String,
        effective_config_hash: String,
        token_budget: u32,
        plugins_snapshot: Option<PluginsSnapshot>,
    ) -> Result<EventId, SessionError>;
    async fn new_child_session(
        &self,
        parent_id: &SessionId,
        spec: &SubagentSpec,
    ) -> Result<SessionId, SessionError>;
    async fn stream(&self, id: &SessionId, range: EventRange) -> Result<EventStream, SessionError>;
    async fn stream_records(
        &self,
        id: &SessionId,
        range: EventRange,
    ) -> Result<EventRecordStream, SessionError> {
        let stream = self.stream(id, range).await?;
        Ok(Box::pin(stream.map(|item| {
            item.map(|event| SessionRecord {
                event_id: EventId::new_v4(),
                event,
            })
        })))
    }
    async fn snapshot(&self, id: &SessionId) -> Result<SessionSnapshot, SessionError>;
    async fn fork(&self, id: &SessionId, from: EventId) -> Result<SessionId, SessionError>;
    async fn wake(&self, id: &SessionId) -> Result<SessionSnapshot, SessionError>;
}

#[cfg(test)]
mod trait_object {
    use std::{path::PathBuf, pin::Pin, sync::Arc};

    use futures::Stream;
    use octopus_sdk_contracts::{
        EventId, PermissionMode, PluginsSnapshot, SessionEvent, SessionId, SubagentSpec,
    };

    use super::{EventRange, EventRecordStream, EventStream, SessionSnapshot, SessionStore};
    use crate::SessionError;

    #[test]
    fn trait_object() {
        struct DummyStore;

        #[async_trait::async_trait]
        impl SessionStore for DummyStore {
            async fn append(
                &self,
                _id: &SessionId,
                _event: SessionEvent,
            ) -> Result<EventId, SessionError> {
                Err(SessionError::NotFound)
            }

            async fn append_session_started(
                &self,
                _id: &SessionId,
                _working_dir: PathBuf,
                _permission_mode: PermissionMode,
                _model: String,
                _config_snapshot_id: String,
                _effective_config_hash: String,
                _token_budget: u32,
                _plugins_snapshot: Option<PluginsSnapshot>,
            ) -> Result<EventId, SessionError> {
                Err(SessionError::NotFound)
            }

            async fn new_child_session(
                &self,
                _parent_id: &SessionId,
                _spec: &SubagentSpec,
            ) -> Result<SessionId, SessionError> {
                Err(SessionError::NotFound)
            }

            async fn stream(
                &self,
                _id: &SessionId,
                _range: EventRange,
            ) -> Result<EventStream, SessionError> {
                Err(SessionError::NotFound)
            }

            async fn stream_records(
                &self,
                _id: &SessionId,
                _range: EventRange,
            ) -> Result<EventRecordStream, SessionError> {
                Err(SessionError::NotFound)
            }

            async fn snapshot(&self, _id: &SessionId) -> Result<SessionSnapshot, SessionError> {
                Err(SessionError::NotFound)
            }

            async fn fork(
                &self,
                _id: &SessionId,
                _from: EventId,
            ) -> Result<SessionId, SessionError> {
                Err(SessionError::NotFound)
            }

            async fn wake(&self, _id: &SessionId) -> Result<SessionSnapshot, SessionError> {
                Err(SessionError::NotFound)
            }
        }

        let _: Arc<dyn SessionStore> = Arc::new(DummyStore);
        let _: Pin<Box<dyn Stream<Item = Result<SessionEvent, SessionError>> + Send>> =
            Box::pin(futures::stream::empty());
    }
}

#[cfg(test)]
mod contract_fields {
    use std::path::PathBuf;

    use octopus_sdk_contracts::{EventId, PermissionMode, PluginsSnapshot, SessionId, Usage};

    use super::EventRange;
    use crate::SessionSnapshot;

    #[test]
    fn event_range_and_snapshot_expose_expected_fields() {
        let range = EventRange {
            after: Some(EventId("event-1".into())),
            limit: Some(25),
        };
        let snapshot = SessionSnapshot {
            id: SessionId("session-1".into()),
            working_dir: PathBuf::from("/tmp/octopus"),
            permission_mode: PermissionMode::Default,
            model: "main".into(),
            config_snapshot_id: "cfg-1".into(),
            effective_config_hash: "hash-1".into(),
            token_budget: 8_192,
            plugins_snapshot: PluginsSnapshot::default(),
            head_event_id: EventId("event-9".into()),
            usage: Usage {
                input_tokens: 3,
                output_tokens: 5,
                cache_creation_input_tokens: 7,
                cache_read_input_tokens: 11,
            },
        };

        assert_eq!(range.limit, Some(25));
        assert_eq!(snapshot.id.0, "session-1");
        assert_eq!(snapshot.working_dir, PathBuf::from("/tmp/octopus"));
        assert_eq!(snapshot.permission_mode, PermissionMode::Default);
        assert_eq!(snapshot.model, "main");
        assert_eq!(snapshot.token_budget, 8_192);
        assert!(snapshot.plugins_snapshot.plugins.is_empty());
        assert_eq!(snapshot.usage.output_tokens, 5);
    }
}
