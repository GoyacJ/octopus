use std::path::PathBuf;

use futures::{stream, StreamExt};
use octopus_sdk_contracts::{EventId, PluginsSnapshot, SessionEvent, SessionId, Usage};
use rusqlite::{params, OptionalExtension};

use crate::{
    jsonl::JsonlRecord, EventRange, EventRecordStream, EventStream, SessionError, SessionRecord,
    SessionSnapshot,
};

use super::{
    append::project_usage, deserialize_permission_mode, event_kind, now_millis,
    serialize_permission_mode, SqliteJsonlSessionStore,
};

impl SqliteJsonlSessionStore {
    pub(crate) fn stream_events(
        &self,
        session_id: &SessionId,
        range: EventRange,
    ) -> Result<EventStream, SessionError> {
        let records = self.stream_record_events(session_id, range)?;
        Ok(Box::pin(records.map(|record| record.map(|record| record.event))))
    }

    pub(crate) fn stream_record_events(
        &self,
        session_id: &SessionId,
        range: EventRange,
    ) -> Result<EventRecordStream, SessionError> {
        let connection = self.open_connection()?;

        if !session_exists(&connection, session_id)? {
            return Err(SessionError::NotFound);
        }

        let after_seq = match range.after {
            Some(after) => {
                event_sequence(&connection, session_id, &after)?.ok_or(SessionError::NotFound)?
            }
            None => 0,
        };

        let records = if let Some(limit) = range.limit {
            let limit = i64::try_from(limit).map_err(|_| SessionError::Corrupted {
                reason: "range_limit_overflows_i64".into(),
            })?;
            load_event_records(&connection, session_id, after_seq, Some(limit))?
        } else {
            load_event_records(&connection, session_id, after_seq, None)?
        };

        Ok(Box::pin(stream::iter(
            records
                .into_iter()
                .map(Result::<SessionRecord, SessionError>::Ok),
        )))
    }

    pub(crate) fn load_snapshot(
        &self,
        session_id: &SessionId,
    ) -> Result<SessionSnapshot, SessionError> {
        let connection = self.open_connection()?;
        let row = load_session_row(&connection, session_id)?.ok_or(SessionError::NotFound)?;

        Ok(SessionSnapshot {
            id: session_id.clone(),
            working_dir: PathBuf::from(row.working_dir),
            permission_mode: deserialize_permission_mode(&row.permission_mode)?,
            model: row.model,
            config_snapshot_id: row.config_snapshot_id,
            effective_config_hash: row.effective_config_hash,
            token_budget: row.token_budget,
            plugins_snapshot: serde_json::from_str::<PluginsSnapshot>(&row.plugins_snapshot_json)?,
            head_event_id: EventId(row.head_event_id),
            usage: serde_json::from_str::<Usage>(&row.usage_json)?,
        })
    }

    pub(crate) fn validate_wake(&self, session_id: &SessionId) -> Result<(), SessionError> {
        let connection = self.open_connection()?;
        let checkpoints = load_checkpoint_rows(&connection, session_id)?;
        let Some((checkpoint_seq, anchor_event_id)) = checkpoints.last() else {
            return Ok(());
        };
        let anchor_seq = event_sequence(&connection, session_id, anchor_event_id)?.ok_or(
            SessionError::Corrupted {
                reason: "checkpoint_anchor_event_not_found".into(),
            },
        )?;

        if anchor_seq >= *checkpoint_seq {
            return Err(SessionError::Corrupted {
                reason: "checkpoint_anchor_event_out_of_range".into(),
            });
        }

        let _ = load_events_after_seq(&connection, session_id, anchor_seq)?;
        Ok(())
    }

    pub(crate) fn fork_session(
        &self,
        session_id: &SessionId,
        from_event_id: &EventId,
    ) -> Result<SessionId, SessionError> {
        let mut connection = self.open_connection()?;
        let source = load_session_row(&connection, session_id)?.ok_or(SessionError::NotFound)?;
        let max_seq = event_sequence(&connection, session_id, from_event_id)?
            .ok_or(SessionError::NotFound)?;
        let source_events = load_events_through(&connection, session_id, max_seq)?;
        let forked_session_id = SessionId::new_v4();
        let mut cloned = Vec::with_capacity(source_events.len());

        for (seq, kind, event) in source_events {
            let event_id = EventId::new_v4();
            crate::jsonl::append_record(&self.jsonl_root, &forked_session_id, &event_id, &event)?;
            cloned.push((event_id, seq, kind, event));
        }

        let head_event_id = cloned
            .last()
            .map(|(event_id, _, _, _)| event_id.clone())
            .ok_or(SessionError::NotFound)?;
        let now = super::now_millis();
        let transaction = connection.transaction()?;

        transaction.execute(
            "
            INSERT INTO sessions (
                session_id,
                working_dir,
                permission_mode,
                model,
                config_snapshot_id,
                effective_config_hash,
                token_budget,
                plugins_snapshot_json,
                head_event_id,
                usage_json,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ",
            params![
                forked_session_id.0,
                source.working_dir,
                source.permission_mode,
                source.model,
                source.config_snapshot_id,
                source.effective_config_hash,
                source.token_budget,
                source.plugins_snapshot_json,
                head_event_id.0,
                source.usage_json,
                now,
                now
            ],
        )?;

        for (event_id, seq, kind, event) in cloned {
            let payload = serde_json::to_string(&event)?;
            transaction.execute(
                "
                INSERT INTO events (event_id, session_id, seq, kind, payload, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
                params![event_id.0, forked_session_id.0, seq, kind, payload, now],
            )?;
        }

        transaction.commit()?;

        Ok(forked_session_id)
    }

    pub(crate) fn reconcile_jsonl_projection(&self) -> Result<(), SessionError> {
        for session_id in crate::jsonl::list_session_ids(&self.jsonl_root)? {
            let records = crate::jsonl::read_records(&self.jsonl_root, &session_id)?;
            if records.is_empty() {
                continue;
            }

            self.reconcile_session_projection(&session_id, &records)?;
        }

        Ok(())
    }

    fn reconcile_session_projection(
        &self,
        session_id: &SessionId,
        records: &[JsonlRecord],
    ) -> Result<(), SessionError> {
        let expected = expected_projection(records)?;
        let connection = self.open_connection()?;

        if !projection_needs_repair(&connection, session_id, records, &expected)? {
            return Ok(());
        }

        let now = now_millis();
        let transaction = connection.unchecked_transaction()?;
        transaction.execute(
            "DELETE FROM events WHERE session_id = ?1",
            [session_id.0.as_str()],
        )?;
        transaction.execute(
            "DELETE FROM sessions WHERE session_id = ?1",
            [session_id.0.as_str()],
        )?;
        transaction.execute(
            "
            INSERT INTO sessions (
                session_id,
                working_dir,
                permission_mode,
                model,
                config_snapshot_id,
                effective_config_hash,
                token_budget,
                plugins_snapshot_json,
                head_event_id,
                usage_json,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ",
            params![
                session_id.0,
                expected.working_dir,
                serialize_permission_mode(expected.permission_mode),
                expected.model,
                expected.config_snapshot_id,
                expected.effective_config_hash,
                expected.token_budget,
                serde_json::to_string(&expected.plugins_snapshot)?,
                expected.head_event_id.0,
                serde_json::to_string(&expected.usage)?,
                now,
                now
            ],
        )?;

        for (index, record) in records.iter().enumerate() {
            let payload = serde_json::to_string(&record.event)?;
            transaction.execute(
                "
                INSERT INTO events (event_id, session_id, seq, kind, payload, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
                params![
                    record.event_id.0,
                    session_id.0,
                    i64::try_from(index + 1).map_err(|_| SessionError::Corrupted {
                        reason: "jsonl_event_count_overflows_i64".into(),
                    })?,
                    event_kind(&record.event),
                    payload,
                    now
                ],
            )?;
        }

        transaction.commit()?;
        Ok(())
    }
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

struct SessionRow {
    working_dir: String,
    permission_mode: String,
    model: String,
    config_snapshot_id: String,
    effective_config_hash: String,
    token_budget: u32,
    plugins_snapshot_json: String,
    head_event_id: String,
    usage_json: String,
}

fn load_session_row(
    connection: &rusqlite::Connection,
    session_id: &SessionId,
) -> Result<Option<SessionRow>, SessionError> {
    connection
        .query_row(
            "
            SELECT working_dir, permission_mode, model, config_snapshot_id, effective_config_hash, token_budget, plugins_snapshot_json, head_event_id, usage_json
            FROM sessions
            WHERE session_id = ?1
            ",
            [session_id.0.as_str()],
            |row| {
                Ok(SessionRow {
                    working_dir: row.get::<_, String>(0)?,
                    permission_mode: row.get::<_, String>(1)?,
                    model: row.get::<_, String>(2)?,
                    config_snapshot_id: row.get::<_, String>(3)?,
                    effective_config_hash: row.get::<_, String>(4)?,
                    token_budget: row.get::<_, u32>(5)?,
                    plugins_snapshot_json: row.get::<_, String>(6)?,
                    head_event_id: row.get::<_, String>(7)?,
                    usage_json: row.get::<_, String>(8)?,
                })
            },
        )
        .optional()
        .map_err(SessionError::from)
}

fn load_event_ids(
    connection: &rusqlite::Connection,
    session_id: &SessionId,
) -> Result<Vec<String>, SessionError> {
    let mut statement = connection.prepare(
        "
        SELECT event_id
        FROM events
        WHERE session_id = ?1
        ORDER BY seq ASC
        ",
    )?;
    let rows = statement.query_map([session_id.0.as_str()], |row| row.get::<_, String>(0))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(SessionError::from)
}

fn load_event_records(
    connection: &rusqlite::Connection,
    session_id: &SessionId,
    after_seq: i64,
    limit: Option<i64>,
) -> Result<Vec<SessionRecord>, SessionError> {
    if let Some(limit) = limit {
        let mut statement = connection.prepare(
            "
            SELECT event_id, payload
            FROM events
            WHERE session_id = ?1 AND seq > ?2
            ORDER BY seq ASC
            LIMIT ?3
            ",
        )?;
        let rows = statement.query_map(params![session_id.0, after_seq, limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        rows.map(|row| -> Result<SessionRecord, SessionError> {
            let (event_id, payload) = row?;
            Ok(SessionRecord {
                event_id: EventId(event_id),
                event: serde_json::from_str(&payload)?,
            })
        })
        .collect()
    } else {
        let mut statement = connection.prepare(
            "
            SELECT event_id, payload
            FROM events
            WHERE session_id = ?1 AND seq > ?2
            ORDER BY seq ASC
            ",
        )?;
        let rows = statement.query_map(params![session_id.0, after_seq], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        rows.map(|row| -> Result<SessionRecord, SessionError> {
            let (event_id, payload) = row?;
            Ok(SessionRecord {
                event_id: EventId(event_id),
                event: serde_json::from_str(&payload)?,
            })
        })
        .collect()
    }
}

fn event_sequence(
    connection: &rusqlite::Connection,
    session_id: &SessionId,
    event_id: &EventId,
) -> Result<Option<i64>, SessionError> {
    connection
        .query_row(
            "
            SELECT seq
            FROM events
            WHERE session_id = ?1 AND event_id = ?2
            ",
            params![session_id.0, event_id.0],
            |row| row.get(0),
        )
        .optional()
        .map_err(SessionError::from)
}

fn load_events_through(
    connection: &rusqlite::Connection,
    session_id: &SessionId,
    max_seq: i64,
) -> Result<Vec<(i64, String, SessionEvent)>, SessionError> {
    let mut statement = connection.prepare(
        "
        SELECT seq, kind, payload
        FROM events
        WHERE session_id = ?1 AND seq <= ?2
        ORDER BY seq ASC
        ",
    )?;
    let rows = statement.query_map(params![session_id.0, max_seq], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    rows.map(|row| -> Result<(i64, String, SessionEvent), SessionError> {
        let (seq, kind, payload) = row?;
        Ok((seq, kind, serde_json::from_str(&payload)?))
    })
    .collect()
}

fn load_events_after_seq(
    connection: &rusqlite::Connection,
    session_id: &SessionId,
    after_seq: i64,
) -> Result<Vec<SessionEvent>, SessionError> {
    let mut statement = connection.prepare(
        "
        SELECT payload
        FROM events
        WHERE session_id = ?1 AND seq > ?2
        ORDER BY seq ASC
        ",
    )?;
    let rows = statement.query_map(params![session_id.0, after_seq], |row| {
        row.get::<_, String>(0)
    })?;

    rows.map(|row| -> Result<SessionEvent, SessionError> { Ok(serde_json::from_str(&row?)?) })
        .collect()
}

fn load_checkpoint_rows(
    connection: &rusqlite::Connection,
    session_id: &SessionId,
) -> Result<Vec<(i64, EventId)>, SessionError> {
    let mut statement = connection.prepare(
        "
        SELECT seq, payload
        FROM events
        WHERE session_id = ?1
        ORDER BY seq ASC
        ",
    )?;
    let rows = statement.query_map([session_id.0.as_str()], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;

    rows.map(|row| -> Result<Option<(i64, EventId)>, SessionError> {
        let (seq, payload) = row?;
        let event = serde_json::from_str::<SessionEvent>(&payload)?;
        Ok(match event {
            SessionEvent::Checkpoint {
                anchor_event_id, ..
            } => Some((seq, anchor_event_id)),
            _ => None,
        })
    })
    .filter_map(|row| match row {
        Ok(Some(value)) => Some(Ok(value)),
        Ok(None) => None,
        Err(error) => Some(Err(error)),
    })
    .collect()
}

struct ExpectedProjection {
    working_dir: String,
    permission_mode: octopus_sdk_contracts::PermissionMode,
    model: String,
    config_snapshot_id: String,
    effective_config_hash: String,
    token_budget: u32,
    plugins_snapshot: PluginsSnapshot,
    head_event_id: EventId,
    usage: Usage,
}

fn expected_projection(records: &[JsonlRecord]) -> Result<ExpectedProjection, SessionError> {
    let first = records.first().ok_or(SessionError::Corrupted {
        reason: "jsonl_session_has_no_events".into(),
    })?;
    let last = records.last().ok_or(SessionError::Corrupted {
        reason: "jsonl_session_has_no_events".into(),
    })?;

    let SessionEvent::SessionStarted {
        working_dir,
        permission_mode,
        model,
        config_snapshot_id,
        effective_config_hash,
        token_budget,
        plugins_snapshot,
    } = &first.event
    else {
        return Err(SessionError::Corrupted {
            reason: "first_event_must_be_session_started".into(),
        });
    };

    let mut resolved_plugins_snapshot = plugins_snapshot.clone().unwrap_or_default();

    for record in &records[1..] {
        if let SessionEvent::SessionPluginsSnapshot { plugins_snapshot } = &record.event {
            resolved_plugins_snapshot = plugins_snapshot.clone();
        }
    }

    Ok(ExpectedProjection {
        working_dir: working_dir.clone(),
        permission_mode: *permission_mode,
        model: model.clone(),
        config_snapshot_id: config_snapshot_id.clone(),
        effective_config_hash: effective_config_hash.clone(),
        token_budget: *token_budget,
        plugins_snapshot: resolved_plugins_snapshot,
        head_event_id: last.event_id.clone(),
        usage: project_usage(records.iter().map(|record| &record.event))?,
    })
}

fn projection_needs_repair(
    connection: &rusqlite::Connection,
    session_id: &SessionId,
    records: &[JsonlRecord],
    expected: &ExpectedProjection,
) -> Result<bool, SessionError> {
    let Some(row) = load_session_row(connection, session_id)?
    else {
        return Ok(true);
    };

    let db_event_ids = load_event_ids(connection, session_id)?;
    let jsonl_event_ids = records
        .iter()
        .map(|record| record.event_id.0.clone())
        .collect::<Vec<_>>();
    let plugins_snapshot = serde_json::from_str::<PluginsSnapshot>(&row.plugins_snapshot_json)?;
    let usage = serde_json::from_str::<Usage>(&row.usage_json)?;

    Ok(row.working_dir != expected.working_dir
        || deserialize_permission_mode(&row.permission_mode)? != expected.permission_mode
        || row.model != expected.model
        || row.config_snapshot_id != expected.config_snapshot_id
        || row.effective_config_hash != expected.effective_config_hash
        || row.token_budget != expected.token_budget
        || plugins_snapshot != expected.plugins_snapshot
        || row.head_event_id != expected.head_event_id.0
        || usage != expected.usage
        || db_event_ids != jsonl_event_ids)
}
