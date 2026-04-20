use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use octopus_sdk_contracts::{EventId, SessionEvent, SessionId};
use serde::{Deserialize, Serialize};

use crate::SessionError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct JsonlRecord {
    pub event_id: EventId,
    pub event: SessionEvent,
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

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        records.push(serde_json::from_str::<JsonlRecord>(trimmed)?);
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
