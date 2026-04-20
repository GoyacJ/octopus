use futures::stream;
use octopus_sdk_contracts::{EventId, SessionEvent, SessionId, Usage};
use rusqlite::{params, OptionalExtension};

use crate::{jsonl::JsonlRecord, EventRange, EventStream, SessionError, SessionSnapshot};

use super::{append::project_usage, event_kind, now_millis, SqliteJsonlSessionStore};

impl SqliteJsonlSessionStore {
    pub(crate) fn stream_events(
        &self,
        session_id: &SessionId,
        range: EventRange,
    ) -> Result<EventStream, SessionError> {
        let connection = self.open_connection()?;

        if !session_exists(&connection, session_id)? {
            return Err(SessionError::NotFound);
        }

        let after_seq = match range.after {
            Some(after) => event_sequence(&connection, session_id, &after)?.ok_or(SessionError::NotFound)?,
            None => 0,
        };

        let events = if let Some(limit) = range.limit {
            let limit = i64::try_from(limit).map_err(|_| SessionError::Corrupted {
                reason: "range_limit_overflows_i64".into(),
            })?;
            let mut statement = connection.prepare(
                "
                SELECT payload
                FROM events
                WHERE session_id = ?1 AND seq > ?2
                ORDER BY seq ASC
                LIMIT ?3
                ",
            )?;
            let rows = statement
                .query_map(params![session_id.0, after_seq, limit], |row| row.get::<_, String>(0))?;

            rows.map(|row| -> Result<SessionEvent, SessionError> {
                Ok(serde_json::from_str(&row?)?)
            })
            .collect::<Result<Vec<_>, _>>()?
        } else {
            let mut statement = connection.prepare(
                "
                SELECT payload
                FROM events
                WHERE session_id = ?1 AND seq > ?2
                ORDER BY seq ASC
                ",
            )?;
            let rows =
                statement.query_map(params![session_id.0, after_seq], |row| row.get::<_, String>(0))?;

            rows.map(|row| -> Result<SessionEvent, SessionError> {
                Ok(serde_json::from_str(&row?)?)
            })
            .collect::<Result<Vec<_>, _>>()?
        };

        Ok(Box::pin(stream::iter(
            events.into_iter().map(Result::<SessionEvent, SessionError>::Ok),
        )))
    }

    pub(crate) fn load_snapshot(&self, session_id: &SessionId) -> Result<SessionSnapshot, SessionError> {
        let connection = self.open_connection()?;
        let row = load_session_row(&connection, session_id)?
            .ok_or(SessionError::NotFound)?;

        Ok(SessionSnapshot {
            id: session_id.clone(),
            config_snapshot_id: row.0,
            effective_config_hash: row.1,
            head_event_id: EventId(row.2),
            usage: serde_json::from_str::<Usage>(&row.3)?,
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
        let max_seq = event_sequence(&connection, session_id, from_event_id)?.ok_or(SessionError::NotFound)?;
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
                forked_session_id.0,
                source.0,
                source.1,
                head_event_id.0,
                source.3,
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
        transaction.execute("DELETE FROM events WHERE session_id = ?1", [session_id.0.as_str()])?;
        transaction.execute("DELETE FROM sessions WHERE session_id = ?1", [session_id.0.as_str()])?;
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
                expected.config_snapshot_id,
                expected.effective_config_hash,
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

fn load_session_row(
    connection: &rusqlite::Connection,
    session_id: &SessionId,
) -> Result<Option<(String, String, String, String)>, SessionError> {
    connection
        .query_row(
            "
            SELECT config_snapshot_id, effective_config_hash, head_event_id, usage_json
            FROM sessions
            WHERE session_id = ?1
            ",
            [session_id.0.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
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

    rows.collect::<Result<Vec<_>, _>>().map_err(SessionError::from)
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
    let rows = statement.query_map(params![session_id.0, after_seq], |row| row.get::<_, String>(0))?;

    rows.map(|row| -> Result<SessionEvent, SessionError> {
        Ok(serde_json::from_str(&row?)?)
    })
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
            SessionEvent::Checkpoint { anchor_event_id, .. } => Some((seq, anchor_event_id)),
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
    config_snapshot_id: String,
    effective_config_hash: String,
    head_event_id: EventId,
    usage: Usage,
}

fn expected_projection(records: &[JsonlRecord]) -> Result<ExpectedProjection, SessionError> {
    let first = records
        .first()
        .ok_or(SessionError::Corrupted {
            reason: "jsonl_session_has_no_events".into(),
        })?;
    let last = records
        .last()
        .ok_or(SessionError::Corrupted {
            reason: "jsonl_session_has_no_events".into(),
        })?;

    let SessionEvent::SessionStarted {
        config_snapshot_id,
        effective_config_hash,
    } = &first.event
    else {
        return Err(SessionError::Corrupted {
            reason: "first_event_must_be_session_started".into(),
        });
    };

    Ok(ExpectedProjection {
        config_snapshot_id: config_snapshot_id.clone(),
        effective_config_hash: effective_config_hash.clone(),
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
    let Some((config_snapshot_id, effective_config_hash, head_event_id, usage_json)) =
        load_session_row(connection, session_id)?
    else {
        return Ok(true);
    };

    let db_event_ids = load_event_ids(connection, session_id)?;
    let jsonl_event_ids = records
        .iter()
        .map(|record| record.event_id.0.clone())
        .collect::<Vec<_>>();
    let usage = serde_json::from_str::<Usage>(&usage_json)?;

    Ok(config_snapshot_id != expected.config_snapshot_id
        || effective_config_hash != expected.effective_config_hash
        || head_event_id != expected.head_event_id.0
        || usage != expected.usage
        || db_event_ids != jsonl_event_ids)
}
