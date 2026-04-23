use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use octopus_sdk_contracts::{EventId, SessionEvent, SessionId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::SessionError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct JsonlRecord {
    pub event_id: EventId,
    pub event: SessionEvent,
}

enum ParsedJsonlLine {
    Record(JsonlRecord),
    SkipLegacyRuntimeEnvelope,
}

pub(crate) fn append_record(
    jsonl_root: &Path,
    session_id: &SessionId,
    event_id: &EventId,
    event: &SessionEvent,
) -> Result<(), SessionError> {
    fs::create_dir_all(jsonl_root)?;

    let record = JsonlRecord {
        event_id: event_id.clone(),
        event: event.clone(),
    };
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(session_path(jsonl_root, session_id))?;
    let line = serde_json::to_string(&record)?;

    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;
    file.sync_all()?;

    Ok(())
}

pub(crate) fn session_path(jsonl_root: &Path, session_id: &SessionId) -> PathBuf {
    jsonl_root.join(format!("{}.jsonl", session_id.0))
}

pub(crate) fn read_records(
    jsonl_root: &Path,
    session_id: &SessionId,
) -> Result<Vec<JsonlRecord>, SessionError> {
    let path = session_path(jsonl_root, session_id);

    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    let mut saw_legacy_runtime_envelope = false;

    for (line_index, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match parse_line(trimmed, session_id, line_index + 1)? {
            ParsedJsonlLine::Record(record) => {
                if saw_legacy_runtime_envelope {
                    return Err(SessionError::Corrupted {
                        reason: "mixed_legacy_runtime_envelope_and_session_records".into(),
                    });
                }
                records.push(record);
            }
            ParsedJsonlLine::SkipLegacyRuntimeEnvelope => {
                if !records.is_empty() {
                    return Err(SessionError::Corrupted {
                        reason: "mixed_session_records_and_legacy_runtime_envelope".into(),
                    });
                }
                saw_legacy_runtime_envelope = true;
            }
        }
    }

    if saw_legacy_runtime_envelope {
        return Ok(Vec::new());
    }

    Ok(records)
}

pub(crate) fn list_session_ids(jsonl_root: &Path) -> Result<Vec<SessionId>, SessionError> {
    if !jsonl_root.exists() {
        return Ok(Vec::new());
    }

    let mut session_ids = fs::read_dir(jsonl_root)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                return None;
            }

            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(|stem| SessionId(stem.to_owned()))
        })
        .collect::<Vec<_>>();

    session_ids.sort_by(|left, right| left.0.cmp(&right.0));

    Ok(session_ids)
}

fn parse_line(
    trimmed: &str,
    session_id: &SessionId,
    line_number: usize,
) -> Result<ParsedJsonlLine, SessionError> {
    match serde_json::from_str::<JsonlRecord>(trimmed) {
        Ok(record) => Ok(ParsedJsonlLine::Record(record)),
        Err(current_error) => parse_legacy_line(trimmed, session_id, line_number, current_error),
    }
}

fn parse_legacy_line(
    trimmed: &str,
    session_id: &SessionId,
    line_number: usize,
    current_error: serde_json::Error,
) -> Result<ParsedJsonlLine, SessionError> {
    let value = match serde_json::from_str::<Value>(trimmed) {
        Ok(value) => value,
        Err(_) => return Err(SessionError::from(current_error)),
    };

    if looks_like_legacy_runtime_envelope(&value) {
        return Ok(ParsedJsonlLine::SkipLegacyRuntimeEnvelope);
    }

    if let Some(record) = parse_legacy_event_wrapper(&value, session_id, line_number)? {
        return Ok(ParsedJsonlLine::Record(record));
    }

    match serde_json::from_value::<SessionEvent>(value) {
        Ok(event) => Ok(ParsedJsonlLine::Record(JsonlRecord {
            event_id: legacy_event_id(session_id, line_number),
            event,
        })),
        Err(_) => Err(SessionError::from(current_error)),
    }
}

fn parse_legacy_event_wrapper(
    value: &Value,
    session_id: &SessionId,
    line_number: usize,
) -> Result<Option<JsonlRecord>, SessionError> {
    let Value::Object(map) = value else {
        return Ok(None);
    };
    let Some(event_value) = map.get("event").cloned() else {
        return Ok(None);
    };
    let event = serde_json::from_value::<SessionEvent>(event_value)?;
    let event_id = map
        .get("event_id")
        .cloned()
        .and_then(|value| serde_json::from_value::<EventId>(value).ok())
        .or_else(|| {
            map.get("id")
                .and_then(|value| value.as_str())
                .map(|value| EventId(value.to_owned()))
        })
        .unwrap_or_else(|| legacy_event_id(session_id, line_number));

    Ok(Some(JsonlRecord { event_id, event }))
}

fn looks_like_legacy_runtime_envelope(value: &Value) -> bool {
    let Value::Object(map) = value else {
        return false;
    };

    map.contains_key("eventType")
        || (map.contains_key("payload") && map.contains_key("sessionId") && map.contains_key("id"))
}

fn legacy_event_id(session_id: &SessionId, line_number: usize) -> EventId {
    EventId(format!("legacy-{}-{}", session_id.0, line_number))
}
