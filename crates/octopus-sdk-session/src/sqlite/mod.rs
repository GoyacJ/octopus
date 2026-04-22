mod append;
mod schema;
mod stream;

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use octopus_sdk_contracts::{
    EventId, PermissionMode, PluginsSnapshot, SessionEvent, SessionId, SubagentSpec,
};
use rusqlite::Connection;

use crate::{
    EventRange, EventRecordStream, EventStream, SessionError, SessionSnapshot, SessionStore,
};

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
    ) -> Result<EventId, SessionError> {
        self.append_event(
            id,
            SessionEvent::SessionStarted {
                working_dir: working_dir.to_string_lossy().into_owned(),
                permission_mode,
                model,
                config_snapshot_id,
                effective_config_hash,
                token_budget,
                plugins_snapshot,
            },
        )
    }

    async fn new_child_session(
        &self,
        parent_id: &SessionId,
        spec: &SubagentSpec,
    ) -> Result<SessionId, SessionError> {
        let _ = spec;
        let parent = self.load_snapshot(parent_id)?;
        let child_id = SessionId::new_v4();

        self.append_event(
            &child_id,
            SessionEvent::SessionStarted {
                working_dir: parent.working_dir.to_string_lossy().into_owned(),
                permission_mode: parent.permission_mode,
                model: parent.model,
                config_snapshot_id: parent.config_snapshot_id,
                effective_config_hash: parent.effective_config_hash,
                token_budget: parent.token_budget,
                plugins_snapshot: Some(parent.plugins_snapshot),
            },
        )?;

        Ok(child_id)
    }

    async fn stream(&self, id: &SessionId, range: EventRange) -> Result<EventStream, SessionError> {
        self.stream_events(id, range)
    }

    async fn stream_records(
        &self,
        id: &SessionId,
        range: EventRange,
    ) -> Result<EventRecordStream, SessionError> {
        self.stream_record_events(id, range)
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
        SessionEvent::SessionPluginsSnapshot { .. } => "session_plugins_snapshot",
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

pub(crate) fn serialize_permission_mode(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::Default => "default",
        PermissionMode::AcceptEdits => "accept_edits",
        PermissionMode::BypassPermissions => "bypass_permissions",
        PermissionMode::Plan => "plan",
    }
}

pub(crate) fn deserialize_permission_mode(value: &str) -> Result<PermissionMode, SessionError> {
    match value {
        "default" => Ok(PermissionMode::Default),
        "accept_edits" => Ok(PermissionMode::AcceptEdits),
        "bypass_permissions" => Ok(PermissionMode::BypassPermissions),
        "plan" => Ok(PermissionMode::Plan),
        _ => Err(SessionError::Corrupted {
            reason: format!("unknown_permission_mode:{value}"),
        }),
    }
}
