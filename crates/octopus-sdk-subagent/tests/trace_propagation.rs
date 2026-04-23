use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::StreamExt;
use octopus_sdk_context::DurableScratchpad;
use octopus_sdk_contracts::{
    AssistantEvent, PermissionGate, PermissionMode, PermissionOutcome, SessionEvent, SessionId,
    StopReason, SubagentOutput, SubagentSpec, SubagentSummary, TaskBudget, ToolCallId,
    ToolCallRequest, ToolCategory,
};
use octopus_sdk_model::{
    ModelError, ModelProvider, ModelRequest, ModelStream, ProtocolFamily, ProviderDescriptor,
    ProviderId,
};
use octopus_sdk_observability::{session_span_id, session_trace_id, TraceSpan, Tracer};
use octopus_sdk_session::{EventRange, SessionStore, SqliteJsonlSessionStore};
use octopus_sdk_subagent::{OrchestratorWorkers, ParentSessionContext, ParentTraceContext};
use octopus_sdk_tools::{Tool, ToolContext, ToolError, ToolRegistry, ToolResult, ToolSpec};

struct AllowAllGate;

#[async_trait]
impl PermissionGate for AllowAllGate {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        PermissionOutcome::Allow
    }
}

struct RecordingTracer {
    spans: Mutex<Vec<TraceSpan>>,
}

impl RecordingTracer {
    fn new() -> Self {
        Self {
            spans: Mutex::new(Vec::new()),
        }
    }

    fn snapshot(&self) -> Vec<TraceSpan> {
        self.spans
            .lock()
            .expect("spans lock should stay available")
            .clone()
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

struct ScriptedProvider {
    turns: Mutex<Vec<Vec<AssistantEvent>>>,
}

#[async_trait]
impl ModelProvider for ScriptedProvider {
    async fn complete(&self, _req: ModelRequest) -> Result<ModelStream, ModelError> {
        Ok(Box::pin(futures::stream::iter(
            self.turns
                .lock()
                .expect("turns lock should stay available")
                .remove(0)
                .into_iter()
                .map(Ok),
        )))
    }

    fn describe(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: ProviderId("mock".into()),
            supported_families: vec![ProtocolFamily::VendorNative],
            catalog_version: "test".into(),
        }
    }
}

struct DummyTool {
    spec: ToolSpec,
}

impl DummyTool {
    fn new(name: &str) -> Self {
        Self {
            spec: ToolSpec {
                name: name.into(),
                description: format!("{name} tool"),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
                category: ToolCategory::Read,
            },
        }
    }
}

#[async_trait]
impl Tool for DummyTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    fn is_concurrency_safe(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        _input: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::default())
    }
}

#[tokio::test]
async fn test_subagent_trace_propagation() {
    let root = tempfile::tempdir().expect("tempdir should create");
    let store = Arc::new(
        SqliteJsonlSessionStore::open(
            &root.path().join("data/main.db"),
            &root.path().join("runtime/events"),
        )
        .expect("store should open"),
    );
    let parent_session = SessionId("parent-trace".into());
    let tracer = Arc::new(RecordingTracer::new());

    store
        .append_session_started(
            &parent_session,
            std::path::PathBuf::from("."),
            PermissionMode::Default,
            "main".into(),
            "cfg-parent".into(),
            "hash-parent".into(),
            8_192,
            Some(octopus_sdk_contracts::PluginsSnapshot::default()),
        )
        .await
        .expect("parent session should start");

    let parent = ParentSessionContext {
        session_id: parent_session.clone(),
        session_store: store.clone(),
        model: Arc::new(ScriptedProvider {
            turns: Mutex::new(vec![
                vec![
                    AssistantEvent::ToolUse {
                        id: ToolCallId("tool-a-1".into()),
                        name: "ToolA".into(),
                        input: serde_json::json!({}),
                    },
                    AssistantEvent::Usage(octopus_sdk_contracts::Usage {
                        input_tokens: 2,
                        output_tokens: 2,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    }),
                    AssistantEvent::MessageStop {
                        stop_reason: StopReason::ToolUse,
                    },
                ],
                vec![
                    AssistantEvent::TextDelta("worker done".into()),
                    AssistantEvent::Usage(octopus_sdk_contracts::Usage {
                        input_tokens: 2,
                        output_tokens: 3,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    }),
                    AssistantEvent::MessageStop {
                        stop_reason: StopReason::EndTurn,
                    },
                ],
            ]),
        }),
        tools: Arc::new(tool_registry(vec!["ToolA"])),
        permissions: Arc::new(AllowAllGate),
        scratchpad: DurableScratchpad::new(root.path().to_path_buf()),
        trace: ParentTraceContext {
            trace_id: session_trace_id(&parent_session.0),
            span_id: session_span_id(&parent_session.0),
            agent_role: "main".into(),
            model_id: "main".into(),
            model_version: "test".into(),
            config_snapshot_id: "cfg-parent".into(),
            tracer: tracer.clone(),
        },
    };

    let outputs = OrchestratorWorkers::new(parent, 2)
        .run(vec![sample_spec()], vec!["trace this".into()])
        .await;
    let output = outputs
        .into_iter()
        .next()
        .expect("worker output should exist")
        .expect("worker should succeed");
    let expected_trace_id = session_trace_id("parent-trace");
    let expected_parent_span_id = session_span_id("parent-trace");

    let SubagentOutput::Summary { meta, .. } = output else {
        panic!("expected summary output");
    };
    assert_eq!(meta.parent_session_id, parent_session);
    assert_eq!(meta.resume_session_id, Some(meta.session_id.clone()));
    assert_eq!(meta.spec_id, "researcher");
    assert_eq!(meta.agent_role, "sub.research");
    assert_eq!(meta.parent_agent_role, "main");
    assert_eq!(meta.trace_id, expected_trace_id);
    assert_eq!(meta.span_id, format!("subagent:{}", meta.session_id.0));
    assert_eq!(meta.parent_span_id, expected_parent_span_id);
    assert_eq!(meta.model_id, "main");
    assert_eq!(meta.model_version, "test");
    assert_eq!(meta.config_snapshot_id, "cfg-parent");
    assert_eq!(meta.permission_mode, PermissionMode::Default);
    assert_eq!(meta.allowed_tools, vec!["ToolA"]);

    let child_events = collect_events(store.as_ref(), &meta.session_id).await;
    assert!(child_events.iter().any(|event| matches!(
        event,
        SessionEvent::Render { blocks, .. }
            if blocks.iter().any(|block| {
                block.payload["title"] == "subagent.spawn"
                    && block.payload["summary"]["trace_id"] == serde_json::json!(meta.trace_id)
                    && block.payload["summary"]["agent_role"] == "sub.research"
            })
    )));

    let parent_events = collect_events(store.as_ref(), &parent_session).await;
    assert!(parent_events.iter().any(|event| {
        let SessionEvent::Render { blocks, .. } = event else {
            return false;
        };
        blocks.iter().any(|block| {
            if block.payload["title"] != "subagent.summary" {
                return false;
            }
            let Ok(summary) =
                serde_json::from_value::<SubagentSummary>(block.payload["summary"].clone())
            else {
                return false;
            };
            summary.session_id == meta.session_id
                && summary.parent_session_id == parent_session
                && summary.resume_session_id.is_none()
                && summary.spec_id == "fan-in"
                && summary.agent_role == "coordinator"
                && summary.parent_agent_role == "main"
                && summary.trace_id == meta.trace_id
                && summary.span_id == format!("subagent-fan-in:{}", parent_session.0)
                && summary.parent_span_id == meta.parent_span_id
                && summary.model_id == meta.model_id
                && summary.model_version == meta.model_version
                && summary.config_snapshot_id == meta.config_snapshot_id
                && summary.permission_mode == meta.permission_mode
                && summary.allowed_tools == meta.allowed_tools
        })
    }));

    let spans = tracer.snapshot();
    assert!(spans.iter().any(|span| {
        span.name == "subagent_spawn"
            && span.trace_id.as_deref() == Some(meta.trace_id.as_str())
            && span.span_id.as_deref() == Some(meta.span_id.as_str())
            && span.parent_span_id.as_deref() == Some(meta.parent_span_id.as_str())
            && span.agent_role.as_deref() == Some("sub.research")
    }));
    assert!(spans.iter().any(|span| {
        span.name == "subagent_permission_decision"
            && span.agent_role.as_deref() == Some("sub.research")
            && span.fields.get("permission_decision")
                == Some(&octopus_sdk_observability::TraceValue::String(
                    "allow".into(),
                ))
    }));
    assert!(spans.iter().any(|span| {
        span.name == "subagent_tool_execution"
            && span.agent_role.as_deref() == Some("sub.research")
            && span.fields.get("tool_name")
                == Some(&octopus_sdk_observability::TraceValue::String(
                    "ToolA".into(),
                ))
    }));
    assert!(spans.iter().any(|span| {
        span.name == "subagent_complete"
            && span.agent_role.as_deref() == Some("sub.research")
            && span.fields.get("spec_id")
                == Some(&octopus_sdk_observability::TraceValue::String(
                    "researcher".into(),
                ))
    }));
}

async fn collect_events(
    store: &SqliteJsonlSessionStore,
    session_id: &SessionId,
) -> Vec<SessionEvent> {
    store
        .stream(session_id, EventRange::default())
        .await
        .expect("stream should open")
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(|event| event.expect("event should decode"))
        .collect::<Vec<_>>()
}

fn sample_spec() -> SubagentSpec {
    SubagentSpec {
        id: "researcher".into(),
        system_prompt: "Be concise.".into(),
        allowed_tools: vec!["ToolA".into()],
        agent_role: "sub.research".into(),
        model_role: "subagent-default".into(),
        permission_mode: PermissionMode::Default,
        task_budget: TaskBudget {
            total: 100,
            completion_threshold: 0.9,
        },
        max_turns: 2,
        depth: 1,
    }
}

fn tool_registry(tool_names: Vec<&str>) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    for name in tool_names {
        registry
            .register(Arc::new(DummyTool::new(name)))
            .expect("tool should register");
    }

    registry
}
