use std::path::PathBuf;

use octopus_sdk_contracts::{EventId, PermissionMode, PluginsSnapshot, SessionId, Usage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub id: SessionId,
    pub working_dir: PathBuf,
    pub permission_mode: PermissionMode,
    pub model: String,
    pub config_snapshot_id: String,
    pub effective_config_hash: String,
    pub token_budget: u32,
    pub plugins_snapshot: PluginsSnapshot,
    pub head_event_id: EventId,
    pub usage: Usage,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use octopus_sdk_contracts::{EventId, PermissionMode, PluginsSnapshot, SessionId, Usage};

    use super::SessionSnapshot;

    #[test]
    fn session_snapshot_keeps_contract_shape() {
        let snapshot = SessionSnapshot {
            id: SessionId("session-1".into()),
            working_dir: PathBuf::from("/tmp/octopus"),
            permission_mode: PermissionMode::Default,
            model: "main".into(),
            config_snapshot_id: "cfg-1".into(),
            effective_config_hash: "hash-1".into(),
            token_budget: 8_192,
            plugins_snapshot: PluginsSnapshot::default(),
            head_event_id: EventId("event-1".into()),
            usage: Usage::default(),
        };

        assert_eq!(snapshot.working_dir, PathBuf::from("/tmp/octopus"));
        assert_eq!(snapshot.permission_mode, PermissionMode::Default);
        assert_eq!(snapshot.model, "main");
        assert_eq!(snapshot.config_snapshot_id, "cfg-1");
        assert_eq!(snapshot.token_budget, 8_192);
        assert!(snapshot.plugins_snapshot.plugins.is_empty());
        assert_eq!(snapshot.head_event_id.0, "event-1");
    }
}
