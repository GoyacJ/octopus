use std::{fs, path::PathBuf};

use futures::StreamExt;
use octopus_sdk_contracts::{
    ContentBlock, EndReason, EventId, Message, Role, SessionEvent, SessionId,
};
use octopus_sdk_session::{EventRange, SessionError, SessionStore, SqliteJsonlSessionStore};
use uuid::Uuid;

#[tokio::test]
async fn test_fork_preserves_prefix() {
    let paths = test_paths("fork");
    let store = SqliteJsonlSessionStore::open(&paths.db_path, &paths.jsonl_root)
        .expect("store should open");
    let session_id = SessionId("session-fork".into());

    let started_id = store
        .append(
            &session_id,
            SessionEvent::SessionStarted {
                config_snapshot_id: "cfg-fork".into(),
                effective_config_hash: "hash-fork".into(),
            },
        )
        .await
        .expect("session started should append");
    let message_id = store
        .append(
            &session_id,
            SessionEvent::UserMessage(Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "open checkpoint".into(),
                }],
            }),
        )
        .await
        .expect("message should append");
    let checkpoint_id = store
        .append(
            &session_id,
            SessionEvent::Checkpoint {
                id: "checkpoint-fork".into(),
                anchor_event_id: message_id.clone(),
            },
        )
        .await
        .expect("checkpoint should append");
    store
        .append(
            &session_id,
            SessionEvent::SessionEnded {
                reason: EndReason::Normal,
            },
        )
        .await
        .expect("session ended should append");

    let forked_session = store
        .fork(&session_id, checkpoint_id.clone())
        .await
        .expect("fork should succeed");

    let forked_events = read_all_events(&store, &forked_session).await;
    let forked_snapshot = store
        .snapshot(&forked_session)
        .await
        .expect("forked snapshot should load");

    assert_ne!(forked_session, session_id);
    assert_eq!(
        forked_events,
        vec![
            SessionEvent::SessionStarted {
                config_snapshot_id: "cfg-fork".into(),
                effective_config_hash: "hash-fork".into(),
            },
            SessionEvent::UserMessage(Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "open checkpoint".into(),
                }],
            }),
            SessionEvent::Checkpoint {
                id: "checkpoint-fork".into(),
                anchor_event_id: message_id,
            },
        ]
    );
    assert_eq!(forked_snapshot.config_snapshot_id, "cfg-fork");
    assert_eq!(forked_snapshot.effective_config_hash, "hash-fork");
    assert_ne!(started_id, checkpoint_id);
}

#[tokio::test]
async fn test_wake_returns_latest_snapshot() {
    let paths = test_paths("wake");
    let store = SqliteJsonlSessionStore::open(&paths.db_path, &paths.jsonl_root)
        .expect("store should open");
    let session_id = SessionId("session-wake".into());

    let _started_id = store
        .append(
            &session_id,
            SessionEvent::SessionStarted {
                config_snapshot_id: "cfg-fork".into(),
                effective_config_hash: "hash-fork".into(),
            },
        )
        .await
        .expect("session started should append");
    let anchor_event_id = store
        .append(
            &session_id,
            SessionEvent::UserMessage(Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "resume from checkpoint".into(),
                }],
            }),
        )
        .await
        .expect("message should append");
    store
        .append(
            &session_id,
            SessionEvent::Checkpoint {
                id: "checkpoint-wake".into(),
                anchor_event_id,
            },
        )
        .await
        .expect("checkpoint should append");
    let last_event_id = store
        .append(
            &session_id,
            SessionEvent::SessionEnded {
                reason: EndReason::Normal,
            },
        )
        .await
        .expect("session ended should append");

    let snapshot = store
        .wake(&session_id)
        .await
        .expect("wake should return snapshot");

    assert_eq!(snapshot.id, session_id);
    assert_eq!(snapshot.config_snapshot_id, "cfg-fork");
    assert_eq!(snapshot.effective_config_hash, "hash-fork");
    assert_eq!(snapshot.head_event_id, last_event_id);
}

#[tokio::test]
async fn test_wake_rejects_checkpoint_with_missing_anchor() {
    let paths = test_paths("wake-missing-anchor");
    let store = SqliteJsonlSessionStore::open(&paths.db_path, &paths.jsonl_root)
        .expect("store should open");
    let session_id = SessionId("session-wake-missing-anchor".into());

    store
        .append(
            &session_id,
            SessionEvent::SessionStarted {
                config_snapshot_id: "cfg-fork".into(),
                effective_config_hash: "hash-fork".into(),
            },
        )
        .await
        .expect("session started should append");
    store
        .append(
            &session_id,
            SessionEvent::Checkpoint {
                id: "checkpoint-wake".into(),
                anchor_event_id: EventId("missing-anchor".into()),
            },
        )
        .await
        .expect("checkpoint should append");

    let error = store
        .wake(&session_id)
        .await
        .expect_err("wake should reject a checkpoint with a missing anchor event");

    assert!(matches!(
        error,
        SessionError::Corrupted { reason }
        if reason == "checkpoint_anchor_event_not_found"
    ));
}

struct TestPaths {
    db_path: PathBuf,
    jsonl_root: PathBuf,
}

fn test_paths(label: &str) -> TestPaths {
    let root = std::env::temp_dir().join(format!("octopus-sdk-session-{label}-{}", Uuid::new_v4()));
    let db_path = root.join("data").join("main.db");
    let jsonl_root = root.join("runtime").join("events");
    fs::create_dir_all(db_path.parent().expect("db parent")).expect("db dir should exist");
    TestPaths {
        db_path,
        jsonl_root,
    }
}

async fn read_all_events(
    store: &SqliteJsonlSessionStore,
    session_id: &SessionId,
) -> Vec<SessionEvent> {
    let stream = store
        .stream(session_id, EventRange::default())
        .await
        .expect("stream should succeed");

    stream
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(|result| result.expect("stream item should succeed"))
        .collect()
}
