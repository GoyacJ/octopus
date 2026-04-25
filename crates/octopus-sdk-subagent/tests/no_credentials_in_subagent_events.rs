use std::{
    fs,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use octopus_sdk_context::DurableScratchpad;
use octopus_sdk_contracts::{
    AskAnswer, AskError, AskPrompt, AskResolver, EventId, EventSink, HookDecision, HookEvent,
    PermissionMode, PermissionOutcome, PluginsSnapshot, RenderBlock, RenderKind, RenderLifecycle,
    RenderMeta, SessionEvent, SessionId, SubagentSpec, TaskBudget, ToolCallId, ToolCallRequest,
    ToolCategory,
};
use octopus_sdk_hooks::{Hook, HookSource};
use octopus_sdk_model::{
    ModelError, ModelProvider, ModelRequest, ModelStream, ProtocolFamily, ProviderDescriptor,
    ProviderId,
};
use octopus_sdk_observability::{session_span_id, session_trace_id, NoopTracer};
use octopus_sdk_permissions::{
    ApprovalBroker, DefaultPermissionGate, PermissionBehavior, PermissionPolicy, PermissionRule,
    PermissionRuleSource,
};
use octopus_sdk_session::{SessionStore, SqliteJsonlSessionStore};
use octopus_sdk_subagent::{ParentSessionContext, ParentTraceContext, SubagentContext};
use octopus_sdk_tools::ToolRegistry;
use serde_json::json;

const SECRET_VALUE: &str = "s3cret-xyz";

struct SilentModelProvider;

#[async_trait]
impl ModelProvider for SilentModelProvider {
    async fn complete(&self, _req: ModelRequest) -> Result<ModelStream, ModelError> {
        Ok(Box::pin(futures::stream::empty()))
    }

    fn describe(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: ProviderId("mock".into()),
            supported_families: vec![ProtocolFamily::VendorNative],
            catalog_version: "test".into(),
        }
    }
}

struct SecretRewriteHook;

#[async_trait]
impl Hook for SecretRewriteHook {
    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "secret-rewrite"
    }

    async fn on_event(&self, event: &HookEvent) -> HookDecision {
        match event {
            HookEvent::PreToolUse { call, .. } => {
                HookDecision::Rewrite(octopus_sdk_contracts::RewritePayload::ToolCall {
                    call: ToolCallRequest {
                        id: call.id.clone(),
                        name: call.name.clone(),
                        input: json!({
                            "command": "curl https://example.invalid",
                            "api_key": SECRET_VALUE,
                        }),
                    },
                })
            }
            _ => HookDecision::Continue,
        }
    }
}

struct FixedAskResolver;

#[async_trait]
impl AskResolver for FixedAskResolver {
    async fn resolve(&self, _prompt_id: &str, _prompt: &AskPrompt) -> Result<AskAnswer, AskError> {
        Ok(AskAnswer {
            prompt_id: "approval:call-secret".into(),
            option_id: "approve".into(),
            text: "approve".into(),
        })
    }
}

struct RecordingEventSink {
    events: Arc<Mutex<Vec<SessionEvent>>>,
}

impl EventSink for RecordingEventSink {
    fn emit(&self, event: SessionEvent) {
        self.events
            .lock()
            .expect("events mutex should stay available")
            .push(event);
    }
}

#[tokio::test]
async fn test_parent_session_events_do_not_include_secret_after_hook_and_permission() {
    let root = tempfile::tempdir().expect("tempdir should create");
    let db_path = root.path().join("data").join("main.db");
    let jsonl_root = root.path().join("runtime").join("events");
    let store =
        Arc::new(SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should open"));
    let parent_session = SessionId("parent-subagent-redaction".into());
    let emitted_events = Arc::new(Mutex::new(Vec::new()));

    store
        .append_session_started(
            &parent_session,
            std::path::PathBuf::from("."),
            octopus_sdk_contracts::PermissionMode::Default,
            "main".into(),
            "cfg-parent".into(),
            "hash-parent".into(),
            8_192,
            Some(PluginsSnapshot::default()),
        )
        .await
        .expect("parent session should start");

    let gate = DefaultPermissionGate::new(
        PermissionPolicy::from_sources(vec![PermissionRule {
            source: PermissionRuleSource::Session,
            behavior: PermissionBehavior::Ask,
            tool_name: "shell_exec".into(),
            rule_content: Some("*".into()),
        }]),
        PermissionMode::Default,
        Arc::new(ApprovalBroker::new(
            Arc::new(RecordingEventSink {
                events: Arc::clone(&emitted_events),
            }),
            Arc::new(FixedAskResolver),
        )),
        Arc::new(|_| ToolCategory::Shell),
    );
    let parent = ParentSessionContext {
        session_id: parent_session.clone(),
        session_store: store.clone(),
        model: Arc::new(SilentModelProvider),
        tools: Arc::new(ToolRegistry::new()),
        permissions: Arc::new(gate),
        scratchpad: DurableScratchpad::new(root.path().to_path_buf()),
        trace: ParentTraceContext {
            trace_id: session_trace_id(&parent_session.0),
            span_id: session_span_id(&parent_session.0),
            agent_role: "main".into(),
            model_id: "main".into(),
            model_version: "test".into(),
            config_snapshot_id: "cfg-parent".into(),
            tracer: Arc::new(NoopTracer),
        },
    };
    let context = SubagentContext::from_parent(parent, sample_spec());

    context.hooks.register(
        "rewrite",
        Arc::new(SecretRewriteHook),
        HookSource::Workspace,
        10,
    );

    let outcome = context
        .hooks
        .run(HookEvent::PreToolUse {
            call: ToolCallRequest {
                id: ToolCallId("call-secret".into()),
                name: "shell_exec".into(),
                input: json!({ "command": "echo safe" }),
            },
            category: ToolCategory::Shell,
        })
        .await
        .expect("hook run should succeed");
    let rewritten_call = match outcome.final_payload {
        Some(octopus_sdk_contracts::RewritePayload::ToolCall { call }) => call,
        other => panic!("expected rewritten tool call, got {other:?}"),
    };

    let permission_outcome = context.permissions.check(&rewritten_call).await;
    assert_eq!(permission_outcome, PermissionOutcome::Allow);

    let events = emitted_events
        .lock()
        .expect("events mutex should stay available")
        .clone();
    for event in events {
        store
            .append(&parent_session, event)
            .await
            .expect("approval event should append");
    }

    store
        .append(
            &parent_session,
            SessionEvent::Render {
                blocks: vec![RenderBlock {
                    kind: RenderKind::Markdown,
                    payload: json!({
                        "title": "subagent.summary",
                        "text": "shell_exec approved without exposing raw tool input",
                    }),
                    meta: RenderMeta {
                        id: EventId::new_v4(),
                        parent: None,
                        ts_ms: 0,
                    },
                }],
                lifecycle: RenderLifecycle::assistant_message(),
            },
        )
        .await
        .expect("summary event should append");

    let json = fs::read_to_string(jsonl_root.join(format!("{}.jsonl", parent_session.0)))
        .expect("parent jsonl should exist");
    assert!(!json.contains(SECRET_VALUE));
    assert!(!json.contains("api_key"));
}

fn sample_spec() -> SubagentSpec {
    SubagentSpec {
        id: "reviewer".into(),
        system_prompt: "Be concise.".into(),
        allowed_tools: vec!["shell_exec".into()],
        agent_role: "worker".into(),
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
