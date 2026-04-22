use std::{fs, path::PathBuf};

use octopus_sdk_contracts::{
    AskPrompt, AssistantEvent, ContentBlock, EndReason, EventId, Message, PluginSourceTag,
    PluginSummary, PluginsSnapshot, RenderBlock, RenderKind, RenderLifecycle, RenderMeta, Role,
    SessionEvent, StopReason, ToolCallId, Usage,
};
use octopus_sdk_session::{EventRange, SessionStore, SqliteJsonlSessionStore};
use rusqlite::params;
use serde_json::json;
use uuid::Uuid;

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
            "DELETE FROM events WHERE session_id = ?1 AND seq > 8",
            params![session_id.0.as_str()],
        )
        .expect("tail rows should delete");
    connection
        .execute(
            "UPDATE sessions SET head_event_id = ?2, usage_json = ?3 WHERE session_id = ?1",
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
