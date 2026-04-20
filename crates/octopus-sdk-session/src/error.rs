use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session not found")]
    NotFound,
    #[error("session data corrupted: {reason}")]
    Corrupted { reason: String },
    #[error("session IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("session sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("session serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::SessionError;

    #[test]
    fn session_error_messages_are_stable() {
        assert_eq!(SessionError::NotFound.to_string(), "session not found");
        assert_eq!(
            SessionError::Corrupted {
                reason: "first_event_must_be_session_started".into(),
            }
            .to_string(),
            "session data corrupted: first_event_must_be_session_started"
        );
    }
}
