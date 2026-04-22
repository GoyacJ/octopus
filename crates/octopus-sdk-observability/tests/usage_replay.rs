use std::sync::{Arc, Mutex};

use octopus_sdk_contracts::{AssistantEvent, ContentBlock, Message, Role, SessionEvent, Usage};
use octopus_sdk_observability::{ReplayTracer, TraceSpan, Tracer, UsageLedger};
use octopus_sdk_session::{SessionStore, SqliteJsonlSessionStore};

struct RecordingTracer {
    spans: Mutex<Vec<TraceSpan>>,
}

impl RecordingTracer {
    fn new() -> Self {
        Self {
            spans: Mutex::new(Vec::new()),
        }
    }
}

impl Tracer for RecordingTracer {
    fn record(&self, span: TraceSpan) {
        self.spans
            .lock()
            .expect("spans lock should stay available")
            .push(span);
    }
}

#[tokio::test]
async fn test_replay_tracer_usage() {
    let root = tempfile::tempdir().expect("tempdir should exist");
    let store = Arc::new(
        SqliteJsonlSessionStore::open(
            &root.path().join("data/main.db"),
            &root.path().join("runtime/events"),
        )
        .expect("session store should open"),
    );
    let session_id = octopus_sdk_contracts::SessionId::new_v4();
    store
        .append_session_started(
            &session_id,
            std::path::PathBuf::from("."),
            octopus_sdk_contracts::PermissionMode::Default,
            "main".into(),
            "cfg-1".into(),
            "hash-1".into(),
            8_192,
            Some(octopus_sdk_contracts::PluginsSnapshot::default()),
        )
        .await
        .expect("session start should append");
    store
        .append(
            &session_id,
            SessionEvent::AssistantMessage(Message {
                role: Role::Assistant,
                content: vec![ContentBlock::Text {
                    text: serde_json::to_string(&AssistantEvent::Usage(Usage {
                        input_tokens: 4,
                        output_tokens: 6,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    }))
                    .expect("usage marker should serialize"),
                }],
            }),
        )
        .await
        .expect("usage marker should append");
    store
        .append(
            &session_id,
            SessionEvent::ToolExecuted {
                call: octopus_sdk_contracts::ToolCallId("call-1".into()),
                name: "bash".into(),
                duration_ms: 10,
                is_error: false,
            },
        )
        .await
        .expect("tool executed should append");
    store
        .append(
            &session_id,
            SessionEvent::Render {
                block: octopus_sdk_contracts::RenderBlock {
                    kind: octopus_sdk_contracts::RenderKind::Text,
                    payload: serde_json::json!({ "text": "done" }),
                    meta: octopus_sdk_contracts::RenderMeta {
                        id: octopus_sdk_contracts::EventId::new_v4(),
                        parent: None,
                        ts_ms: 1,
                    },
                },
                lifecycle: octopus_sdk_contracts::RenderLifecycle::OnToolResult,
            },
        )
        .await
        .expect("render should append");

    let tracer = RecordingTracer::new();
    let ledger = UsageLedger::new();
    ReplayTracer::replay_session(store.as_ref(), &session_id, &tracer, &ledger)
        .await
        .expect("replay should succeed");

    let snapshot = ledger.snapshot();
    assert_eq!(snapshot.sessions_started, 1);
    assert_eq!(snapshot.tool_calls, 1);
    assert_eq!(snapshot.renders, 1);
    assert_eq!(snapshot.model_usage.input_tokens, 4);
    assert_eq!(snapshot.model_usage.output_tokens, 6);
    assert_eq!(
        tracer
            .spans
            .lock()
            .expect("spans lock should stay available")
            .len(),
        4
    );
}
