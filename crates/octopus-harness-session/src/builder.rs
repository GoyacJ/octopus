use std::sync::Arc;

use harness_contracts::SessionError;
use harness_journal::EventStore;

use crate::{Session, SessionOptions, SessionPaths};

#[derive(Default)]
pub struct SessionBuilder {
    options: Option<SessionOptions>,
    event_store: Option<Arc<dyn EventStore>>,
}

impl SessionBuilder {
    #[must_use]
    pub fn with_options(mut self, options: SessionOptions) -> Self {
        self.options = Some(options);
        self
    }

    #[must_use]
    pub fn with_event_store(mut self, event_store: Arc<dyn EventStore>) -> Self {
        self.event_store = Some(event_store);
        self
    }

    pub async fn build(self) -> Result<Session, SessionError> {
        let mut options = self
            .options
            .ok_or_else(|| SessionError::Message("session options missing".to_owned()))?;
        let event_store = self
            .event_store
            .ok_or_else(|| SessionError::Message("event store missing".to_owned()))?;

        options.workspace_root = options
            .workspace_root
            .canonicalize()
            .map_err(|error| SessionError::Message(format!("workspace_root invalid: {error}")))?;
        let paths = SessionPaths::from_workspace(
            &options.workspace_root,
            options.tenant_id,
            options.session_id,
        );

        Session::create(options, paths, event_store).await
    }
}
