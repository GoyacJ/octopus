use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use octopus_sdk_contracts::{EventId, SessionEvent, SessionId};

use crate::{SessionError, SessionSnapshot};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventRange {
    pub after: Option<EventId>,
    pub limit: Option<usize>,
}

pub type EventStream = Pin<Box<dyn Stream<Item = Result<SessionEvent, SessionError>> + Send>>;

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn append(&self, id: &SessionId, event: SessionEvent) -> Result<EventId, SessionError>;
    async fn stream(&self, id: &SessionId, range: EventRange) -> Result<EventStream, SessionError>;
    async fn snapshot(&self, id: &SessionId) -> Result<SessionSnapshot, SessionError>;
    async fn fork(&self, id: &SessionId, from: EventId) -> Result<SessionId, SessionError>;
    async fn wake(&self, id: &SessionId) -> Result<SessionSnapshot, SessionError>;
}

#[cfg(test)]
mod trait_object {
    use std::{pin::Pin, sync::Arc};

    use futures::Stream;
    use octopus_sdk_contracts::{EventId, SessionEvent, SessionId};

    use super::{EventRange, EventStream, SessionSnapshot, SessionStore};
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

            async fn stream(
                &self,
                _id: &SessionId,
                _range: EventRange,
            ) -> Result<EventStream, SessionError> {
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
    use octopus_sdk_contracts::{EventId, SessionId, Usage};

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
            config_snapshot_id: "cfg-1".into(),
            effective_config_hash: "hash-1".into(),
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
        assert_eq!(snapshot.usage.output_tokens, 5);
    }
}
