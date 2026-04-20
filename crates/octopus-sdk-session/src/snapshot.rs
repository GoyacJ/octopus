use octopus_sdk_contracts::{EventId, SessionId, Usage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub id: SessionId,
    pub config_snapshot_id: String,
    pub effective_config_hash: String,
    pub head_event_id: EventId,
    pub usage: Usage,
}

#[cfg(test)]
mod tests {
    use octopus_sdk_contracts::{EventId, SessionId, Usage};

    use super::SessionSnapshot;

    #[test]
    fn session_snapshot_keeps_contract_shape() {
        let snapshot = SessionSnapshot {
            id: SessionId("session-1".into()),
            config_snapshot_id: "cfg-1".into(),
            effective_config_hash: "hash-1".into(),
            head_event_id: EventId("event-1".into()),
            usage: Usage::default(),
        };

        assert_eq!(snapshot.config_snapshot_id, "cfg-1");
        assert_eq!(snapshot.head_event_id.0, "event-1");
    }
}
