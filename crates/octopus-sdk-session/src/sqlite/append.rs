use octopus_sdk_contracts::{AssistantEvent, ContentBlock, EventId, SessionEvent, SessionId, Usage};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::SessionError;

use super::{event_kind, now_millis, SqliteJsonlSessionStore};

impl SqliteJsonlSessionStore {
    pub(crate) fn append_event(
        &self,
        session_id: &SessionId,
        event: SessionEvent,
    ) -> Result<EventId, SessionError> {
        let event_id = EventId::new_v4();
        let mut connection = self.open_connection()?;

        if !session_exists(&connection, session_id)? && !matches!(event, SessionEvent::SessionStarted { .. }) {
            return Err(SessionError::Corrupted {
                reason: "first_event_must_be_session_started".into(),
            });
        }

        crate::jsonl::append_record(&self.jsonl_root, session_id, &event_id, &event)?;

        let transaction = connection.transaction()?;

        self.ensure_session_row(&transaction, session_id, &event, &event_id)?;

        let seq = next_sequence(&transaction, session_id)?;
        let payload = serde_json::to_string(&event)?;
        let now = now_millis();

        transaction.execute(
            "
            INSERT INTO events (event_id, session_id, seq, kind, payload, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            params![event_id.0, session_id.0, seq, event_kind(&event), payload, now],
        )?;

        self.update_session_projection(&transaction, session_id, &event, &event_id, now)?;
        transaction.commit()?;

        Ok(event_id)
    }

    fn ensure_session_row(
        &self,
        transaction: &Transaction<'_>,
        session_id: &SessionId,
        event: &SessionEvent,
        event_id: &EventId,
    ) -> Result<(), SessionError> {
        let exists = transaction
            .query_row(
                "SELECT 1 FROM sessions WHERE session_id = ?1",
                [session_id.0.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .is_some();

        if exists {
            return Ok(());
        }

        let SessionEvent::SessionStarted {
            config_snapshot_id,
            effective_config_hash,
        } = event
        else {
            return Err(SessionError::Corrupted {
                reason: "first_event_must_be_session_started".into(),
            });
        };

        let usage_json = serde_json::to_string(&Usage::default())?;
        let now = now_millis();

        transaction.execute(
            "
            INSERT INTO sessions (
                session_id,
                config_snapshot_id,
                effective_config_hash,
                head_event_id,
                usage_json,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ",
            params![
                session_id.0,
                config_snapshot_id,
                effective_config_hash,
                event_id.0,
                usage_json,
                now,
                now
            ],
        )?;

        Ok(())
    }

    fn update_session_projection(
        &self,
        transaction: &Transaction<'_>,
        session_id: &SessionId,
        event: &SessionEvent,
        event_id: &EventId,
        now: i64,
    ) -> Result<(), SessionError> {
        let usage_json = serde_json::to_string(&next_usage(transaction, session_id, event)?)?;

        match event {
            SessionEvent::SessionStarted {
                config_snapshot_id,
                effective_config_hash,
            } => {
                transaction.execute(
                    "
                    UPDATE sessions
                    SET config_snapshot_id = ?2,
                        effective_config_hash = ?3,
                        head_event_id = ?4,
                        usage_json = ?5,
                        updated_at = ?6
                    WHERE session_id = ?1
                    ",
                    params![
                        session_id.0,
                        config_snapshot_id,
                        effective_config_hash,
                        event_id.0,
                        usage_json,
                        now
                    ],
                )?;
            }
            _ => {
                transaction.execute(
                    "
                    UPDATE sessions
                    SET head_event_id = ?2,
                        usage_json = ?3,
                        updated_at = ?4
                    WHERE session_id = ?1
                    ",
                    params![session_id.0, event_id.0, usage_json, now],
                )?;
            }
        }

        Ok(())
    }
}

fn next_sequence(transaction: &Transaction<'_>, session_id: &SessionId) -> Result<i64, SessionError> {
    transaction
        .query_row(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM events WHERE session_id = ?1",
            [session_id.0.as_str()],
            |row| row.get(0),
        )
        .map_err(SessionError::from)
}

fn session_exists(
    connection: &rusqlite::Connection,
    session_id: &SessionId,
) -> Result<bool, SessionError> {
    Ok(connection
        .query_row(
            "SELECT 1 FROM sessions WHERE session_id = ?1",
            [session_id.0.as_str()],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some())
}

fn next_usage(
    transaction: &Transaction<'_>,
    session_id: &SessionId,
    event: &SessionEvent,
) -> Result<Usage, SessionError> {
    if matches!(event, SessionEvent::SessionStarted { .. }) {
        return Ok(Usage::default());
    }

    let usage_json: String = transaction.query_row(
        "SELECT usage_json FROM sessions WHERE session_id = ?1",
        [session_id.0.as_str()],
        |row| row.get(0),
    )?;
    let current = serde_json::from_str::<Usage>(&usage_json)?;
    let delta = usage_delta(event)?;

    Ok(&current + &delta)
}

pub(super) fn project_usage<'a>(
    events: impl IntoIterator<Item = &'a SessionEvent>,
) -> Result<Usage, SessionError> {
    let mut total = Usage::default();

    for event in events {
        total = &total + &usage_delta(event)?;
    }

    Ok(total)
}

fn usage_delta(event: &SessionEvent) -> Result<Usage, SessionError> {
    match event {
        SessionEvent::AssistantMessage(message) => project_message_usage(message.content.iter()),
        _ => Ok(Usage::default()),
    }
}

fn project_message_usage<'a>(
    blocks: impl IntoIterator<Item = &'a ContentBlock>,
) -> Result<Usage, SessionError> {
    let mut total = Usage::default();

    for block in blocks {
        total = &total + &project_block_usage(block)?;
    }

    Ok(total)
}

fn project_block_usage(block: &ContentBlock) -> Result<Usage, SessionError> {
    match block {
        ContentBlock::Text { text } | ContentBlock::Thinking { text } => {
            Ok(parse_usage_event(text).unwrap_or_default())
        }
        ContentBlock::ToolResult { content, .. } => project_message_usage(content.iter()),
        ContentBlock::ToolUse { .. } => Ok(Usage::default()),
    }
}

fn parse_usage_event(text: &str) -> Option<Usage> {
    match serde_json::from_str::<AssistantEvent>(text).ok()? {
        AssistantEvent::Usage(usage) => Some(usage),
        _ => None,
    }
}
