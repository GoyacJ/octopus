mod append;
mod schema;
mod stream;

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use octopus_sdk_contracts::{EventId, SessionEvent, SessionId};
use rusqlite::Connection;

use crate::{EventRange, EventStream, SessionError, SessionSnapshot, SessionStore};

#[derive(Debug, Clone)]
pub struct SqliteJsonlSessionStore {
    pub(crate) db_path: PathBuf,
    pub(crate) jsonl_root: PathBuf,
}

impl SqliteJsonlSessionStore {
    pub fn open(db: &Path, jsonl_root: &Path) -> Result<Self, SessionError> {
        if let Some(parent) = db.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(jsonl_root)?;

        let connection = Self::open_connection_at(db)?;
        schema::initialize(&connection)?;

        let store = Self {
            db_path: db.to_path_buf(),
            jsonl_root: jsonl_root.to_path_buf(),
        };
        store.reconcile_jsonl_projection()?;

        Ok(store)
    }

    pub(crate) fn open_connection(&self) -> Result<Connection, SessionError> {
        Self::open_connection_at(&self.db_path)
    }

    fn open_connection_at(db: &Path) -> Result<Connection, SessionError> {
        let connection = Connection::open(db)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(connection)
    }
}

#[async_trait]
impl SessionStore for SqliteJsonlSessionStore {
    async fn append(&self, id: &SessionId, event: SessionEvent) -> Result<EventId, SessionError> {
        self.append_event(id, event)
    }

    async fn stream(&self, id: &SessionId, range: EventRange) -> Result<EventStream, SessionError> {
        self.stream_events(id, range)
    }

    async fn snapshot(&self, id: &SessionId) -> Result<SessionSnapshot, SessionError> {
        self.load_snapshot(id)
    }

    async fn fork(&self, id: &SessionId, from: EventId) -> Result<SessionId, SessionError> {
        self.fork_session(id, &from)
    }

    async fn wake(&self, id: &SessionId) -> Result<SessionSnapshot, SessionError> {
        self.validate_wake(id)?;
        self.load_snapshot(id)
    }
}

pub(crate) fn event_kind(event: &SessionEvent) -> &'static str {
    match event {
        SessionEvent::SessionStarted { .. } => "session_started",
        SessionEvent::UserMessage(_) => "user_message",
        SessionEvent::AssistantMessage(_) => "assistant_message",
        SessionEvent::ToolExecuted { .. } => "tool_executed",
        SessionEvent::Render { .. } => "render",
        SessionEvent::Ask { .. } => "ask",
        SessionEvent::Checkpoint { .. } => "checkpoint",
        SessionEvent::SessionEnded { .. } => "session_ended",
    }
}

pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_millis() as i64
}
