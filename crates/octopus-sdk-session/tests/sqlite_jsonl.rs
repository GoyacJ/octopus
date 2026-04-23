use std::{fs, path::PathBuf};

use octopus_sdk_contracts::{
    AskPrompt, AssistantEvent, ContentBlock, EndReason, EventId, Message, PluginSourceTag,
    PluginSummary, PluginsSnapshot, RenderBlock, RenderKind, RenderLifecycle, RenderMeta, Role,
    SessionEvent, StopReason, ToolCallId, Usage,
};
use octopus_sdk_session::{EventRange, SessionError, SessionStore, SqliteJsonlSessionStore};
use rusqlite::params;
use serde_json::json;
use uuid::Uuid;

const SESSIONS_TABLE: &str = "runtime_session_store_sessions";
const EVENTS_TABLE: &str = "runtime_session_store_events";

#[tokio::test]
async fn test_append_roundtrip() {
    let paths = test_paths("append");
    let store = SqliteJsonlSessionStore::open(&paths.db_path, &paths.jsonl_root)
        .expect("store should open");
    let session_id = octopus_sdk_contracts::SessionId("session-roundtrip".into());
    let expected = sample_events();

    for event in &expected {
        store
            .append(&session_id, event.clone())
            .await
            .expect("append should succeed");
    }

    let actual = read_all_events(&store, &session_id).await;
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn test_stream_after_cursor() {
    let paths = test_paths("stream");
    let store = SqliteJsonlSessionStore::open(&paths.db_path, &paths.jsonl_root)
        .expect("store should open");
    let session_id = octopus_sdk_contracts::SessionId("session-stream".into());
    let expected = sample_events();
    let mut event_ids = Vec::new();

    for event in &expected {
        let event_id = store
            .append(&session_id, event.clone())
            .await
            .expect("append should succeed");
        event_ids.push(event_id);
    }

    let stream = store
        .stream(
            &session_id,
            EventRange {
                after: Some(event_ids[3].clone()),
                limit: Some(3),
            },
        )
        .await
        .expect("stream should succeed");

    let actual = collect_stream(stream).await;
    assert_eq!(actual, expected[4..7].to_vec());
}

#[tokio::test]
async fn test_snapshot_matches_last_event() {
    let paths = test_paths("snapshot");
    let store = SqliteJsonlSessionStore::open(&paths.db_path, &paths.jsonl_root)
        .expect("store should open");
    let session_id = octopus_sdk_contracts::SessionId("session-snapshot".into());

    for event in sample_events() {
        store
            .append(&session_id, event)
            .await
            .expect("append should succeed");
    }

    let snapshot = store
        .snapshot(&session_id)
        .await
        .expect("snapshot should load");

    assert_eq!(snapshot.id.0, "session-snapshot");
    assert_eq!(snapshot.config_snapshot_id, "cfg-1");
    assert_eq!(snapshot.effective_config_hash, "hash-1");
    assert_eq!(snapshot.plugins_snapshot, sample_plugins_snapshot());
    assert_eq!(
        snapshot.usage,
        Usage {
            input_tokens: 7,
            output_tokens: 11,
            cache_creation_input_tokens: 13,
            cache_read_input_tokens: 17,
        }
    );
}

#[tokio::test]
async fn test_open_repairs_db_projection_from_jsonl_tail() {
    let paths = test_paths("repair");
    let session_id = octopus_sdk_contracts::SessionId("session-repair".into());
    let expected = sample_events();
    let store = SqliteJsonlSessionStore::open(&paths.db_path, &paths.jsonl_root)
        .expect("store should open");
    let mut event_ids = Vec::new();

    for event in &expected {
        let event_id = store
            .append(&session_id, event.clone())
            .await
            .expect("append should succeed");
        event_ids.push(event_id);
    }

    let connection = rusqlite::Connection::open(&paths.db_path).expect("sqlite db should open");
    connection
        .execute(
            &format!("DELETE FROM {EVENTS_TABLE} WHERE session_id = ?1 AND seq > 8"),
            params![session_id.0.as_str()],
        )
        .expect("tail rows should delete");
    connection
        .execute(
            &format!(
                "UPDATE {SESSIONS_TABLE} SET head_event_id = ?2, usage_json = ?3 WHERE session_id = ?1"
            ),
            params![
                session_id.0.as_str(),
                event_ids[7].0.as_str(),
                serde_json::to_string(&Usage::default()).expect("usage should serialize")
            ],
        )
        .expect("session row should update");

    let repaired = SqliteJsonlSessionStore::open(&paths.db_path, &paths.jsonl_root)
        .expect("store should reopen");
    let actual = read_all_events(&repaired, &session_id).await;
    let snapshot = repaired
        .snapshot(&session_id)
        .await
        .expect("snapshot should load after repair");

    assert_eq!(actual, expected);
    assert_eq!(snapshot.head_event_id, event_ids[9]);
    assert_eq!(snapshot.plugins_snapshot, sample_plugins_snapshot());
    assert_eq!(
        snapshot.usage,
        Usage {
            input_tokens: 7,
            output_tokens: 11,
            cache_creation_input_tokens: 13,
            cache_read_input_tokens: 17,
        }
    );
}

#[tokio::test]
async fn test_open_replays_legacy_event_only_jsonl_records() {
    let paths = test_paths("legacy-event-only");
    let session_id = octopus_sdk_contracts::SessionId("session-legacy-event-only".into());
    let expected = sample_events();
    let path = paths.jsonl_root.join(format!("{}.jsonl", session_id.0));

    fs::create_dir_all(&paths.jsonl_root).expect("jsonl root should exist");
    fs::write(
        &path,
        format!(
            "{}\n",
            expected
                .iter()
                .map(|event| serde_json::to_string(event).expect("event should serialize"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    )
    .expect("legacy jsonl should write");

    let reopened = SqliteJsonlSessionStore::open(&paths.db_path, &paths.jsonl_root)
        .expect("store should reopen");
    let actual = read_all_events(&reopened, &session_id).await;
    let snapshot = reopened
        .snapshot(&session_id)
        .await
        .expect("snapshot should load after replay");

    assert_eq!(actual, expected);
    assert_eq!(
        snapshot.head_event_id.0,
        format!("legacy-{}-{}", session_id.0, snapshot_event_count())
    );
}

#[tokio::test]
async fn test_open_replays_latest_session_started_from_legacy_jsonl_records() {
    let paths = test_paths("legacy-rebind-session-started");
    let session_id = octopus_sdk_contracts::SessionId("session-legacy-rebind".into());
    let path = paths.jsonl_root.join(format!("{}.jsonl", session_id.0));
    let events = vec![
        SessionEvent::SessionStarted {
            working_dir: ".".into(),
            permission_mode: octopus_sdk_contracts::PermissionMode::Default,
            model: "main".into(),
            config_snapshot_id: "cfg-1".into(),
            effective_config_hash: "hash-1".into(),
            token_budget: 8_192,
            plugins_snapshot: Some(sample_plugins_snapshot()),
        },
        SessionEvent::UserMessage(Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Switch the configured model.".into(),
            }],
        }),
        SessionEvent::SessionStarted {
            working_dir: "./apps/desktop".into(),
            permission_mode: octopus_sdk_contracts::PermissionMode::AcceptEdits,
            model: "claude-sonnet-4-5".into(),
            config_snapshot_id: "cfg-2".into(),
            effective_config_hash: "hash-2".into(),
            token_budget: 4_096,
            plugins_snapshot: Some(sample_plugins_snapshot()),
        },
    ];

    fs::create_dir_all(&paths.jsonl_root).expect("jsonl root should exist");
    fs::write(
        &path,
        format!(
            "{}\n",
            events
                .iter()
                .map(|event| serde_json::to_string(event).expect("event should serialize"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    )
    .expect("legacy jsonl should write");

    let reopened = SqliteJsonlSessionStore::open(&paths.db_path, &paths.jsonl_root)
        .expect("store should reopen");
    let actual = read_all_events(&reopened, &session_id).await;
    let snapshot = reopened
        .snapshot(&session_id)
        .await
        .expect("snapshot should load after replay");

    assert_eq!(actual, events);
    assert_eq!(snapshot.working_dir, PathBuf::from("./apps/desktop"));
    assert_eq!(
        snapshot.permission_mode,
        octopus_sdk_contracts::PermissionMode::AcceptEdits
    );
    assert_eq!(snapshot.model, "claude-sonnet-4-5");
    assert_eq!(snapshot.config_snapshot_id, "cfg-2");
    assert_eq!(snapshot.effective_config_hash, "hash-2");
    assert_eq!(snapshot.token_budget, 4_096);
    assert_eq!(snapshot.plugins_snapshot, sample_plugins_snapshot());
    assert_eq!(snapshot.head_event_id.0, format!("legacy-{}-3", session_id.0));
}

#[tokio::test]
async fn test_open_skips_legacy_runtime_envelope_jsonl_files() {
    let paths = test_paths("legacy-runtime-envelope");
    let session_id = octopus_sdk_contracts::SessionId("rt-legacy-runtime-envelope".into());
    let path = paths.jsonl_root.join(format!("{}.jsonl", session_id.0));

    fs::create_dir_all(&paths.jsonl_root).expect("jsonl root should exist");
    fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string(&json!({
                "id": "evt-legacy-runtime-envelope",
                "eventType": "runtime.session.updated",
                "kind": "runtime.session.updated",
                "sessionId": session_id.0,
                "payload": {
                    "summary": {
                        "id": session_id.0,
                        "status": "idle"
                    }
                }
            }))
            .expect("legacy runtime envelope should serialize")
        ),
    )
    .expect("legacy runtime envelope should write");

    let reopened = SqliteJsonlSessionStore::open(&paths.db_path, &paths.jsonl_root)
        .expect("store should reopen");
    let missing = reopened
        .snapshot(&session_id)
        .await
        .expect_err("session should skip");
    let connection = rusqlite::Connection::open(&paths.db_path).expect("sqlite db should open");
    let sessions: i64 = connection
        .query_row(
            &format!("SELECT COUNT(*) FROM {SESSIONS_TABLE} WHERE session_id = ?1"),
            [session_id.0.as_str()],
            |row| row.get(0),
        )
        .expect("count should query");

    assert!(matches!(missing, SessionError::NotFound));
    assert_eq!(sessions, 0);
}

#[tokio::test]
async fn test_append_uses_namespaced_runtime_tables_when_auth_tables_exist() {
    let paths = test_paths("auth-shared-db");
    let session_id = octopus_sdk_contracts::SessionId("session-auth-shared-db".into());
    let expected = sample_events();
    let connection = rusqlite::Connection::open(&paths.db_path).expect("sqlite db should open");
    connection
        .execute_batch(
            "
            CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                client_app_id TEXT NOT NULL,
                token TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER
            );

            CREATE TABLE events (
                event_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                seq INTEGER NOT NULL,
                kind TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            ",
        )
        .expect("shared auth tables should create");
    connection
        .execute(
            "
            INSERT INTO sessions (
                id,
                workspace_id,
                user_id,
                client_app_id,
                token,
                status,
                created_at,
                expires_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ",
            params![
                "auth-session-1",
                "workspace-1",
                "user-1",
                "desktop",
                "token-1",
                "active",
                1_i64,
                Option::<i64>::None
            ],
        )
        .expect("auth session row should insert");
    drop(connection);

    let store = SqliteJsonlSessionStore::open(&paths.db_path, &paths.jsonl_root)
        .expect("store should open");
    for event in &expected {
        store
            .append(&session_id, event.clone())
            .await
            .expect("append should succeed");
    }

    let actual = read_all_events(&store, &session_id).await;
    let snapshot = store
        .snapshot(&session_id)
        .await
        .expect("snapshot should load");
    let connection = rusqlite::Connection::open(&paths.db_path).expect("sqlite db should reopen");
    let auth_sessions: i64 = connection
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
        .expect("auth session rows should query");
    let legacy_events: i64 = connection
        .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
        .expect("legacy event rows should query");
    let runtime_sessions: i64 = connection
        .query_row(
            &format!("SELECT COUNT(*) FROM {SESSIONS_TABLE} WHERE session_id = ?1"),
            [session_id.0.as_str()],
            |row| row.get(0),
        )
        .expect("runtime session rows should query");
    let runtime_events: i64 = connection
        .query_row(
            &format!("SELECT COUNT(*) FROM {EVENTS_TABLE} WHERE session_id = ?1"),
            [session_id.0.as_str()],
            |row| row.get(0),
        )
        .expect("runtime event rows should query");

    assert_eq!(actual, expected);
    assert_eq!(snapshot.id, session_id);
    assert_eq!(snapshot.plugins_snapshot, sample_plugins_snapshot());
    assert_eq!(auth_sessions, 1);
    assert_eq!(legacy_events, 0);
    assert_eq!(runtime_sessions, 1);
    assert_eq!(runtime_events, expected.len() as i64);
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
    session_id: &octopus_sdk_contracts::SessionId,
) -> Vec<SessionEvent> {
    let stream = store
        .stream(session_id, EventRange::default())
        .await
        .expect("stream should succeed");

    collect_stream(stream).await
}

async fn collect_stream(stream: octopus_sdk_session::EventStream) -> Vec<SessionEvent> {
    use futures::StreamExt;

    stream
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(|result| result.expect("stream item should succeed"))
        .collect()
}

fn sample_events() -> Vec<SessionEvent> {
    let session_started = SessionEvent::SessionStarted {
        working_dir: ".".into(),
        permission_mode: octopus_sdk_contracts::PermissionMode::Default,
        model: "main".into(),
        config_snapshot_id: "cfg-1".into(),
        effective_config_hash: "hash-1".into(),
        token_budget: 8_192,
        plugins_snapshot: Some(sample_plugins_snapshot()),
    };
    let user_message = SessionEvent::UserMessage(Message {
        role: Role::User,
        content: vec![ContentBlock::Text {
            text: "Open the repo state.".into(),
        }],
    });
    let assistant_message = SessionEvent::AssistantMessage(Message {
        role: Role::Assistant,
        content: vec![ContentBlock::Text {
            text: "Inspecting the latest checkpoint.".into(),
        }],
    });
    let tool_executed = SessionEvent::ToolExecuted {
        call: ToolCallId("call-1".into()),
        name: "read_file".into(),
        duration_ms: 42,
        is_error: false,
    };
    let render = SessionEvent::Render {
        block: RenderBlock {
            kind: RenderKind::Record,
            payload: json!({
                "rows": [{ "label": "changed_files", "value": "3" }],
                "title": "Workspace State"
            }),
            meta: RenderMeta {
                id: EventId("event-render".into()),
                parent: None,
                ts_ms: 1_713_692_800_123,
            },
        },
        lifecycle: RenderLifecycle::OnToolResult,
    };
    let ask = SessionEvent::Ask {
        prompt: AskPrompt {
            kind: "ask-user".into(),
            questions: vec![],
        },
    };
    let checkpoint = SessionEvent::Checkpoint {
        id: "checkpoint-1".into(),
        anchor_event_id: EventId("event-4".into()),
        compaction: None,
    };
    let session_ended = SessionEvent::SessionEnded {
        reason: EndReason::Normal,
    };
    let assistant_usage = SessionEvent::AssistantMessage(Message {
        role: Role::Assistant,
        content: vec![
            ContentBlock::Thinking {
                text: "Reasoning complete".into(),
            },
            ContentBlock::ToolResult {
                tool_use_id: ToolCallId("call-1".into()),
                content: vec![ContentBlock::Text {
                    text: serde_json::to_string(&AssistantEvent::Usage(Usage {
                        input_tokens: 7,
                        output_tokens: 11,
                        cache_creation_input_tokens: 13,
                        cache_read_input_tokens: 17,
                    }))
                    .expect("usage should serialize"),
                }],
                is_error: false,
            },
        ],
    });
    let message_stop = SessionEvent::AssistantMessage(Message {
        role: Role::Assistant,
        content: vec![ContentBlock::Text {
            text: serde_json::to_string(&AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            })
            .expect("message stop should serialize"),
        }],
    });

    vec![
        session_started,
        user_message,
        assistant_message,
        tool_executed,
        render,
        ask,
        checkpoint,
        session_ended,
        assistant_usage,
        message_stop,
    ]
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

fn snapshot_event_count() -> usize {
    sample_events().len()
}
