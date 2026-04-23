use std::sync::{Arc, Mutex};

use octopus_sdk_contracts::{
    AssistantEvent, ContentBlock, Message, PermissionMode, PermissionOutcome, Role, SessionEvent,
    SessionId, SubagentSummary, Usage,
};
use octopus_sdk_contracts::{RenderLifecycle, RenderPhase, ToolCallId};
use octopus_sdk_observability::{
    session_span_id, session_trace_id, ReplayTracer, TraceSpan, Tracer, UsageLedger,
};
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
            SessionEvent::PermissionDecision {
                call: octopus_sdk_contracts::ToolCallId("call-1".into()),
                name: "bash".into(),
                mode: PermissionMode::Default,
                outcome: PermissionOutcome::Allow,
            },
        )
        .await
        .expect("permission decision should append");
    store
        .append(
            &session_id,
            SessionEvent::Render {
                blocks: vec![octopus_sdk_contracts::RenderBlock {
                    kind: octopus_sdk_contracts::RenderKind::Text,
                    payload: serde_json::json!({ "text": "done" }),
                    meta: octopus_sdk_contracts::RenderMeta {
                        id: octopus_sdk_contracts::EventId::new_v4(),
                        parent: None,
                        ts_ms: 1,
                    },
                }],
                lifecycle: RenderLifecycle::tool_phase(
                    RenderPhase::OnToolResult,
                    ToolCallId("call-1".into()),
                    "bash",
                ),
            },
        )
        .await
        .expect("render should append");
    store
        .append(
            &session_id,
            SessionEvent::Render {
                blocks: vec![octopus_sdk_contracts::RenderBlock {
                    kind: octopus_sdk_contracts::RenderKind::Markdown,
                    payload: serde_json::json!({
                        "title": "subagent.summary",
                        "text": "child worker done",
                        "summary": SubagentSummary {
                            session_id: SessionId("child-1".into()),
                            parent_session_id: session_id.clone(),
                            resume_session_id: Some(SessionId("child-1".into())),
                            spec_id: "worker-1".into(),
                            agent_role: "worker".into(),
                            parent_agent_role: "main".into(),
                            turns: 2,
                            tokens_used: 9,
                            duration_ms: 12,
                            trace_id: session_trace_id(&session_id.0),
                            span_id: "subagent:child-1".into(),
                            parent_span_id: session_span_id(&session_id.0),
                            model_id: "main".into(),
                            model_version: "test".into(),
                            config_snapshot_id: "cfg-1".into(),
                            permission_mode: PermissionMode::Default,
                            allowed_tools: vec!["bash".into()],
                        },
                    }),
                    meta: octopus_sdk_contracts::RenderMeta {
                        id: octopus_sdk_contracts::EventId::new_v4(),
                        parent: None,
                        ts_ms: 2,
                    },
                }],
                lifecycle: RenderLifecycle::assistant_message(),
            },
        )
        .await
        .expect("subagent summary should append");

    let tracer = RecordingTracer::new();
    let ledger = UsageLedger::new();
    ReplayTracer::replay_session(store.as_ref(), &session_id, &tracer, &ledger)
        .await
        .expect("replay should succeed");

    let snapshot = ledger.snapshot();
    assert_eq!(snapshot.sessions_started, 1);
    assert_eq!(snapshot.tool_calls, 1);
    assert_eq!(snapshot.permission_decisions, 1);
    assert_eq!(snapshot.renders, 2);
    assert_eq!(snapshot.subagent_summaries, 1);
    assert_eq!(snapshot.model_usage.input_tokens, 4);
    assert_eq!(snapshot.model_usage.output_tokens, 6);
    assert_eq!(
        tracer
            .spans
            .lock()
            .expect("spans lock should stay available")
            .len(),
        6
    );
    let spans = tracer
        .spans
        .lock()
        .expect("spans lock should stay available")
        .clone();
    let expected_trace_id = format!("trace:{}", session_id.0);
    let expected_parent_span_id = session_span_id(&session_id.0);
    assert!(spans.iter().any(|span| {
        span.name == "session_event"
            && span.trace_id.as_deref() == Some(expected_trace_id.as_str())
            && span.span_id.as_deref() == Some("permission:call-1")
            && span.parent_span_id.as_deref() == Some("tool:call-1")
            && span.agent_role.as_deref() == Some("main")
            && span.fields.get("permission_decision")
                == Some(&octopus_sdk_observability::TraceValue::String(
                    "Allow".into(),
                ))
    }));
    assert!(spans.iter().any(|span| {
        span.name == "subagent_summary"
            && span.trace_id.as_deref() == Some(expected_trace_id.as_str())
            && span.span_id.as_deref() == Some("subagent:child-1")
            && span.parent_span_id.as_deref() == Some(expected_parent_span_id.as_str())
            && span.agent_role.as_deref() == Some("worker")
    }));
}
