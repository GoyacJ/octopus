//! 临时 mini-engine，M3 期专用 E2E 闭环验证脚手架。
//!
//! TODO(M5-T15): 完成真 engine 后由 `crates/octopus-harness-engine/tests/e2e_engine.rs` 替代；
//!               M5-T15 任务卡完成后必须 `git rm` 本文件。
//!
//! 治理来源：docs/plans/harness-sdk/milestones/M3-l2-core.md M3-T20

use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use futures::{stream, StreamExt};
use harness_context::{ContextEngine, ContextSessionView};
use harness_contracts::{
    CapabilityRegistry, CausationId, ConfigHash, CorrelationId, DecidedBy, Decision, DecisionId,
    DecisionScope, EndReason, Event, EventId, FallbackPolicy, HookEventKind, InteractivityLevel,
    Message, MessageContent, MessageId, MessageMetadata, MessagePart, MessageRole, ModelProvider,
    NoopRedactor, PermissionMode, PermissionRequestedEvent, PermissionResolvedEvent,
    PermissionSubject, ProviderRestriction, RunEndedEvent, RunId, RunStartedEvent, StopReason,
    TenantId, ToolDescriptor, ToolGroup, ToolOrigin, ToolProperties, ToolResult,
    ToolUseApprovedEvent, ToolUseCompletedEvent, ToolUseId, ToolUseRequestedEvent, TrustLevel,
    TurnInput, UsageSnapshot,
};
use harness_hook::{
    HookContext, HookDispatcher, HookEvent, HookHandler, HookOutcome, HookRegistry,
    HookSessionView, ReplayMode,
};
use harness_journal::{EventStore, InMemoryEventStore, ReplayCursor};
use harness_permission::{PermissionBroker, PermissionContext, PermissionRequest, RuleSnapshot};
use harness_session::{Session, SessionOptions};
use harness_tool::{
    default_result_budget, BuiltinToolset, InterruptToken, OrchestratorContext,
    SchemaResolverContext, Tool, ToolCall, ToolContext, ToolEvent, ToolOrchestrator, ToolPool,
    ToolPoolFilter, ToolPoolModelProfile, ToolRegistry, ToolSearchMode, ToolStream,
    ValidationError,
};
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::sync::Mutex;

#[tokio::test]
async fn minimal_driver_runs_list_dir_turn() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("marker.txt"), "m3").unwrap();

    let tenant_id = TenantId::SINGLE;
    let session_id = harness_contracts::SessionId::new();
    let event_store = Arc::new(InMemoryEventStore::new(Arc::new(NoopRedactor)));
    let session = Session::builder()
        .with_options(
            SessionOptions::new(workspace.path())
                .with_tenant_id(tenant_id)
                .with_session_id(session_id),
        )
        .with_event_store(event_store.clone())
        .build()
        .await
        .unwrap();

    let driver = MiniDriver::new(workspace, tenant_id, session_id, event_store.clone()).await;

    let answer = driver.run_turn(&session, "list current dir").await.unwrap();

    assert!(answer.contains("marker.txt"));
    assert_eq!(driver.user_prompt_hooks.load(Ordering::SeqCst), 1);
    assert!(events_for(event_store, tenant_id, session_id)
        .await
        .iter()
        .any(|event| matches!(event, Event::RunEnded(_))));
}

struct MiniDriver {
    workspace: TempDir,
    tenant_id: TenantId,
    session_id: harness_contracts::SessionId,
    event_store: Arc<InMemoryEventStore>,
    hooks: HookDispatcher,
    context: ContextEngine,
    tools: ToolPool,
    broker: Arc<AllowBroker>,
    user_prompt_hooks: Arc<AtomicUsize>,
}

impl MiniDriver {
    async fn new(
        workspace: TempDir,
        tenant_id: TenantId,
        session_id: harness_contracts::SessionId,
        event_store: Arc<InMemoryEventStore>,
    ) -> Self {
        let user_prompt_hooks = Arc::new(AtomicUsize::new(0));
        let hooks = HookRegistry::builder()
            .with_hook(Box::new(CountingHook {
                calls: user_prompt_hooks.clone(),
            }))
            .build()
            .unwrap();
        let context = ContextEngine::builder().build().unwrap();
        let registry = ToolRegistry::builder()
            .with_builtin_toolset(BuiltinToolset::Custom(vec![Box::new(
                TestListDirTool::new(),
            )]))
            .build()
            .unwrap();
        let tools = ToolPool::assemble(
            &registry.snapshot(),
            &ToolPoolFilter::default(),
            &ToolSearchMode::Disabled,
            &ToolPoolModelProfile {
                provider: ModelProvider("mock".to_owned()),
                supports_tool_reference: false,
                max_context_tokens: Some(8_000),
            },
            &SchemaResolverContext {
                run_id: RunId::new(),
                session_id,
                tenant_id,
            },
        )
        .await
        .unwrap();

        Self {
            workspace,
            tenant_id,
            session_id,
            event_store,
            hooks: HookDispatcher::new(hooks.snapshot()),
            context,
            tools,
            broker: Arc::new(AllowBroker::default()),
            user_prompt_hooks,
        }
    }

    async fn run_turn(&self, session: &Session, prompt: &str) -> Result<String, String> {
        session
            .run_turn(prompt)
            .await
            .map_err(|error| error.to_string())?;

        let run_id = RunId::new();
        let user_message = message(
            MessageRole::User,
            vec![MessagePart::Text(prompt.to_owned())],
        );
        let turn_input = TurnInput {
            message: user_message.clone(),
            metadata: json!({ "turn": 1 }),
        };
        self.append(&[Event::RunStarted(RunStartedEvent {
            run_id,
            session_id: self.session_id,
            tenant_id: self.tenant_id,
            parent_run_id: None,
            input: turn_input.clone(),
            snapshot_id: session.snapshot_id(),
            effective_config_hash: ConfigHash([0; 32]),
            started_at: harness_contracts::now(),
            correlation_id: CorrelationId::new(),
        })])
        .await?;

        self.hooks
            .dispatch(
                HookEvent::UserPromptSubmit {
                    run_id,
                    input: json!({ "prompt": prompt }),
                },
                self.hook_context(run_id),
            )
            .await
            .map_err(|error| error.to_string())?;

        let prompt_view = PromptView {
            tenant_id: self.tenant_id,
            session_id: self.session_id,
            descriptors: self
                .tools
                .iter()
                .map(|tool| tool.descriptor().clone())
                .collect(),
            messages: Vec::new(),
        };
        self.context
            .assemble(&prompt_view, &turn_input)
            .await
            .map_err(|error| error.to_string())?;

        let tool_call = mock_llm_tool_call(run_id, self.workspace.path());
        let descriptor = self
            .tools
            .descriptor(&tool_call.tool_name)
            .ok_or_else(|| "tool descriptor missing".to_owned())?
            .clone();
        self.append(&[
            Event::UserMessageAppended(harness_contracts::UserMessageAppendedEvent {
                run_id,
                message_id: user_message.id,
                content: MessageContent::Text(prompt.to_owned()),
                metadata: MessageMetadata::default(),
                at: harness_contracts::now(),
            }),
            Event::ToolUseRequested(ToolUseRequestedEvent {
                run_id,
                tool_use_id: tool_call.tool_use_id,
                tool_name: tool_call.tool_name.clone(),
                input: tool_call.input.clone(),
                properties: descriptor.properties.clone(),
                causation_id: EventId::new(),
                at: harness_contracts::now(),
            }),
        ])
        .await?;

        let results = ToolOrchestrator::default()
            .dispatch(vec![tool_call.clone()], self.orchestrator_context(run_id))
            .await;
        let result = results
            .into_iter()
            .next()
            .ok_or_else(|| "tool result missing".to_owned())?;
        let tool_result = result.result.map_err(|error| error.to_string())?;
        let permission = self
            .broker
            .take_requests()
            .await
            .pop()
            .ok_or_else(|| "permission request missing".to_owned())?;
        let decision_id = DecisionId::new();
        self.append(&[
            Event::PermissionRequested(PermissionRequestedEvent {
                request_id: permission.request_id,
                run_id,
                session_id: self.session_id,
                tenant_id: self.tenant_id,
                tool_use_id: permission.tool_use_id,
                tool_name: permission.tool_name.clone(),
                subject: permission.subject.clone(),
                severity: permission.severity,
                scope_hint: permission.scope_hint.clone(),
                fingerprint: None,
                presented_options: vec![Decision::AllowOnce, Decision::DenyOnce],
                interactivity: InteractivityLevel::NoInteractive,
                causation_id: EventId::new(),
                at: harness_contracts::now(),
            }),
            Event::PermissionResolved(PermissionResolvedEvent {
                request_id: permission.request_id,
                decision: Decision::AllowOnce,
                decided_by: DecidedBy::Broker {
                    broker_id: "m3-mini-driver".to_owned(),
                },
                scope: permission.scope_hint,
                fingerprint: None,
                rationale: None,
                at: harness_contracts::now(),
            }),
            Event::ToolUseApproved(ToolUseApprovedEvent {
                tool_use_id: tool_call.tool_use_id,
                decision_id,
                scope: DecisionScope::ToolName(tool_call.tool_name.clone()),
                at: harness_contracts::now(),
            }),
            Event::ToolUseCompleted(ToolUseCompletedEvent {
                tool_use_id: tool_call.tool_use_id,
                result: tool_result.clone(),
                usage: None,
                duration_ms: result.duration.as_millis().min(u128::from(u64::MAX)) as u64,
                at: harness_contracts::now(),
            }),
        ])
        .await?;

        self.context
            .after_turn(
                &prompt_view,
                &[harness_contracts::ToolResultEnvelope {
                    result: tool_result.clone(),
                    usage: None,
                    is_error: false,
                    overflow: result.overflow,
                }],
            )
            .await
            .map_err(|error| error.to_string())?;

        let answer = format!("ListDir result: {}", tool_result_summary(&tool_result));
        self.append(&[
            Event::AssistantMessageCompleted(harness_contracts::AssistantMessageCompletedEvent {
                run_id,
                message_id: MessageId::new(),
                content: MessageContent::Text(answer.clone()),
                tool_uses: vec![harness_contracts::ToolUseSummary {
                    tool_use_id: tool_call.tool_use_id,
                    tool_name: tool_call.tool_name.clone(),
                }],
                usage: UsageSnapshot::default(),
                pricing_snapshot_id: None,
                stop_reason: StopReason::EndTurn,
                at: harness_contracts::now(),
            }),
            Event::RunEnded(RunEndedEvent {
                run_id,
                reason: EndReason::Completed,
                usage: Some(UsageSnapshot::default()),
                ended_at: harness_contracts::now(),
            }),
        ])
        .await?;

        Ok(answer)
    }

    async fn append(&self, events: &[Event]) -> Result<(), String> {
        self.event_store
            .append(self.tenant_id, self.session_id, events)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn hook_context(&self, run_id: RunId) -> HookContext {
        HookContext {
            tenant_id: self.tenant_id,
            session_id: self.session_id,
            run_id: Some(run_id),
            turn_index: Some(1),
            correlation_id: CorrelationId::new(),
            causation_id: CausationId::new(),
            trust_level: TrustLevel::UserControlled,
            permission_mode: PermissionMode::Default,
            interactivity: InteractivityLevel::NoInteractive,
            at: harness_contracts::now(),
            view: Arc::new(HookView {
                workspace_root: self.workspace.path().to_path_buf(),
                redactor: NoopRedactor,
            }),
            upstream_outcome: None,
            replay_mode: ReplayMode::Live,
        }
    }

    fn orchestrator_context(&self, run_id: RunId) -> OrchestratorContext {
        OrchestratorContext {
            pool: self.tools.clone(),
            tool_context: ToolContext {
                tool_use_id: ToolUseId::new(),
                run_id,
                session_id: self.session_id,
                tenant_id: self.tenant_id,
                sandbox: None,
                permission_broker: self.broker.clone(),
                cap_registry: Arc::new(CapabilityRegistry::default()),
                interrupt: InterruptToken::new(),
                parent_run: None,
            },
            permission_context: PermissionContext {
                permission_mode: PermissionMode::Default,
                previous_mode: None,
                session_id: self.session_id,
                tenant_id: self.tenant_id,
                interactivity: InteractivityLevel::NoInteractive,
                timeout_policy: None,
                fallback_policy: FallbackPolicy::DenyAll,
                rule_snapshot: Arc::new(RuleSnapshot {
                    rules: vec![],
                    generation: 0,
                    built_at: harness_contracts::now(),
                }),
                hook_overrides: vec![],
            },
            blob_store: None,
            event_emitter: Arc::new(harness_tool::NoopToolEventEmitter),
        }
    }
}

#[derive(Default)]
struct AllowBroker {
    requests: Mutex<Vec<PermissionRequest>>,
}

impl AllowBroker {
    async fn take_requests(&self) -> Vec<PermissionRequest> {
        std::mem::take(&mut *self.requests.lock().await)
    }
}

#[async_trait]
impl PermissionBroker for AllowBroker {
    async fn decide(&self, request: PermissionRequest, _ctx: PermissionContext) -> Decision {
        self.requests.lock().await.push(request);
        Decision::AllowOnce
    }

    async fn persist(
        &self,
        _decision_id: DecisionId,
        _scope: DecisionScope,
    ) -> Result<(), harness_contracts::PermissionError> {
        Ok(())
    }
}

struct CountingHook {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl HookHandler for CountingHook {
    fn handler_id(&self) -> &'static str {
        "m3-e2e-user-prompt"
    }

    fn interested_events(&self) -> &[HookEventKind] {
        &[HookEventKind::UserPromptSubmit]
    }

    async fn handle(
        &self,
        _event: HookEvent,
        _ctx: HookContext,
    ) -> Result<HookOutcome, harness_contracts::HookError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(HookOutcome::Continue)
    }
}

struct HookView {
    workspace_root: PathBuf,
    redactor: NoopRedactor,
}

impl HookSessionView for HookView {
    fn workspace_root(&self) -> Option<&Path> {
        Some(&self.workspace_root)
    }

    fn recent_messages(&self, _limit: usize) -> Vec<harness_hook::HookMessageView> {
        Vec::new()
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn redacted(&self) -> &dyn harness_contracts::Redactor {
        &self.redactor
    }

    fn current_tool_descriptor(&self) -> Option<harness_hook::ToolDescriptorView> {
        None
    }
}

struct PromptView {
    tenant_id: TenantId,
    session_id: harness_contracts::SessionId,
    descriptors: Vec<ToolDescriptor>,
    messages: Vec<Message>,
}

impl ContextSessionView for PromptView {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    fn session_id(&self) -> Option<harness_contracts::SessionId> {
        Some(self.session_id)
    }

    fn system(&self) -> Option<String> {
        Some("M3 minimal driver".to_owned())
    }

    fn messages(&self) -> Vec<Message> {
        self.messages.clone()
    }

    fn tools_snapshot(&self) -> Vec<ToolDescriptor> {
        self.descriptors.clone()
    }
}

#[derive(Clone)]
struct TestListDirTool {
    descriptor: ToolDescriptor,
}

impl TestListDirTool {
    fn new() -> Self {
        Self {
            descriptor: ToolDescriptor {
                name: "ListDir".to_owned(),
                display_name: "ListDir".to_owned(),
                description: "List directory entries".to_owned(),
                category: "filesystem".to_owned(),
                group: ToolGroup::FileSystem,
                version: "0.0.1".to_owned(),
                input_schema: json!({
                    "type": "object",
                    "required": ["path"],
                    "properties": {
                        "path": { "type": "string" }
                    }
                }),
                output_schema: None,
                dynamic_schema: false,
                properties: ToolProperties {
                    is_concurrency_safe: true,
                    is_read_only: true,
                    is_destructive: false,
                    long_running: None,
                    defer_policy: harness_contracts::DeferPolicy::AlwaysLoad,
                },
                trust_level: TrustLevel::AdminTrusted,
                required_capabilities: vec![],
                budget: default_result_budget(),
                provider_restriction: ProviderRestriction::All,
                origin: ToolOrigin::Builtin,
                search_hint: None,
            },
        }
    }
}

#[async_trait]
impl Tool for TestListDirTool {
    fn descriptor(&self) -> &ToolDescriptor {
        &self.descriptor
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ValidationError> {
        if input.get("path").and_then(Value::as_str).is_none() {
            return Err(ValidationError::from("path is required"));
        }
        Ok(())
    }

    async fn check_permission(
        &self,
        input: &Value,
        _ctx: &ToolContext,
    ) -> harness_permission::PermissionCheck {
        harness_permission::PermissionCheck::AskUser {
            subject: PermissionSubject::ToolInvocation {
                tool: self.descriptor.name.clone(),
                input: input.clone(),
            },
            scope: DecisionScope::ToolName(self.descriptor.name.clone()),
        }
    }

    async fn execute(
        &self,
        input: Value,
        _ctx: ToolContext,
    ) -> Result<ToolStream, harness_contracts::ToolError> {
        let root = input.get("path").and_then(Value::as_str).ok_or_else(|| {
            harness_contracts::ToolError::Validation("path is required".to_owned())
        })?;
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(root)
            .map_err(|error| harness_contracts::ToolError::Message(error.to_string()))?
        {
            let entry =
                entry.map_err(|error| harness_contracts::ToolError::Message(error.to_string()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let meta = entry
                .metadata()
                .map_err(|error| harness_contracts::ToolError::Message(error.to_string()))?;
            entries.push(json!({
                "path": name,
                "kind": if meta.is_dir() { "dir" } else { "file" },
                "size": meta.len(),
            }));
        }
        entries.sort_by(|left, right| {
            left["path"]
                .as_str()
                .unwrap_or_default()
                .cmp(right["path"].as_str().unwrap_or_default())
        });
        Ok(Box::pin(stream::iter([ToolEvent::Final(
            ToolResult::Structured(Value::Array(entries)),
        )])))
    }
}

fn mock_llm_tool_call(_run_id: RunId, workspace_root: &Path) -> ToolCall {
    ToolCall {
        tool_use_id: ToolUseId::new(),
        tool_name: "ListDir".to_owned(),
        input: json!({ "path": workspace_root }),
    }
}

fn message(role: MessageRole, parts: Vec<MessagePart>) -> Message {
    Message {
        id: MessageId::new(),
        role,
        parts,
        created_at: harness_contracts::now(),
    }
}

fn tool_result_summary(result: &ToolResult) -> String {
    match result {
        ToolResult::Structured(Value::Array(entries)) => entries
            .iter()
            .filter_map(|entry| entry.get("path").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        ToolResult::Text(text) => text.clone(),
        _ => String::new(),
    }
}

async fn events_for(
    store: Arc<InMemoryEventStore>,
    tenant_id: TenantId,
    session_id: harness_contracts::SessionId,
) -> Vec<Event> {
    store
        .read_envelopes(tenant_id, session_id, ReplayCursor::FromStart)
        .await
        .unwrap()
        .map(|envelope| envelope.payload)
        .collect()
        .await
}
