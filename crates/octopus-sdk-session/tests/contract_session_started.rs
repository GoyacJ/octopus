use std::fs;

use octopus_sdk_contracts::{
    ContentBlock, Message, PluginSourceTag, PluginSummary, PluginsSnapshot, Role, SessionEvent,
    SessionId,
};
use octopus_sdk_session::{SessionError, SessionStore, SqliteJsonlSessionStore};
use uuid::Uuid;

#[tokio::test]
async fn first_event_must_be_session_started() {
    let root =
        std::env::temp_dir().join(format!("octopus-sdk-session-contract-{}", Uuid::new_v4()));
    let db_path = root.join("data").join("main.db");
    let jsonl_root = root.join("runtime").join("events");
    fs::create_dir_all(db_path.parent().expect("db parent")).expect("db dir should exist");

    let store = SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should open");
    let session_id = SessionId("session-contract-corrupted".into());
    let event = SessionEvent::UserMessage(Message {
        role: Role::User,
        content: vec![ContentBlock::Text {
            text: "hello".into(),
        }],
    });

    let error = store
        .append(&session_id, event)
        .await
        .expect_err("non-SessionStarted first event should fail");

    assert!(matches!(
        error,
        SessionError::Corrupted {
            reason
        } if reason == "first_event_must_be_session_started"
    ));
}

#[tokio::test]
async fn session_started_fields_roundtrip_into_snapshot() {
    let root =
        std::env::temp_dir().join(format!("octopus-sdk-session-contract-{}", Uuid::new_v4()));
    let db_path = root.join("data").join("main.db");
    let jsonl_root = root.join("runtime").join("events");
    fs::create_dir_all(db_path.parent().expect("db parent")).expect("db dir should exist");

    let store = SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should open");
    let session_id = SessionId("session-contract-started".into());
    let plugins_snapshot = sample_plugins_snapshot();

    store
        .append_session_started(
            &session_id,
            std::path::PathBuf::from("."),
            octopus_sdk_contracts::PermissionMode::Default,
            "main".into(),
            "cfg-contract".into(),
            "hash-contract".into(),
            8_192,
            Some(plugins_snapshot.clone()),
        )
        .await
        .expect("session started should append");

    let snapshot = store
        .snapshot(&session_id)
        .await
        .expect("snapshot should load");

    assert_eq!(snapshot.config_snapshot_id, "cfg-contract");
    assert_eq!(snapshot.effective_config_hash, "hash-contract");
    assert_eq!(snapshot.plugins_snapshot, plugins_snapshot);
}

fn sample_plugins_snapshot() -> PluginsSnapshot {
    PluginsSnapshot {
        api_version: "1.0.0".into(),
        plugins: vec![PluginSummary {
            id: "example-noop-tool".into(),
            version: "0.1.0".into(),
            git_sha: Some("0123456789abcdef0123456789abcdef01234567".into()),
            source: PluginSourceTag::Bundled,
            enabled: true,
            components_count: 1,
        }],
    }
}
