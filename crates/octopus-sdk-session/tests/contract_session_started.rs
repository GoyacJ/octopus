use std::fs;

use octopus_sdk_contracts::{ContentBlock, Message, Role, SessionEvent, SessionId};
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

    store
        .append(
            &session_id,
            SessionEvent::SessionStarted {
                config_snapshot_id: "cfg-contract".into(),
                effective_config_hash: "hash-contract".into(),
            },
        )
        .await
        .expect("session started should append");

    let snapshot = store
        .snapshot(&session_id)
        .await
        .expect("snapshot should load");

    assert_eq!(snapshot.config_snapshot_id, "cfg-contract");
    assert_eq!(snapshot.effective_config_hash, "hash-contract");
}
