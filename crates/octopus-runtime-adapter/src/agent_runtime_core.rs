#![allow(clippy::large_futures)]

use super::*;
use octopus_core::RuntimeTargetPolicyDecision;

#[derive(Debug, Clone)]
pub(crate) struct AgentRuntimeCore;

type SubmitState = (
    RuntimeMessage,
    RuntimeTraceItem,
    Option<RuntimeTraceItem>,
    Option<RuntimeMessage>,
    Option<ApprovalRequestRecord>,
    RuntimeRunSnapshot,
    String,
    String,
);

type ApprovalResolutionState = (
    ApprovalRequestRecord,
    Option<RuntimeTraceItem>,
    Option<RuntimeMessage>,
    RuntimeRunSnapshot,
    String,
    String,
);

type AuthChallengeResolutionState = (
    RuntimeAuthChallengeSummary,
    Option<RuntimeTraceItem>,
    Option<RuntimeMessage>,
    RuntimeRunSnapshot,
    String,
    String,
);

type MemoryProposalResolutionState = (RuntimeMemoryProposal, RuntimeRunSnapshot, String, String);

type TeamSubrunApprovalResolutionState = (
    ApprovalRequestRecord,
    Option<RuntimeTraceItem>,
    Option<RuntimeMessage>,
    RuntimeRunSnapshot,
    String,
    String,
    Vec<RuntimeLoopPlannerEvent>,
    Vec<RuntimeLoopModelIteration>,
    Vec<RuntimeLoopCapabilityEvent>,
    Option<String>,
);

type TeamSubrunAuthChallengeResolutionState = (
    RuntimeAuthChallengeSummary,
    Option<RuntimeTraceItem>,
    Option<RuntimeMessage>,
    RuntimeRunSnapshot,
    String,
    String,
    Vec<RuntimeLoopPlannerEvent>,
    Vec<RuntimeLoopModelIteration>,
    Vec<RuntimeLoopCapabilityEvent>,
    Option<String>,
);

const MAX_RUNTIME_ITERATIONS: u32 = 8;

#[derive(Debug, Clone)]
pub(crate) struct RuntimePendingToolUse {
    pub(crate) tool_use_id: String,
    pub(crate) tool_name: String,
    pub(crate) input: Value,
}

#[derive(Debug, Clone)]
struct RuntimeLoopResult {
    response: ModelExecutionResult,
    serialized_session: Value,
    usage_summary: RuntimeUsageSummary,
    consumed_tokens: Option<u32>,
    current_iteration_index: u32,
    capability_projection: capability_planner_bridge::CapabilityProjection,
    mediation_request: Option<approval_broker::MediationRequest>,
    broker_decision: Option<approval_broker::BrokerDecision>,
    planner_events: Vec<RuntimeLoopPlannerEvent>,
    model_iterations: Vec<RuntimeLoopModelIteration>,
    capability_events: Vec<RuntimeLoopCapabilityEvent>,
}

#[derive(Debug, Clone)]
struct RuntimeLoopFailure {
    error: String,
    serialized_session: Value,
    usage_summary: RuntimeUsageSummary,
    current_iteration_index: u32,
    capability_projection: capability_planner_bridge::CapabilityProjection,
    planner_events: Vec<RuntimeLoopPlannerEvent>,
    model_iterations: Vec<RuntimeLoopModelIteration>,
    capability_events: Vec<RuntimeLoopCapabilityEvent>,
}

#[derive(Debug, Clone)]
enum RuntimeLoopExit {
    Completed(Box<RuntimeLoopResult>),
    Failed(Box<RuntimeLoopFailure>),
}

#[derive(Debug, Clone)]
struct TeamSubrunExecutionOutcome {
    state: team_runtime::PersistedSubrunState,
    planner_events: Vec<RuntimeLoopPlannerEvent>,
    model_iterations: Vec<RuntimeLoopModelIteration>,
    capability_events: Vec<RuntimeLoopCapabilityEvent>,
    runtime_error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum RuntimeLoopModelEventKind {
    Delta,
    ToolUse { tool_use_id: String },
    Usage,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeLoopModelEvent {
    pub(crate) kind: RuntimeLoopModelEventKind,
    pub(crate) message: RuntimeMessage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeLoopPlannerPhase {
    Started,
    Completed,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeLoopPlannerEvent {
    pub(crate) iteration: u32,
    pub(crate) phase: RuntimeLoopPlannerPhase,
    pub(crate) capability_plan_summary: Option<RuntimeCapabilityPlanSummary>,
    pub(crate) provider_state_summary: Option<Vec<RuntimeCapabilityProviderState>>,
    pub(crate) capability_state_ref: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeLoopModelIteration {
    pub(crate) iteration: u32,
    pub(crate) completed: bool,
    pub(crate) events: Vec<RuntimeLoopModelEvent>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeLoopCapabilityEvent {
    pub(crate) iteration: u32,
    pub(crate) tool_use_id: String,
    pub(crate) capability: Option<tools::CapabilitySpec>,
    pub(crate) execution: tools::CapabilityExecutionEvent,
}

fn capability_execution_outcome(
    outcome: &str,
    detail: Option<String>,
    requires_approval: bool,
    requires_auth: bool,
) -> RuntimeCapabilityExecutionOutcome {
    RuntimeCapabilityExecutionOutcome {
        capability_id: None,
        tool_name: None,
        provider_key: None,
        dispatch_kind: None,
        outcome: outcome.into(),
        detail,
        requires_approval,
        requires_auth,
        concurrency_policy: Some("serialized".into()),
    }
}

fn add_token_usage(summary: &mut RuntimeUsageSummary, usage: runtime::TokenUsage) {
    summary.input_tokens = summary.input_tokens.saturating_add(usage.input_tokens);
    summary.output_tokens = summary.output_tokens.saturating_add(usage.output_tokens);
    summary.total_tokens = summary.total_tokens.saturating_add(usage.total_tokens());
}

fn serialize_runtime_message(message: &runtime::ConversationMessage) -> Value {
    json!({
        "role": match message.role {
            runtime::MessageRole::System => "system",
            runtime::MessageRole::User => "user",
            runtime::MessageRole::Assistant => "assistant",
            runtime::MessageRole::Tool => "tool",
        },
        "blocks": message.blocks.iter().map(serialize_runtime_content_block).collect::<Vec<_>>(),
        "usage": message.usage.map(|usage| json!({
            "inputTokens": usage.input_tokens,
            "outputTokens": usage.output_tokens,
            "cacheCreationInputTokens": usage.cache_creation_input_tokens,
            "cacheReadInputTokens": usage.cache_read_input_tokens,
        })),
    })
}

fn serialize_runtime_content_block(block: &runtime::ContentBlock) -> Value {
    match block {
        runtime::ContentBlock::Text { text } => json!({
            "type": "text",
            "text": text,
        }),
        runtime::ContentBlock::ToolUse { id, name, input } => json!({
            "type": "tool_use",
            "id": id,
            "name": name,
            "input": input,
        }),
        runtime::ContentBlock::ToolResult {
            tool_use_id,
            tool_name,
            output,
            is_error,
        } => json!({
            "type": "tool_result",
            "toolUseId": tool_use_id,
            "toolName": tool_name,
            "output": output,
            "isError": is_error,
        }),
    }
}

fn deserialize_runtime_message(value: &Value) -> Result<runtime::ConversationMessage, AppError> {
    let role =
        match value.get("role").and_then(Value::as_str).ok_or_else(|| {
            AppError::runtime("serialized runtime session message is missing role")
        })? {
            "system" => runtime::MessageRole::System,
            "user" => runtime::MessageRole::User,
            "assistant" => runtime::MessageRole::Assistant,
            "tool" => runtime::MessageRole::Tool,
            other => {
                return Err(AppError::runtime(format!(
                    "serialized runtime session has unsupported role `{other}`"
                )))
            }
        };
    let blocks = value
        .get("blocks")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::runtime("serialized runtime session message is missing blocks"))?
        .iter()
        .map(deserialize_runtime_content_block)
        .collect::<Result<Vec<_>, _>>()?;
    let usage = value
        .get("usage")
        .and_then(|raw| (!raw.is_null()).then_some(raw))
        .map(deserialize_token_usage)
        .transpose()?;
    Ok(runtime::ConversationMessage {
        role,
        blocks,
        usage,
    })
}

fn deserialize_runtime_content_block(value: &Value) -> Result<runtime::ContentBlock, AppError> {
    match value
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::runtime("serialized runtime content block is missing type"))?
    {
        "text" => Ok(runtime::ContentBlock::Text {
            text: value
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        }),
        "tool_use" => Ok(runtime::ContentBlock::ToolUse {
            id: value
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            name: value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            input: value
                .get("input")
                .and_then(Value::as_str)
                .unwrap_or("{}")
                .to_string(),
        }),
        "tool_result" => Ok(runtime::ContentBlock::ToolResult {
            tool_use_id: value
                .get("toolUseId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            tool_name: value
                .get("toolName")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            output: value
                .get("output")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            is_error: value
                .get("isError")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        other => Err(AppError::runtime(format!(
            "serialized runtime session has unsupported content block `{other}`"
        ))),
    }
}

fn deserialize_token_usage(value: &Value) -> Result<runtime::TokenUsage, AppError> {
    fn token_value(parent: &Value, key: &str) -> Result<u32, AppError> {
        parent
            .get(key)
            .and_then(Value::as_u64)
            .map(|value| value as u32)
            .ok_or_else(|| AppError::runtime(format!("serialized token usage is missing `{key}`")))
    }

    Ok(runtime::TokenUsage {
        input_tokens: token_value(value, "inputTokens")?,
        output_tokens: token_value(value, "outputTokens")?,
        cache_creation_input_tokens: token_value(value, "cacheCreationInputTokens")?,
        cache_read_input_tokens: token_value(value, "cacheReadInputTokens")?,
    })
}

fn initial_runtime_session(content: &str) -> Result<runtime::Session, AppError> {
    let mut session = runtime::Session::new();
    session
        .push_user_text(content)
        .map_err(|error| AppError::runtime(error.to_string()))?;
    Ok(session)
}

fn restore_runtime_session(
    serialized_session: &Value,
    content: &str,
) -> Result<runtime::Session, AppError> {
    let Some(messages) = serialized_session
        .get("session")
        .and_then(|session| session.get("messages"))
        .and_then(Value::as_array)
    else {
        return initial_runtime_session(content);
    };

    let mut session = runtime::Session::new();
    session.messages = messages
        .iter()
        .map(deserialize_runtime_message)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(session)
}

fn trace_context_from_serialized_session(serialized_session: &Value) -> RuntimeTraceContext {
    serialized_session
        .get("traceContext")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

fn requested_permission_mode_from_serialized_session(
    serialized_session: &Value,
    fallback: &str,
) -> String {
    serialized_session
        .get("requestedPermissionMode")
        .and_then(Value::as_str)
        .and_then(octopus_core::normalize_runtime_permission_mode_label)
        .unwrap_or(fallback)
        .to_string()
}

#[derive(Debug, Clone)]
struct ConsumedAssistantTurn {
    assistant_message: runtime::ConversationMessage,
    usage: Option<runtime::TokenUsage>,
    model_events: Vec<RuntimeLoopModelEvent>,
}

#[derive(Debug, Clone)]
struct InterruptedAssistantTurn {
    detail: String,
    partial_content: String,
    usage: Option<runtime::TokenUsage>,
    model_events: Vec<RuntimeLoopModelEvent>,
}

fn serialize_runtime_usage_json(usage: runtime::TokenUsage) -> Value {
    json!({
        "inputTokens": usage.input_tokens,
        "outputTokens": usage.output_tokens,
        "cacheCreationInputTokens": usage.cache_creation_input_tokens,
        "cacheReadInputTokens": usage.cache_read_input_tokens,
        "totalTokens": usage.total_tokens(),
    })
}

fn assistant_text_snapshot(blocks: &[runtime::ContentBlock], pending_text: &str) -> String {
    let mut content = blocks
        .iter()
        .filter_map(|block| match block {
            runtime::ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    content.push_str(pending_text);
    content
}

fn model_event_message_snapshot(
    session_id: &str,
    conversation_id: &str,
    resolved_target: &ResolvedExecutionTarget,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    content: String,
    usage: Option<runtime::TokenUsage>,
) -> RuntimeMessage {
    RuntimeMessage {
        id: format!("msg-{}", Uuid::new_v4()),
        session_id: session_id.to_string(),
        conversation_id: conversation_id.to_string(),
        sender_type: "assistant".into(),
        sender_label: actor_manifest.label().to_string(),
        content,
        timestamp: timestamp_now(),
        configured_model_id: Some(resolved_target.configured_model_id.clone()),
        configured_model_name: Some(resolved_target.configured_model_name.clone()),
        model_id: Some(resolved_target.registry_model_id.clone()),
        status: "streaming".into(),
        requested_actor_kind: Some(actor_manifest.actor_kind_label().into()),
        requested_actor_id: Some(actor_manifest.actor_ref().to_string()),
        resolved_actor_kind: Some(actor_manifest.actor_kind_label().into()),
        resolved_actor_id: Some(actor_manifest.actor_ref().to_string()),
        resolved_actor_label: Some(actor_manifest.label().to_string()),
        used_default_actor: Some(false),
        resource_ids: Some(Vec::new()),
        attachments: Some(Vec::new()),
        artifacts: None,
        deliverable_refs: None,
        usage: usage.map(serialize_runtime_usage_json),
        tool_calls: None,
        process_entries: None,
    }
}

fn consume_assistant_turn_events(
    session_id: &str,
    conversation_id: &str,
    resolved_target: &ResolvedExecutionTarget,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    events: Vec<runtime::AssistantEvent>,
) -> Result<ConsumedAssistantTurn, InterruptedAssistantTurn> {
    let mut text = String::new();
    let mut blocks = Vec::new();
    let mut finished = false;
    let mut usage = None;
    let mut model_events = Vec::new();

    for event in events {
        match event {
            runtime::AssistantEvent::TextDelta(delta) => {
                text.push_str(&delta);
                model_events.push(RuntimeLoopModelEvent {
                    kind: RuntimeLoopModelEventKind::Delta,
                    message: model_event_message_snapshot(
                        session_id,
                        conversation_id,
                        resolved_target,
                        actor_manifest,
                        assistant_text_snapshot(&blocks, &text),
                        usage,
                    ),
                });
            }
            runtime::AssistantEvent::ToolUse { id, name, input } => {
                if !text.is_empty() {
                    blocks.push(runtime::ContentBlock::Text {
                        text: std::mem::take(&mut text),
                    });
                }
                blocks.push(runtime::ContentBlock::ToolUse { id, name, input });
                let runtime::ContentBlock::ToolUse { id, .. } =
                    blocks.last().expect("tool use block should be present")
                else {
                    unreachable!("last content block should be a tool use");
                };
                model_events.push(RuntimeLoopModelEvent {
                    kind: RuntimeLoopModelEventKind::ToolUse {
                        tool_use_id: id.clone(),
                    },
                    message: model_event_message_snapshot(
                        session_id,
                        conversation_id,
                        resolved_target,
                        actor_manifest,
                        assistant_text_snapshot(&blocks, &text),
                        usage,
                    ),
                });
            }
            runtime::AssistantEvent::Usage(value) => {
                usage = Some(value);
                model_events.push(RuntimeLoopModelEvent {
                    kind: RuntimeLoopModelEventKind::Usage,
                    message: model_event_message_snapshot(
                        session_id,
                        conversation_id,
                        resolved_target,
                        actor_manifest,
                        assistant_text_snapshot(&blocks, &text),
                        usage,
                    ),
                });
            }
            runtime::AssistantEvent::PromptCache(_) => {}
            runtime::AssistantEvent::MessageStop => finished = true,
        }
    }

    if !text.is_empty() {
        blocks.push(runtime::ContentBlock::Text { text });
    }

    if !finished {
        return Err(InterruptedAssistantTurn {
            detail: "assistant stream ended without a message stop event".into(),
            partial_content: assistant_text_snapshot(&blocks, ""),
            usage,
            model_events,
        });
    }
    if blocks.is_empty() {
        return Err(InterruptedAssistantTurn {
            detail: "assistant stream produced no content".into(),
            partial_content: String::new(),
            usage,
            model_events,
        });
    }

    Ok(ConsumedAssistantTurn {
        assistant_message: runtime::ConversationMessage::assistant_with_usage(blocks, usage),
        usage,
        model_events,
    })
}

fn assistant_message_content(message: &runtime::ConversationMessage) -> String {
    message
        .blocks
        .iter()
        .filter_map(|block| match block {
            runtime::ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn collect_tool_uses(message: &runtime::ConversationMessage) -> Vec<RuntimePendingToolUse> {
    message
        .blocks
        .iter()
        .filter_map(|block| match block {
            runtime::ContentBlock::ToolUse { id, name, input } => Some(RuntimePendingToolUse {
                tool_use_id: id.clone(),
                tool_name: name.clone(),
                input: serde_json::from_str(input).unwrap_or_else(|_| json!({ "raw": input })),
            }),
            _ => None,
        })
        .collect()
}

fn serialize_pending_tool_use(tool_use: &RuntimePendingToolUse) -> Value {
    json!({
        "toolUseId": tool_use.tool_use_id,
        "toolName": tool_use.tool_name,
        "input": tool_use.input,
    })
}

fn deserialize_pending_tool_use(value: &Value) -> Result<RuntimePendingToolUse, AppError> {
    Ok(RuntimePendingToolUse {
        tool_use_id: value
            .get("toolUseId")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::runtime("pending tool use is missing toolUseId"))?
            .to_string(),
        tool_name: value
            .get("toolName")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::runtime("pending tool use is missing toolName"))?
            .to_string(),
        input: value
            .get("input")
            .cloned()
            .ok_or_else(|| AppError::runtime("pending tool use is missing input"))?,
    })
}

fn pending_tool_uses_from_serialized_session(
    serialized_session: &Value,
) -> Result<Vec<RuntimePendingToolUse>, AppError> {
    serialized_session
        .get("pendingToolUses")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(deserialize_pending_tool_use)
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn latest_runtime_response(
    session: &runtime::Session,
    total_tokens: Option<u32>,
) -> Result<ModelExecutionResult, AppError> {
    let assistant_message = session
        .messages
        .iter()
        .rev()
        .find(|message| message.role == runtime::MessageRole::Assistant)
        .ok_or_else(|| AppError::runtime("runtime session does not have an assistant message"))?;
    Ok(ModelExecutionResult {
        content: assistant_message_content(assistant_message),
        request_id: None,
        total_tokens,
        deliverables: Vec::new(),
    })
}

fn infer_deliverable_content_type(output: &ModelExecutionDeliverable) -> Option<String> {
    output
        .content_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            if output.text_content.is_some() {
                Some(
                    match output.preview_kind.as_str() {
                        "markdown" => "text/markdown",
                        "json" => "application/json",
                        _ => "text/plain",
                    }
                    .to_string(),
                )
            } else if output.data_base64.is_some() {
                Some("application/octet-stream".to_string())
            } else {
                None
            }
        })
}

fn resolve_deliverable_title(output: &ModelExecutionDeliverable, fallback: &str) -> String {
    if let Some(title) = output
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return title.to_string();
    }

    if let Some(text_content) = output.text_content.as_deref() {
        if let Some(title) = text_content
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(|line| line.trim_start_matches('#').trim())
            .filter(|line| !line.is_empty())
        {
            return title.chars().take(80).collect();
        }
    }

    fallback.to_string()
}

fn build_pending_runtime_deliverables(
    workspace_id: &str,
    project_id: &str,
    conversation_id: &str,
    session_id: &str,
    run_id: &str,
    fallback_title: &str,
    updated_at: u64,
    outputs: &[ModelExecutionDeliverable],
    source_message_id: Option<&str>,
) -> Vec<PendingRuntimeDeliverable> {
    if project_id.trim().is_empty() {
        return Vec::new();
    }

    outputs
        .iter()
        .map(|output| {
            let deliverable_id = format!("artifact-{}", Uuid::new_v4());
            let preview_kind = output
                .preview_kind
                .trim()
                .to_string()
                .chars()
                .collect::<String>();
            let preview_kind = if preview_kind.is_empty() {
                "markdown".to_string()
            } else {
                preview_kind
            };
            let title = resolve_deliverable_title(output, fallback_title);
            let content_type = infer_deliverable_content_type(output);
            let latest_version_ref = ArtifactVersionReference {
                artifact_id: deliverable_id.clone(),
                version: 1,
                title: title.clone(),
                preview_kind: preview_kind.clone(),
                updated_at,
                content_type: content_type.clone(),
            };
            let detail = DeliverableDetail {
                id: deliverable_id.clone(),
                workspace_id: workspace_id.to_string(),
                project_id: project_id.to_string(),
                conversation_id: conversation_id.to_string(),
                session_id: session_id.to_string(),
                run_id: run_id.to_string(),
                source_message_id: source_message_id.map(str::to_string),
                parent_artifact_id: None,
                title: title.clone(),
                status: "ready".into(),
                preview_kind: preview_kind.clone(),
                latest_version: 1,
                latest_version_ref,
                promotion_state: "not-promoted".into(),
                promotion_knowledge_id: None,
                updated_at,
                storage_path: None,
                content_hash: None,
                byte_size: None,
                content_type: content_type.clone(),
            };
            let content = DeliverableVersionContent {
                artifact_id: deliverable_id,
                version: 1,
                preview_kind,
                editable: true,
                file_name: output.file_name.clone(),
                content_type,
                text_content: output.text_content.clone(),
                data_base64: output.data_base64.clone(),
                byte_size: None,
            };
            PendingRuntimeDeliverable {
                detail,
                content,
                source_message_id: source_message_id.map(str::to_string),
                parent_version: None,
            }
        })
        .collect()
}

fn register_pending_runtime_deliverables(
    aggregate: &mut RuntimeAggregate,
    deliverables: Vec<PendingRuntimeDeliverable>,
) -> Vec<ArtifactVersionReference> {
    let refs = deliverables
        .iter()
        .map(|deliverable| deliverable.detail.latest_version_ref.clone())
        .collect::<Vec<_>>();

    for deliverable in deliverables {
        aggregate
            .metadata
            .pending_deliverables
            .insert(deliverable.detail.id.clone(), deliverable);
    }

    refs
}

fn serialized_runtime_session(
    content: &str,
    trace_context: &RuntimeTraceContext,
    requested_permission_mode: &str,
) -> Value {
    let session = initial_runtime_session(content).unwrap_or_default();
    serialized_runtime_session_with_state(
        content,
        trace_context,
        requested_permission_mode,
        &session,
    )
}

fn serialized_runtime_session_with_state(
    content: &str,
    trace_context: &RuntimeTraceContext,
    requested_permission_mode: &str,
    session: &runtime::Session,
) -> Value {
    json!({
        "content": content,
        "pendingContent": content,
        "requestedPermissionMode": requested_permission_mode,
        "traceContext": trace_context,
        "pendingToolUses": [],
        "session": {
            "messages": session
                .messages
                .iter()
                .map(serialize_runtime_message)
                .collect::<Vec<_>>(),
        }
    })
}

fn serialized_runtime_session_with_pending_tool_uses(
    content: &str,
    trace_context: &RuntimeTraceContext,
    requested_permission_mode: &str,
    session: &runtime::Session,
    pending_tool_uses: &[RuntimePendingToolUse],
) -> Value {
    let mut serialized = serialized_runtime_session_with_state(
        content,
        trace_context,
        requested_permission_mode,
        session,
    );
    if let Some(parent) = serialized.as_object_mut() {
        parent.insert(
            "pendingToolUses".to_string(),
            Value::Array(
                pending_tool_uses
                    .iter()
                    .map(serialize_pending_tool_use)
                    .collect(),
            ),
        );
    }
    serialized
}

fn attach_partial_output_metadata(
    serialized_session: &mut Value,
    content: String,
    usage: Option<runtime::TokenUsage>,
    error: &str,
) {
    if let Some(parent) = serialized_session.as_object_mut() {
        parent.insert(
            "partialOutput".to_string(),
            json!({
                "status": "interrupted",
                "content": content,
                "usage": usage.map(serialize_runtime_usage_json),
                "error": error,
            }),
        );
    }
}

fn model_execution_target_ref(run_id: &str, configured_model_id: &str) -> String {
    format!("model-execution:{run_id}:{configured_model_id}")
}

fn team_spawn_target_ref(run_id: &str, actor_ref: &str) -> String {
    format!("team-spawn:{run_id}:{actor_ref}")
}

fn workflow_continuation_target_ref(run_id: &str) -> String {
    let workflow_run_id = format!("workflow-{run_id}");
    format!("workflow-continuation:{run_id}:{workflow_run_id}")
}

fn provider_auth_target_ref(provider_key: &str) -> String {
    provider_key.to_string()
}

fn approval_replays_runtime_loop(target_kind: Option<&str>) -> bool {
    matches!(target_kind, Some("model-execution" | "capability-call"))
}

fn resumable_approval_target(target_kind: Option<&str>) -> bool {
    approval_replays_runtime_loop(target_kind)
        || matches!(target_kind, Some("team-spawn" | "workflow-continuation"))
}

fn resumable_auth_target(target_kind: &str) -> bool {
    matches!(target_kind, "provider-auth" | "capability-call")
}

fn approval_blocks_team_projection(approval: Option<&ApprovalRequestRecord>) -> bool {
    approval.is_some_and(|approval| {
        approval.status == "pending" && approval.target_kind.as_deref() == Some("team-spawn")
    })
}

fn resolved_approval_status(run: &RuntimeRunSnapshot, approval_id: &str) -> Option<String> {
    run.last_mediation_outcome
        .as_ref()
        .filter(|outcome| {
            outcome.mediation_kind == "approval"
                && outcome.mediation_id.as_deref() == Some(approval_id)
                && outcome.outcome != "pending"
        })
        .map(|outcome| outcome.outcome.clone())
        .or_else(|| {
            run.checkpoint
                .last_mediation_outcome
                .as_ref()
                .filter(|outcome| {
                    outcome.mediation_kind == "approval"
                        && outcome.mediation_id.as_deref() == Some(approval_id)
                        && outcome.outcome != "pending"
                })
                .map(|outcome| outcome.outcome.clone())
        })
        .or_else(|| {
            run.approval_target
                .as_ref()
                .filter(|approval| approval.id == approval_id && approval.status != "pending")
                .map(|approval| approval.status.clone())
        })
        .or_else(|| {
            run.checkpoint
                .pending_approval
                .as_ref()
                .filter(|approval| approval.id == approval_id && approval.status != "pending")
                .map(|approval| approval.status.clone())
        })
}

fn runtime_approval_lookup_error(aggregate: &RuntimeAggregate, approval_id: &str) -> AppError {
    resolved_approval_status(&aggregate.detail.run, approval_id)
        .or_else(|| {
            aggregate
                .metadata
                .subrun_states
                .values()
                .find_map(|state| resolved_approval_status(&state.run, approval_id))
        })
        .map(|status| {
            AppError::conflict(format!(
                "runtime approval `{approval_id}` is already {status}"
            ))
        })
        .unwrap_or_else(|| AppError::not_found("runtime approval"))
}

fn team_spawn_policy_decision(
    run_context: &run_context::RunContext,
) -> Option<RuntimeTargetPolicyDecision> {
    matches!(
        run_context.actor_manifest,
        actor_manifest::CompiledActorManifest::Team(_)
    )
    .then(|| {
        run_context
            .session_policy
            .target_decisions
            .get(&format!(
                "team-spawn:{}",
                run_context.actor_manifest.actor_ref()
            ))
            .cloned()
    })
    .flatten()
}

fn workflow_continuation_policy_decision(
    actor_manifest: &actor_manifest::CompiledActorManifest,
    session_policy: &session_policy::CompiledSessionPolicy,
) -> Option<RuntimeTargetPolicyDecision> {
    matches!(
        actor_manifest,
        actor_manifest::CompiledActorManifest::Team(_)
    )
    .then(|| {
        session_policy
            .target_decisions
            .get(&format!(
                "workflow-continuation:{}",
                actor_manifest.actor_ref()
            ))
            .cloned()
    })
    .flatten()
}

fn team_spawn_mediation_request(
    run_context: &run_context::RunContext,
    checkpoint_ref: Option<String>,
    created_at: u64,
) -> Option<approval_broker::MediationRequest> {
    let actor_manifest::CompiledActorManifest::Team(team_manifest) = &run_context.actor_manifest
    else {
        return None;
    };
    let policy_decision = team_spawn_policy_decision(run_context)?;
    if !policy_decision.requires_approval {
        return None;
    }
    let worker_total = worker_runtime::worker_actor_refs(team_manifest).len();
    if worker_total == 0 {
        return None;
    }
    let worker_refs = worker_runtime::worker_actor_refs(team_manifest);
    let target_ref =
        team_spawn_target_ref(&run_context.run_id, run_context.actor_manifest.actor_ref());
    let detail = policy_decision.reason.clone().unwrap_or_else(|| {
        format!(
            "Approve spawning {worker_total} worker subruns before team execution can continue."
        )
    });
    Some(approval_broker::MediationRequest {
        session_id: run_context.session_id.clone(),
        conversation_id: run_context.conversation_id.clone(),
        run_id: run_context.run_id.clone(),
        tool_name: run_context.actor_manifest.label().to_string(),
        summary: "Team worker dispatch requires approval".into(),
        detail: detail.clone(),
        mediation_kind: "approval".into(),
        approval_layer: "team-spawn".into(),
        target_kind: "team-spawn".into(),
        target_ref,
        capability_id: Some(run_context.actor_manifest.actor_ref().to_string()),
        dispatch_kind: "team_spawn".into(),
        provider_key: None,
        concurrency_policy: "serialized".into(),
        input: json!({
            "actorRef": run_context.actor_manifest.actor_ref(),
            "workerRefs": worker_refs,
        }),
        required_permission: policy_decision.required_permission.clone(),
        escalation_reason: Some(detail),
        requires_approval: true,
        requires_auth: false,
        created_at,
        risk_level: "high".into(),
        checkpoint_ref,
        policy_action: Some(policy_decision.action.clone()),
        pending_state: None,
    })
}

fn workflow_continuation_mediation_request(
    session_id: &str,
    conversation_id: &str,
    run_id: &str,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    session_policy: &session_policy::CompiledSessionPolicy,
    checkpoint_ref: Option<String>,
    created_at: u64,
) -> Option<approval_broker::MediationRequest> {
    let actor_manifest::CompiledActorManifest::Team(team_manifest) = actor_manifest else {
        return None;
    };
    let policy_decision = workflow_continuation_policy_decision(actor_manifest, session_policy)?;
    if !policy_decision.requires_approval {
        return None;
    }
    let worker_total = worker_runtime::worker_actor_refs(team_manifest).len();
    if worker_total == 0 {
        return None;
    }
    let workflow_run_id = format!("workflow-{run_id}");
    let target_ref = workflow_continuation_target_ref(run_id);
    let detail = policy_decision.reason.clone().unwrap_or_else(|| {
        format!(
            "Approve continuing workflow `workflow-{run_id}` before team coordination can proceed."
        )
    });
    Some(approval_broker::MediationRequest {
        session_id: session_id.to_string(),
        conversation_id: conversation_id.to_string(),
        run_id: run_id.to_string(),
        tool_name: actor_manifest.label().to_string(),
        summary: "Workflow continuation requires approval".into(),
        detail: detail.clone(),
        mediation_kind: "approval".into(),
        approval_layer: "workflow-continuation".into(),
        target_kind: "workflow-continuation".into(),
        target_ref,
        capability_id: Some(actor_manifest.actor_ref().to_string()),
        dispatch_kind: "workflow_continuation".into(),
        provider_key: None,
        concurrency_policy: "serialized".into(),
        input: json!({
            "actorRef": actor_manifest.actor_ref(),
            "workflowRunId": workflow_run_id,
            "workerCount": worker_total,
        }),
        required_permission: policy_decision.required_permission.clone(),
        escalation_reason: Some(detail),
        requires_approval: true,
        requires_auth: false,
        created_at,
        risk_level: "high".into(),
        checkpoint_ref,
        policy_action: Some(policy_decision.action.clone()),
        pending_state: None,
    })
}

fn next_runtime_iteration_index_from_value(current_iteration_index: u32) -> Result<u32, AppError> {
    let next = current_iteration_index.saturating_add(1);
    if next > MAX_RUNTIME_ITERATIONS {
        return Err(AppError::runtime(format!(
            "runtime execution exceeded the maximum number of iterations ({MAX_RUNTIME_ITERATIONS})"
        )));
    }
    Ok(next)
}

fn planner_started_event(iteration: u32) -> RuntimeLoopPlannerEvent {
    RuntimeLoopPlannerEvent {
        iteration,
        phase: RuntimeLoopPlannerPhase::Started,
        capability_plan_summary: None,
        provider_state_summary: None,
        capability_state_ref: None,
    }
}

fn planner_completed_event(
    iteration: u32,
    projection: &capability_planner_bridge::CapabilityProjection,
) -> RuntimeLoopPlannerEvent {
    RuntimeLoopPlannerEvent {
        iteration,
        phase: RuntimeLoopPlannerPhase::Completed,
        capability_plan_summary: Some(projection.plan_summary.clone()),
        provider_state_summary: Some(projection.provider_state_summary.clone()),
        capability_state_ref: Some(projection.capability_state_ref.clone()),
    }
}

async fn execute_runtime_turn_loop(
    adapter: &RuntimeAdapter,
    session_id: &str,
    conversation_id: &str,
    run_id: &str,
    resolved_target: &ResolvedExecutionTarget,
    configured_model: &ConfiguredModelRecord,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    session_policy: &session_policy::CompiledSessionPolicy,
    requested_permission_mode: &str,
    capability_state_ref: &str,
    content: &str,
    trace_context: &RuntimeTraceContext,
    mut session: runtime::Session,
    pending_tool_uses: Vec<RuntimePendingToolUse>,
    starting_iteration_index: u32,
    mut usage_summary: RuntimeUsageSummary,
) -> Result<RuntimeLoopExit, AppError> {
    let starting_total_tokens = usage_summary.total_tokens;
    let mut current_iteration_index = starting_iteration_index;
    let mut usage_complete = true;
    let mut pending_tool_uses = std::collections::VecDeque::from(pending_tool_uses);
    let mut planner_events = Vec::new();
    let mut model_iterations = Vec::new();
    let mut capability_events = Vec::new();

    loop {
        let planned_iteration = next_runtime_iteration_index_from_value(current_iteration_index)?;
        planner_events.push(planner_started_event(planned_iteration));
        let capability_store = adapter.load_capability_store(Some(capability_state_ref))?;
        let prepared = adapter
            .prepare_capability_runtime_async(
                actor_manifest,
                session_policy,
                &session_policy.config_snapshot_id,
                capability_state_ref.to_string(),
                &capability_store,
            )
            .await?;
        planner_events.push(planner_completed_event(
            planned_iteration,
            &prepared.projection,
        ));
        let capability_projection = prepared.projection.clone();
        let capability_runtime = prepared.capability_runtime.clone();
        let managed_mcp_runtime = prepared.managed_mcp_runtime.clone();
        let visible_capabilities = prepared.visible_capabilities.clone();
        let planned_tool_names = prepared.planned_tool_names.clone();
        let mut processed_pending_tool_use = false;

        let pending_mcp_servers = managed_mcp_runtime.as_ref().and_then(|runtime| {
            runtime
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .pending_servers()
        });
        let mcp_degraded = managed_mcp_runtime.as_ref().and_then(|runtime| {
            runtime
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .degraded_report()
        });

        while let Some(tool_use) = pending_tool_uses.pop_front() {
            processed_pending_tool_use = true;
            let execution = capability_executor_bridge::execute_pending_tool_use(
                adapter,
                &capability_runtime,
                managed_mcp_runtime.as_ref(),
                &visible_capabilities,
                &planned_tool_names,
                &capability_store,
                session_id,
                conversation_id,
                run_id,
                requested_permission_mode,
                &tool_use,
                pending_mcp_servers.clone(),
                mcp_degraded.clone(),
            )?;
            capability_events.extend(execution.execution_events.into_iter().map(|event| {
                RuntimeLoopCapabilityEvent {
                    iteration: current_iteration_index,
                    tool_use_id: tool_use.tool_use_id.clone(),
                    capability: execution.capability.clone(),
                    execution: event,
                }
            }));
            match execution.outcome {
                runtime::ToolExecutionOutcome::Allow { output } => {
                    let result_message = runtime::ConversationMessage::tool_result(
                        tool_use.tool_use_id,
                        tool_use.tool_name,
                        output,
                        false,
                    );
                    session
                        .push_message(result_message)
                        .map_err(|error| AppError::runtime(error.to_string()))?;
                }
                runtime::ToolExecutionOutcome::RequireApproval { .. }
                | runtime::ToolExecutionOutcome::RequireAuth { .. } => {
                    let mediation_request = execution.mediation_request.ok_or_else(|| {
                        AppError::runtime(
                            "runtime capability mediation did not capture a blocked request",
                        )
                    })?;
                    let broker_decision = approval_broker::mediate(&mediation_request);
                    let updated_projection = adapter
                        .project_capability_state_async(
                            actor_manifest,
                            session_policy,
                            &session_policy.config_snapshot_id,
                            capability_state_ref.to_string(),
                            &capability_store,
                        )
                        .await?;
                    let mut remaining_tool_uses = vec![tool_use];
                    remaining_tool_uses.extend(pending_tool_uses.into_iter());
                    adapter.persist_capability_store(capability_state_ref, &capability_store)?;
                    let segment_total_tokens = usage_complete.then_some(
                        usage_summary
                            .total_tokens
                            .saturating_sub(starting_total_tokens),
                    );
                    let response = latest_runtime_response(&session, segment_total_tokens)?;
                    let consumed_tokens =
                        adapter.resolve_consumed_tokens(configured_model, &response)?;
                    return Ok(RuntimeLoopExit::Completed(Box::new(RuntimeLoopResult {
                        response,
                        serialized_session: serialized_runtime_session_with_pending_tool_uses(
                            content,
                            trace_context,
                            requested_permission_mode,
                            &session,
                            &remaining_tool_uses,
                        ),
                        usage_summary,
                        consumed_tokens,
                        current_iteration_index,
                        capability_projection: updated_projection,
                        mediation_request: Some(mediation_request),
                        broker_decision: Some(broker_decision),
                        planner_events,
                        model_iterations,
                        capability_events,
                    })));
                }
                other => {
                    let result_message = runtime::ConversationMessage::tool_result(
                        tool_use.tool_use_id,
                        tool_use.tool_name.clone(),
                        other.message_for_tool(&tool_use.tool_name),
                        true,
                    );
                    session
                        .push_message(result_message)
                        .map_err(|error| AppError::runtime(error.to_string()))?;
                }
            }
        }
        adapter.persist_capability_store(capability_state_ref, &capability_store)?;

        let (capability_projection, visible_capabilities) = if processed_pending_tool_use {
            planner_events.push(planner_started_event(planned_iteration));
            let prepared = adapter
                .prepare_capability_runtime_async(
                    actor_manifest,
                    session_policy,
                    &session_policy.config_snapshot_id,
                    capability_state_ref.to_string(),
                    &capability_store,
                )
                .await?;
            planner_events.push(planner_completed_event(
                planned_iteration,
                &prepared.projection,
            ));
            (prepared.projection, prepared.visible_capabilities)
        } else {
            (capability_projection, visible_capabilities)
        };

        let system_prompt = actor_manifest.system_prompt();
        let request = RuntimeConversationRequest {
            system_prompt: (!system_prompt.trim().is_empty())
                .then_some(system_prompt)
                .into_iter()
                .collect(),
            messages: session.messages.clone(),
            tools: visible_capabilities
                .iter()
                .map(tools::CapabilitySpec::to_tool_definition)
                .collect(),
        };
        current_iteration_index = next_runtime_iteration_index_from_value(current_iteration_index)?;
        let conversation_execution = adapter
            .execute_resolved_conversation(resolved_target, &request)
            .await?;
        let consumed_turn = consume_assistant_turn_events(
            session_id,
            conversation_id,
            resolved_target,
            actor_manifest,
            conversation_execution.events,
        );
        let consumed_turn = match consumed_turn {
            Ok(consumed_turn) => consumed_turn,
            Err(interrupted_turn) => {
                if let Some(usage) = interrupted_turn.usage {
                    add_token_usage(&mut usage_summary, usage);
                }
                model_iterations.push(RuntimeLoopModelIteration {
                    iteration: current_iteration_index,
                    completed: false,
                    events: interrupted_turn.model_events,
                });
                let mut serialized_session = serialized_runtime_session_with_state(
                    content,
                    trace_context,
                    requested_permission_mode,
                    &session,
                );
                attach_partial_output_metadata(
                    &mut serialized_session,
                    interrupted_turn.partial_content,
                    interrupted_turn.usage,
                    &interrupted_turn.detail,
                );
                return Ok(RuntimeLoopExit::Failed(Box::new(RuntimeLoopFailure {
                    error: interrupted_turn.detail,
                    serialized_session,
                    usage_summary,
                    current_iteration_index,
                    capability_projection,
                    planner_events,
                    model_iterations,
                    capability_events,
                })));
            }
        };
        let assistant_message = consumed_turn.assistant_message;
        let usage = consumed_turn.usage;
        model_iterations.push(RuntimeLoopModelIteration {
            iteration: current_iteration_index,
            completed: true,
            events: consumed_turn.model_events,
        });
        usage_complete &= usage.is_some();
        if let Some(usage) = usage {
            add_token_usage(&mut usage_summary, usage);
        }
        let segment_total_tokens = usage_complete.then_some(
            usage_summary
                .total_tokens
                .saturating_sub(starting_total_tokens),
        );
        let response_content = assistant_message_content(&assistant_message);
        let response = ModelExecutionResult {
            content: response_content,
            request_id: None,
            total_tokens: segment_total_tokens,
            deliverables: conversation_execution.deliverables,
        };
        let tool_uses = collect_tool_uses(&assistant_message);
        session
            .push_message(assistant_message)
            .map_err(|error| AppError::runtime(error.to_string()))?;

        if tool_uses.is_empty() {
            let consumed_tokens = adapter.resolve_consumed_tokens(configured_model, &response)?;
            return Ok(RuntimeLoopExit::Completed(Box::new(RuntimeLoopResult {
                response,
                serialized_session: serialized_runtime_session_with_state(
                    content,
                    trace_context,
                    requested_permission_mode,
                    &session,
                ),
                usage_summary,
                consumed_tokens,
                current_iteration_index,
                capability_projection,
                mediation_request: None,
                broker_decision: None,
                planner_events,
                model_iterations,
                capability_events,
            })));
        }
        pending_tool_uses.extend(tool_uses);
    }
}

async fn execute_runtime_turn_loop_with_budget_reservation(
    adapter: &RuntimeAdapter,
    session_id: &str,
    conversation_id: &str,
    run_id: &str,
    resolved_target: &ResolvedExecutionTarget,
    configured_model: &ConfiguredModelRecord,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    session_policy: &session_policy::CompiledSessionPolicy,
    requested_permission_mode: &str,
    capability_state_ref: &str,
    content: &str,
    trace_context: &RuntimeTraceContext,
    session: runtime::Session,
    pending_tool_uses: Vec<RuntimePendingToolUse>,
    starting_iteration_index: u32,
    usage_summary: RuntimeUsageSummary,
) -> Result<RuntimeLoopExit, AppError> {
    adapter.reserve_configured_model_budget(
        run_id,
        configured_model,
        crate::model_budget::BUDGET_TRAFFIC_CLASS_INTERACTIVE_TURN,
        timestamp_now(),
    )?;
    let loop_exit = execute_runtime_turn_loop(
        adapter,
        session_id,
        conversation_id,
        run_id,
        resolved_target,
        configured_model,
        actor_manifest,
        session_policy,
        requested_permission_mode,
        capability_state_ref,
        content,
        trace_context,
        session,
        pending_tool_uses,
        starting_iteration_index,
        usage_summary,
    )
    .await;

    match loop_exit {
        Ok(RuntimeLoopExit::Completed(result)) => {
            adapter.settle_configured_model_budget_reservation(
                run_id,
                &configured_model.configured_model_id,
                result.consumed_tokens.unwrap_or(0),
                timestamp_now(),
            )?;
            Ok(RuntimeLoopExit::Completed(result))
        }
        Ok(RuntimeLoopExit::Failed(failure)) => {
            adapter.release_configured_model_budget_reservation(run_id, timestamp_now())?;
            Ok(RuntimeLoopExit::Failed(failure))
        }
        Err(error) => {
            adapter.release_configured_model_budget_reservation(run_id, timestamp_now())?;
            Err(error)
        }
    }
}

fn interrupted_model_execution_outcome(
    actor_manifest: &actor_manifest::CompiledActorManifest,
    resolved_target: &ResolvedExecutionTarget,
    run_id: &str,
    detail: &str,
) -> RuntimeCapabilityExecutionOutcome {
    RuntimeCapabilityExecutionOutcome {
        capability_id: Some(model_execution_target_ref(
            run_id,
            &resolved_target.configured_model_id,
        )),
        tool_name: Some(actor_manifest.label().to_string()),
        provider_key: None,
        dispatch_kind: Some("model_execution".into()),
        outcome: "failed".into(),
        detail: Some(detail.to_string()),
        requires_approval: false,
        requires_auth: false,
        concurrency_policy: Some("serialized".into()),
    }
}

fn build_runtime_checkpoint(
    current_iteration_index: u32,
    usage_summary: RuntimeUsageSummary,
    pending_approval: Option<ApprovalRequestRecord>,
    pending_auth_challenge: Option<RuntimeAuthChallengeSummary>,
    pending_mediation: Option<RuntimePendingMediationSummary>,
    capability_state_ref: Option<String>,
    capability_plan_summary: RuntimeCapabilityPlanSummary,
    last_execution_outcome: Option<RuntimeCapabilityExecutionOutcome>,
    last_mediation_outcome: Option<RuntimeMediationOutcome>,
    mediation_request: Option<&approval_broker::MediationRequest>,
    broker_decision: Option<&approval_broker::BrokerDecision>,
    checkpoint_artifact_ref: Option<String>,
) -> RuntimeRunCheckpoint {
    let broker_state = broker_decision.map(|decision| decision.state.clone());
    let (
        approval_layer,
        capability_id,
        tool_name,
        dispatch_kind,
        provider_key,
        concurrency_policy,
        input,
        reason,
        required_permission,
        requires_approval,
        requires_auth,
        target_kind,
        target_ref,
    ) = if let Some(request) = mediation_request {
        (
            Some(request.approval_layer.clone()),
            request.capability_id.clone(),
            Some(request.tool_name.clone()),
            Some(request.dispatch_kind.clone()),
            request.provider_key.clone(),
            Some(request.concurrency_policy.clone()),
            Some(request.input.clone()),
            request.escalation_reason.clone(),
            request.required_permission.clone(),
            Some(request.requires_approval),
            Some(request.requires_auth),
            Some(request.target_kind.clone()),
            Some(request.target_ref.clone()),
        )
    } else {
        (
            None, None, None, None, None, None, None, None, None, None, None, None, None,
        )
    };

    RuntimeRunCheckpoint {
        approval_layer,
        broker_decision: broker_state,
        capability_id,
        checkpoint_artifact_ref,
        current_iteration_index,
        tool_name,
        dispatch_kind,
        concurrency_policy,
        input,
        usage_summary,
        pending_approval,
        pending_auth_challenge,
        pending_mediation,
        provider_key,
        reason,
        required_permission,
        requires_approval,
        requires_auth,
        target_kind,
        target_ref,
        capability_state_ref,
        capability_plan_summary,
        last_execution_outcome,
        last_mediation_outcome,
    }
}

fn runtime_execution_mediation_request(
    run_context: &run_context::RunContext,
    input_content: &str,
    summary: String,
    detail: String,
    requires_approval: bool,
    checkpoint_ref: Option<String>,
    created_at: u64,
) -> approval_broker::MediationRequest {
    let policy_decision = run_context.execution_policy_decision.as_ref();
    approval_broker::MediationRequest {
        session_id: run_context.session_id.clone(),
        conversation_id: run_context.conversation_id.clone(),
        run_id: run_context.run_id.clone(),
        tool_name: run_context.actor_manifest.label().to_string(),
        summary,
        detail,
        mediation_kind: "approval".into(),
        approval_layer: "execution-permission".into(),
        target_kind: "model-execution".into(),
        target_ref: model_execution_target_ref(
            &run_context.run_id,
            &run_context.resolved_target.configured_model_id,
        ),
        capability_id: Some(model_execution_target_ref(
            &run_context.run_id,
            &run_context.resolved_target.configured_model_id,
        )),
        dispatch_kind: "model_execution".into(),
        provider_key: None,
        concurrency_policy: "serialized".into(),
        input: json!({
            "content": input_content,
            "requestedPermissionMode": run_context.requested_permission_mode,
        }),
        required_permission: policy_decision
            .and_then(|decision| decision.required_permission.clone())
            .or_else(|| Some(run_context.requested_permission_mode.clone())),
        escalation_reason: requires_approval.then(|| {
            policy_decision
                .and_then(|decision| decision.reason.clone())
                .unwrap_or_else(|| "session ceiling requires approval".into())
        }),
        requires_approval,
        requires_auth: false,
        created_at,
        risk_level: "medium".into(),
        checkpoint_ref,
        policy_action: policy_decision.map(|decision| decision.action.clone()),
        pending_state: None,
    }
}

fn runtime_provider_auth_mediation_request(
    run_context: &run_context::RunContext,
    checkpoint_ref: Option<String>,
    created_at: u64,
) -> Option<approval_broker::MediationRequest> {
    provider_auth_mediation_request(
        &run_context.session_id,
        &run_context.conversation_id,
        &run_context.run_id,
        &run_context.actor_manifest,
        &run_context.provider_state_summary,
        &run_context.auth_state_summary,
        run_context.provider_auth_policy_decision.as_ref(),
        checkpoint_ref,
        created_at,
    )
}

fn provider_auth_mediation_request(
    session_id: &str,
    conversation_id: &str,
    run_id: &str,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    provider_state_summary: &[RuntimeCapabilityProviderState],
    auth_state_summary: &RuntimeAuthStateSummary,
    policy_decision: Option<&RuntimeTargetPolicyDecision>,
    checkpoint_ref: Option<String>,
    created_at: u64,
) -> Option<approval_broker::MediationRequest> {
    let provider_key = auth_state_summary
        .challenged_provider_keys
        .first()
        .cloned()
        .or_else(|| {
            provider_state_summary
                .iter()
                .find(|provider| provider.state == "auth_required")
                .map(|provider| provider.provider_key.clone())
        })?;

    Some(approval_broker::MediationRequest {
        session_id: session_id.to_string(),
        conversation_id: conversation_id.to_string(),
        run_id: run_id.to_string(),
        tool_name: actor_manifest.label().to_string(),
        summary: format!("{} requires provider authentication", actor_manifest.label()),
        detail: format!(
            "Resolve provider or MCP authentication for `{provider_key}` before execution can continue."
        ),
        mediation_kind: "auth".into(),
        approval_layer: policy_decision
            .map(|value| value.target_kind.clone())
            .unwrap_or_else(|| "provider-auth".into()),
        target_kind: "provider-auth".into(),
        target_ref: provider_auth_target_ref(&provider_key),
        capability_id: policy_decision
            .and_then(|value| value.capability_id.clone())
            .or_else(|| Some(format!("provider-auth:{provider_key}"))),
        dispatch_kind: "provider_auth".into(),
        provider_key: Some(provider_key.clone()),
        concurrency_policy: "serialized".into(),
        input: json!({
            "providerKey": provider_key,
        }),
        required_permission: policy_decision.and_then(|value| value.required_permission.clone()),
        escalation_reason: policy_decision
            .and_then(|value| value.reason.clone())
            .or_else(|| Some("provider or MCP auth must resolve before execution can continue".into())),
        requires_approval: policy_decision.is_some_and(|value| value.requires_approval),
        requires_auth: true,
        created_at,
        risk_level: "medium".into(),
        checkpoint_ref,
        policy_action: policy_decision.map(|value| value.action.clone()),
        pending_state: None,
    })
}

fn provider_auth_required(projection: &capability_planner_bridge::CapabilityProjection) -> bool {
    !projection
        .auth_state_summary
        .challenged_provider_keys
        .is_empty()
        || projection
            .provider_state_summary
            .iter()
            .any(|provider| provider.state == "auth_required")
}

fn apply_checkpoint_ref(
    approval: &mut Option<ApprovalRequestRecord>,
    auth_target: &mut Option<RuntimeAuthChallengeSummary>,
    pending_mediation: &mut Option<RuntimePendingMediationSummary>,
    last_mediation_outcome: &mut Option<RuntimeMediationOutcome>,
    checkpoint_ref: &str,
) {
    if let Some(approval) = approval.as_mut() {
        approval.checkpoint_ref = Some(checkpoint_ref.to_string());
    }
    if let Some(challenge) = auth_target.as_mut() {
        challenge.checkpoint_ref = Some(checkpoint_ref.to_string());
    }
    if let Some(mediation) = pending_mediation.as_mut() {
        mediation.checkpoint_ref = Some(checkpoint_ref.to_string());
    }
    if let Some(outcome) = last_mediation_outcome.as_mut() {
        outcome.checkpoint_ref = Some(checkpoint_ref.to_string());
    }
}

fn finalize_mediation_checkpoint_ref(
    adapter: &RuntimeAdapter,
    session_id: &str,
    run_id: &str,
    approval: &mut Option<ApprovalRequestRecord>,
    auth_target: &mut Option<RuntimeAuthChallengeSummary>,
    pending_mediation: &mut Option<RuntimePendingMediationSummary>,
    last_mediation_outcome: &mut Option<RuntimeMediationOutcome>,
) -> Option<String> {
    let checkpoint_ref = pending_mediation
        .as_ref()
        .and_then(|mediation| mediation.mediation_id.as_deref())
        .map(|mediation_id| {
            adapter.runtime_mediation_checkpoint_ref(session_id, run_id, mediation_id)
        })
        .or_else(|| {
            pending_mediation
                .as_ref()
                .and_then(|mediation| mediation.checkpoint_ref.clone())
        })
        .or_else(|| {
            approval
                .as_ref()
                .and_then(|item| item.checkpoint_ref.clone())
        })
        .or_else(|| {
            auth_target
                .as_ref()
                .and_then(|item| item.checkpoint_ref.clone())
        })
        .or_else(|| {
            last_mediation_outcome
                .as_ref()
                .and_then(|item| item.checkpoint_ref.clone())
        });

    if let Some(checkpoint_ref) = checkpoint_ref.as_deref() {
        apply_checkpoint_ref(
            approval,
            auth_target,
            pending_mediation,
            last_mediation_outcome,
            checkpoint_ref,
        );
    }

    checkpoint_ref
}

fn memory_proposal_pending_mediation(
    run_context: &run_context::RunContext,
    proposal: &RuntimeMemoryProposal,
) -> approval_broker::MediationRequest {
    approval_broker::MediationRequest {
        session_id: run_context.session_id.clone(),
        conversation_id: run_context.conversation_id.clone(),
        run_id: run_context.run_id.clone(),
        tool_name: run_context.actor_manifest.label().to_string(),
        summary: proposal.summary.clone(),
        detail: proposal.proposal_reason.clone(),
        mediation_kind: "memory".into(),
        approval_layer: "memory-review".into(),
        target_kind: "memory-write".into(),
        target_ref: proposal.proposal_id.clone(),
        capability_id: Some(run_context.actor_manifest.actor_ref().to_string()),
        dispatch_kind: "memory_write".into(),
        provider_key: None,
        concurrency_policy: "serialized".into(),
        input: json!({
            "memoryId": proposal.memory_id,
            "scope": proposal.scope,
            "kind": proposal.kind,
            "summary": proposal.summary,
        }),
        required_permission: None,
        escalation_reason: Some("durable memory writes remain proposal-only until review".into()),
        requires_approval: false,
        requires_auth: false,
        created_at: run_context.now,
        risk_level: "low".into(),
        checkpoint_ref: None,
        policy_action: Some("defer".into()),
        pending_state: Some(proposal.proposal_state.clone()),
    }
}

fn memory_proposal_mediation_outcome(
    proposal: &RuntimeMemoryProposal,
    decision_status: &str,
    now: u64,
) -> RuntimeMediationOutcome {
    RuntimeMediationOutcome {
        approval_layer: Some("memory-review".into()),
        capability_id: None,
        checkpoint_ref: None,
        detail: Some(proposal.proposal_reason.clone()),
        mediation_id: Some(proposal.proposal_id.clone()),
        mediation_kind: "memory".into(),
        outcome: decision_status.into(),
        provider_key: None,
        reason: proposal
            .review
            .as_ref()
            .and_then(|review| review.note.clone())
            .or_else(|| Some(proposal.proposal_reason.clone())),
        requires_approval: false,
        requires_auth: false,
        resolved_at: Some(now),
        target_kind: "memory-write".into(),
        target_ref: proposal.proposal_id.clone(),
        tool_name: Some(proposal.title.clone()),
    }
}

fn blocking_mediation_state(
    approval: Option<&ApprovalRequestRecord>,
    auth_target: Option<&RuntimeAuthChallengeSummary>,
) -> (&'static str, &'static str, &'static str) {
    if approval.is_some() {
        ("waiting_approval", "awaiting_approval", "approval")
    } else if auth_target.is_some() {
        ("waiting_input", "awaiting_auth", "auth")
    } else {
        ("completed", "completed", "idle")
    }
}

fn apply_runtime_resolution_checkpoint(
    current_iteration_index: u32,
    usage_summary: RuntimeUsageSummary,
    pending_approval: Option<ApprovalRequestRecord>,
    pending_auth_challenge: Option<RuntimeAuthChallengeSummary>,
    pending_mediation: Option<RuntimePendingMediationSummary>,
    capability_state_ref: Option<String>,
    capability_plan_summary: RuntimeCapabilityPlanSummary,
    last_execution_outcome: Option<RuntimeCapabilityExecutionOutcome>,
    last_mediation_outcome: Option<RuntimeMediationOutcome>,
) -> RuntimeRunCheckpoint {
    let checkpoint_artifact_ref = pending_approval
        .as_ref()
        .and_then(|approval| approval.checkpoint_ref.clone())
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .and_then(|challenge| challenge.checkpoint_ref.clone())
        });
    let tool_name = pending_approval
        .as_ref()
        .map(|approval| approval.tool_name.clone())
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .and_then(|challenge| challenge.tool_name.clone())
        })
        .or_else(|| {
            pending_mediation
                .as_ref()
                .and_then(|mediation| mediation.tool_name.clone())
        });
    let dispatch_kind = pending_approval
        .as_ref()
        .and_then(|approval| approval.dispatch_kind.clone())
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .and_then(|challenge| challenge.dispatch_kind.clone())
        })
        .or_else(|| {
            pending_mediation
                .as_ref()
                .and_then(|mediation| mediation.dispatch_kind.clone())
        });
    let provider_key = pending_approval
        .as_ref()
        .and_then(|approval| approval.provider_key.clone())
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .and_then(|challenge| challenge.provider_key.clone())
        })
        .or_else(|| {
            pending_mediation
                .as_ref()
                .and_then(|mediation| mediation.provider_key.clone())
        });
    let concurrency_policy = pending_approval
        .as_ref()
        .and_then(|approval| approval.concurrency_policy.clone())
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .and_then(|challenge| challenge.concurrency_policy.clone())
        })
        .or_else(|| {
            pending_mediation
                .as_ref()
                .and_then(|mediation| mediation.concurrency_policy.clone())
        });
    let input = pending_approval
        .as_ref()
        .and_then(|approval| approval.input.clone())
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .and_then(|challenge| challenge.input.clone())
        })
        .or_else(|| {
            pending_mediation
                .as_ref()
                .and_then(|mediation| mediation.input.clone())
        });
    let approval_layer = pending_approval
        .as_ref()
        .and_then(|approval| approval.approval_layer.clone())
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .map(|challenge| challenge.approval_layer.clone())
        })
        .or_else(|| {
            pending_mediation
                .as_ref()
                .and_then(|mediation| mediation.approval_layer.clone())
        });
    let capability_id = pending_approval
        .as_ref()
        .and_then(|approval| approval.capability_id.clone())
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .and_then(|challenge| challenge.capability_id.clone())
        })
        .or_else(|| {
            pending_mediation
                .as_ref()
                .and_then(|mediation| mediation.capability_id.clone())
        });
    let reason = pending_approval
        .as_ref()
        .and_then(|approval| approval.escalation_reason.clone())
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .map(|challenge| challenge.escalation_reason.clone())
        })
        .or_else(|| {
            pending_mediation
                .as_ref()
                .and_then(|mediation| mediation.reason.clone())
        });
    let required_permission = pending_approval
        .as_ref()
        .and_then(|approval| approval.required_permission.clone())
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .and_then(|challenge| challenge.required_permission.clone())
        })
        .or_else(|| {
            pending_mediation
                .as_ref()
                .and_then(|mediation| mediation.required_permission.clone())
        });
    let requires_approval = pending_approval
        .as_ref()
        .map(|approval| approval.requires_approval)
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .map(|challenge| challenge.requires_approval)
        })
        .or_else(|| {
            pending_mediation
                .as_ref()
                .map(|mediation| mediation.requires_approval)
        });
    let requires_auth = pending_approval
        .as_ref()
        .map(|approval| approval.requires_auth)
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .map(|challenge| challenge.requires_auth)
        })
        .or_else(|| {
            pending_mediation
                .as_ref()
                .map(|mediation| mediation.requires_auth)
        });
    let target_kind = pending_approval
        .as_ref()
        .and_then(|approval| approval.target_kind.clone())
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .map(|challenge| challenge.target_kind.clone())
        })
        .or_else(|| Some(pending_mediation.as_ref()?.target_kind.clone()));
    let target_ref = pending_approval
        .as_ref()
        .and_then(|approval| approval.target_ref.clone())
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .map(|challenge| challenge.target_ref.clone())
        })
        .or_else(|| Some(pending_mediation.as_ref()?.target_ref.clone()));
    RuntimeRunCheckpoint {
        approval_layer,
        capability_id,
        usage_summary,
        current_iteration_index,
        tool_name,
        dispatch_kind,
        concurrency_policy,
        input,
        pending_approval,
        pending_auth_challenge,
        pending_mediation,
        provider_key,
        reason,
        required_permission,
        requires_approval,
        requires_auth,
        target_kind,
        target_ref,
        capability_state_ref,
        capability_plan_summary,
        last_execution_outcome,
        last_mediation_outcome,
        checkpoint_artifact_ref,
        ..Default::default()
    }
}

fn build_submit_trace(
    session_id: &str,
    run_id: &str,
    conversation_id: &str,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    now: u64,
    detail: String,
    tone: &str,
) -> RuntimeTraceItem {
    RuntimeTraceItem {
        id: format!("trace-{}", Uuid::new_v4()),
        session_id: session_id.to_string(),
        run_id: run_id.to_string(),
        conversation_id: conversation_id.to_string(),
        kind: "planner.step".into(),
        title: "Capability plan prepared".into(),
        detail,
        tone: tone.into(),
        timestamp: now,
        actor: actor_manifest.label().to_string(),
        actor_kind: Some(actor_manifest.actor_kind_label().into()),
        actor_id: Some(actor_manifest.actor_ref().to_string()),
        related_message_id: None,
        related_tool_name: None,
    }
}

fn build_execution_trace(
    session_id: &str,
    run_id: &str,
    conversation_id: &str,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    resolved_target: &ResolvedExecutionTarget,
    response: &ModelExecutionResult,
    now: u64,
    related_message_id: Option<String>,
) -> RuntimeTraceItem {
    RuntimeTraceItem {
        id: format!("trace-{}", Uuid::new_v4()),
        session_id: session_id.to_string(),
        run_id: run_id.to_string(),
        conversation_id: conversation_id.to_string(),
        kind: "model.step".into(),
        title: "Model loop completed".into(),
        detail: format!(
            "Resolved {}:{} via {} and produced {} characters.",
            resolved_target.provider_id,
            resolved_target.configured_model_name,
            resolved_target.protocol_family,
            response.content.chars().count()
        ),
        tone: "success".into(),
        timestamp: now,
        actor: actor_manifest.label().to_string(),
        actor_kind: Some(actor_manifest.actor_kind_label().into()),
        actor_id: Some(actor_manifest.actor_ref().to_string()),
        related_message_id,
        related_tool_name: None,
    }
}

fn subrun_checkpoint_content(
    checkpoint: &RuntimeRunCheckpoint,
    serialized_session: &Value,
) -> Option<String> {
    checkpoint
        .input
        .as_ref()
        .and_then(|input| input.get("content"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            serialized_session
                .get("content")
                .or_else(|| serialized_session.get("pendingContent"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(ToString::to_string)
        })
        .or_else(|| {
            serialized_session
                .get("session")
                .and_then(|session| session.get("messages"))
                .and_then(Value::as_array)
                .and_then(|messages| messages.first())
                .and_then(|message| message.get("blocks"))
                .and_then(Value::as_array)
                .and_then(|blocks| blocks.first())
                .and_then(|block| block.get("text"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(ToString::to_string)
        })
}

fn durable_subrun_content(state: &team_runtime::PersistedSubrunState) -> Result<String, AppError> {
    if !state.dispatch.worker_input.content.trim().is_empty() {
        return Ok(state.dispatch.worker_input.content.clone());
    }

    subrun_checkpoint_content(&state.run.checkpoint, &state.serialized_session)
        .ok_or_else(|| AppError::runtime("subrun checkpoint content is unavailable"))
}

fn resumable_subrun_status(status: &str) -> bool {
    matches!(
        status,
        "queued" | "running" | "waiting_approval" | "auth-required"
    )
}

fn primary_run_is_blocking_team_subruns(detail: &RuntimeSessionDetail) -> bool {
    detail
        .run
        .checkpoint
        .pending_approval
        .as_ref()
        .is_some_and(|approval| approval.run_id == detail.run.id && approval.status == "pending")
        || detail
            .run
            .checkpoint
            .pending_auth_challenge
            .as_ref()
            .is_some_and(|challenge| {
                challenge.run_id == detail.run.id && challenge.status == "pending"
            })
        || detail.pending_approval.as_ref().is_some_and(|approval| {
            approval.run_id == detail.run.id && approval.status == "pending"
        })
        || detail.run.auth_target.as_ref().is_some_and(|challenge| {
            challenge.run_id == detail.run.id && challenge.status == "pending"
        })
        || matches!(detail.run.status.as_str(), "waiting_input" | "blocked")
}

async fn execute_team_subrun(
    adapter: &RuntimeAdapter,
    session_id: &str,
    conversation_id: &str,
    parent_run_id: &str,
    state: &team_runtime::PersistedSubrunState,
    resolved_mediation_outcome: Option<RuntimeMediationOutcome>,
) -> Result<TeamSubrunExecutionOutcome, AppError> {
    if !resumable_subrun_status(&state.run.status) {
        return Ok(TeamSubrunExecutionOutcome {
            state: state.clone(),
            planner_events: Vec::new(),
            model_iterations: Vec::new(),
            capability_events: Vec::new(),
            runtime_error: None,
        });
    }

    let session_policy =
        adapter.load_session_policy_snapshot(&state.session_policy_snapshot_ref)?;
    let actor_manifest = adapter.load_actor_manifest_snapshot(&state.manifest_snapshot_ref)?;
    let configured_model_id = state
        .run
        .configured_model_id
        .clone()
        .or_else(|| session_policy.selected_configured_model_id.clone())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::runtime("configured model is unavailable for subrun execution"))?;
    let (resolved_target, configured_model) = adapter
        .resolve_approved_execution(&session_policy.config_snapshot_id, &configured_model_id)?;
    let capability_state_ref = state
        .run
        .capability_state_ref
        .clone()
        .unwrap_or_else(|| format!("{}-capability-state", state.run.id));
    let content = durable_subrun_content(state)?;
    let trace_context = if state.run.trace_context.session_id.is_empty() {
        let restored = trace_context_from_serialized_session(&state.serialized_session);
        if restored.session_id.is_empty() {
            trace_context::runtime_trace_context(session_id, Some(state.run.id.clone()))
        } else {
            restored
        }
    } else {
        state.run.trace_context.clone()
    };
    let session = restore_runtime_session(&state.serialized_session, &content)?;
    let pending_tool_uses = pending_tool_uses_from_serialized_session(&state.serialized_session)?;
    let requested_permission_mode = requested_permission_mode_from_serialized_session(
        &state.serialized_session,
        &session_policy.execution_permission_mode,
    );
    let loop_exit = execute_runtime_turn_loop_with_budget_reservation(
        adapter,
        session_id,
        conversation_id,
        &state.run.id,
        &resolved_target,
        &configured_model,
        &actor_manifest,
        &session_policy,
        &requested_permission_mode,
        &capability_state_ref,
        &content,
        &trace_context,
        session,
        pending_tool_uses,
        state.run.checkpoint.current_iteration_index,
        state.run.checkpoint.usage_summary.clone(),
    )
    .await?;

    let mut updated = state.clone();
    let updated_at = timestamp_now();
    let (
        checkpoint,
        consumed_tokens,
        approval,
        auth_target,
        pending_mediation,
        usage_summary,
        capability_state_ref,
        capability_plan_summary,
        provider_state_summary,
        last_execution_outcome,
        last_mediation_outcome,
        planner_events,
        model_iterations,
        capability_events,
        runtime_error,
    ) = match loop_exit {
        RuntimeLoopExit::Completed(loop_result) => {
            updated.serialized_session = loop_result.serialized_session.clone();
            let mut approval = loop_result
                .broker_decision
                .as_ref()
                .and_then(|decision| decision.approval.clone());
            let mut auth_target = loop_result
                .broker_decision
                .as_ref()
                .and_then(|decision| decision.auth_challenge.clone());
            let mut pending_mediation = loop_result
                .broker_decision
                .as_ref()
                .and_then(|decision| decision.pending_mediation.clone());
            let last_execution_outcome = loop_result
                .broker_decision
                .as_ref()
                .map(|decision| decision.execution_outcome.clone());
            let mut last_mediation_outcome = loop_result
                .broker_decision
                .as_ref()
                .and_then(|decision| decision.mediation_outcome.clone())
                .or(resolved_mediation_outcome.clone());
            let checkpoint_artifact_ref = finalize_mediation_checkpoint_ref(
                adapter,
                session_id,
                &state.run.id,
                &mut approval,
                &mut auth_target,
                &mut pending_mediation,
                &mut last_mediation_outcome,
            );
            let mut checkpoint = build_runtime_checkpoint(
                loop_result.current_iteration_index,
                loop_result.usage_summary.clone(),
                approval.clone(),
                auth_target.clone(),
                pending_mediation.clone(),
                Some(
                    loop_result
                        .capability_projection
                        .capability_state_ref
                        .clone(),
                ),
                loop_result.capability_projection.plan_summary.clone(),
                last_execution_outcome.clone(),
                last_mediation_outcome.clone(),
                loop_result.mediation_request.as_ref(),
                loop_result.broker_decision.as_ref(),
                checkpoint_artifact_ref.clone(),
            );
            if let Some(mediation_id) = pending_mediation
                .as_ref()
                .and_then(|mediation| mediation.mediation_id.as_deref())
            {
                let checkpoint_artifact =
                    persistence::PersistedRuntimeCheckpointArtifact::from_public_checkpoint(
                        checkpoint.clone(),
                        loop_result.serialized_session.clone(),
                        json!({}),
                    );
                let (storage_path, _) = adapter.persist_runtime_mediation_checkpoint(
                    session_id,
                    &state.run.id,
                    mediation_id,
                    &checkpoint_artifact,
                )?;
                checkpoint.checkpoint_artifact_ref = Some(storage_path.clone());
                apply_checkpoint_ref(
                    &mut approval,
                    &mut auth_target,
                    &mut pending_mediation,
                    &mut last_mediation_outcome,
                    &storage_path,
                );
                checkpoint.pending_approval = approval.clone();
                checkpoint.pending_auth_challenge = auth_target.clone();
                checkpoint.pending_mediation = pending_mediation.clone();
                checkpoint.last_mediation_outcome = last_mediation_outcome.clone();
            }
            (
                checkpoint,
                loop_result.consumed_tokens,
                approval,
                auth_target,
                pending_mediation,
                loop_result.usage_summary,
                loop_result.capability_projection.capability_state_ref,
                loop_result.capability_projection.plan_summary,
                loop_result.capability_projection.provider_state_summary,
                last_execution_outcome,
                last_mediation_outcome,
                loop_result.planner_events,
                loop_result.model_iterations,
                loop_result.capability_events,
                None,
            )
        }
        RuntimeLoopExit::Failed(loop_failure) => {
            updated.serialized_session = loop_failure.serialized_session.clone();
            let last_execution_outcome = Some(interrupted_model_execution_outcome(
                &actor_manifest,
                &resolved_target,
                &state.run.id,
                &loop_failure.error,
            ));
            let checkpoint = persist_runtime_resolution_failure_checkpoint(
                adapter,
                session_id,
                &state.run.id,
                loop_failure.current_iteration_index,
                loop_failure.usage_summary.clone(),
                &loop_failure.capability_projection.capability_state_ref,
                loop_failure.capability_projection.plan_summary.clone(),
                last_execution_outcome.clone(),
                resolved_mediation_outcome.clone(),
                loop_failure.serialized_session.clone(),
            )?;
            (
                checkpoint,
                None,
                None,
                None,
                None,
                loop_failure.usage_summary,
                loop_failure.capability_projection.capability_state_ref,
                loop_failure.capability_projection.plan_summary,
                loop_failure.capability_projection.provider_state_summary,
                last_execution_outcome,
                resolved_mediation_outcome.clone(),
                loop_failure.planner_events,
                loop_failure.model_iterations,
                loop_failure.capability_events,
                Some(loop_failure.error),
            )
        }
    };

    let (status, current_step, next_action, approval_state) = if runtime_error.is_some() {
        ("failed", "failed", "idle", "not-required")
    } else if approval.is_some() {
        (
            "waiting_approval",
            "awaiting_approval",
            "approval",
            "pending",
        )
    } else if auth_target.is_some() {
        ("auth-required", "awaiting_auth", "auth", "auth-required")
    } else {
        ("completed", "completed", "idle", "not-required")
    };

    updated.run.status = status.into();
    updated.run.current_step = current_step.into();
    updated.run.updated_at = updated_at;
    updated.run.consumed_tokens = consumed_tokens;
    updated.run.next_action = Some(next_action.into());
    updated.run.configured_model_id = Some(resolved_target.configured_model_id.clone());
    updated.run.configured_model_name = Some(resolved_target.configured_model_name.clone());
    updated.run.model_id = Some(resolved_target.registry_model_id.clone());
    updated.run.run_kind = "subrun".into();
    updated.run.parent_run_id = Some(parent_run_id.to_string());
    updated.run.actor_ref = actor_manifest.actor_ref().to_string();
    updated.run.workflow_run = state.run.workflow_run.clone();
    updated.run.mailbox_ref = state.run.mailbox_ref.clone();
    updated.run.handoff_ref = state.run.handoff_ref.clone();
    updated.run.background_state = None;
    updated.run.worker_dispatch = None;
    updated.run.approval_state = approval_state.into();
    updated.run.approval_target = approval;
    updated.run.auth_target = auth_target;
    updated.run.usage_summary = usage_summary;
    updated.run.artifact_refs = if runtime_error.is_some() {
        Vec::new()
    } else {
        vec![persistence::runtime_output_artifact_ref(&state.run.id)]
    };
    updated.run.deliverable_refs = Vec::new();
    updated.run.trace_context = trace_context;
    updated.run.checkpoint = checkpoint;
    updated.run.capability_plan_summary = capability_plan_summary;
    updated.run.provider_state_summary = provider_state_summary;
    updated.run.pending_mediation = pending_mediation;
    updated.run.capability_state_ref = Some(capability_state_ref);
    updated.run.last_execution_outcome = last_execution_outcome;
    updated.run.last_mediation_outcome = last_mediation_outcome;
    updated.run.resolved_target = Some(resolved_target);
    updated.run.requested_actor_kind = Some(actor_manifest.actor_kind_label().into());
    updated.run.requested_actor_id = Some(actor_manifest.actor_ref().to_string());
    updated.run.resolved_actor_kind = Some(actor_manifest.actor_kind_label().into());
    updated.run.resolved_actor_id = Some(actor_manifest.actor_ref().to_string());
    updated.run.resolved_actor_label = Some(actor_manifest.label().to_string());

    Ok(TeamSubrunExecutionOutcome {
        state: updated,
        planner_events,
        model_iterations,
        capability_events,
        runtime_error,
    })
}

async fn continue_team_runtime_subruns(
    adapter: &RuntimeAdapter,
    session_id: &str,
    team_manifest: &actor_manifest::CompiledTeamManifest,
) -> Result<Option<(RuntimeRunSnapshot, String, String)>, AppError> {
    let (conversation_id, project_id, parent_run_id, blocked) = {
        let sessions = adapter
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        (
            aggregate.detail.summary.conversation_id.clone(),
            aggregate.detail.summary.project_id.clone(),
            aggregate.detail.run.id.clone(),
            primary_run_is_blocking_team_subruns(&aggregate.detail),
        )
    };

    if blocked {
        return Ok(None);
    }

    loop {
        let scheduled_run_ids = {
            let mut sessions = adapter
                .state
                .sessions
                .lock()
                .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
            let aggregate = sessions
                .get_mut(session_id)
                .ok_or_else(|| AppError::not_found("runtime session"))?;
            let concurrency_limit = worker_runtime::worker_concurrency_limit(team_manifest);
            let tick_now = timestamp_now();
            let scheduler_tick = subrun_orchestrator::schedule_subrun_tick(
                &mut aggregate.metadata.subrun_states,
                concurrency_limit,
                tick_now,
            );
            if scheduler_tick.runnable_run_ids.is_empty() {
                break;
            }
            team_runtime::apply_team_runtime_state(
                &mut aggregate.detail,
                team_manifest,
                &aggregate.metadata.subrun_states,
                tick_now,
            );
            aggregate.detail.run.updated_at = tick_now;
            aggregate.detail.summary.updated_at = tick_now;
            sync_runtime_session_detail(&mut aggregate.detail);
            adapter.persist_runtime_projections(aggregate)?;
            scheduler_tick.runnable_run_ids
        };

        for run_id in scheduled_run_ids {
            let next_state = {
                let sessions = adapter
                    .state
                    .sessions
                    .lock()
                    .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
                let aggregate = sessions
                    .get(session_id)
                    .ok_or_else(|| AppError::not_found("runtime session"))?;
                aggregate
                    .metadata
                    .subrun_states
                    .get(&run_id)
                    .cloned()
                    .ok_or_else(|| AppError::not_found("runtime subrun state"))?
            };

            let updated_state = execute_team_subrun(
                adapter,
                session_id,
                &conversation_id,
                &parent_run_id,
                &next_state,
                None,
            )
            .await?;
            let updated_at = updated_state.state.run.updated_at;

            let mut sessions = adapter
                .state
                .sessions
                .lock()
                .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
            let aggregate = sessions
                .get_mut(session_id)
                .ok_or_else(|| AppError::not_found("runtime session"))?;
            aggregate
                .metadata
                .subrun_states
                .insert(updated_state.state.run.id.clone(), updated_state.state);
            team_runtime::apply_team_runtime_state(
                &mut aggregate.detail,
                team_manifest,
                &aggregate.metadata.subrun_states,
                updated_at,
            );
            aggregate.detail.run.updated_at = updated_at;
            aggregate.detail.summary.updated_at = updated_at;
            sync_runtime_session_detail(&mut aggregate.detail);
            adapter.persist_runtime_projections(aggregate)?;
        }
    }

    let sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    let aggregate = sessions
        .get(session_id)
        .ok_or_else(|| AppError::not_found("runtime session"))?;
    Ok(Some((
        aggregate.detail.run.clone(),
        conversation_id,
        project_id,
    )))
}

fn load_session_team_manifest(
    adapter: &RuntimeAdapter,
    session_id: &str,
) -> Result<actor_manifest::CompiledTeamManifest, AppError> {
    let session_policy_snapshot_ref = {
        let sessions = adapter
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        aggregate.metadata.session_policy_snapshot_ref.clone()
    };
    let session_policy = adapter.load_session_policy_snapshot(&session_policy_snapshot_ref)?;
    match adapter.load_actor_manifest_snapshot(&session_policy.manifest_snapshot_ref)? {
        actor_manifest::CompiledActorManifest::Team(team_manifest) => Ok(team_manifest),
        actor_manifest::CompiledActorManifest::Agent(_) => {
            Err(AppError::runtime("runtime session is not a team session"))
        }
    }
}

fn persist_team_subrun_state(
    adapter: &RuntimeAdapter,
    session_id: &str,
    team_manifest: &actor_manifest::CompiledTeamManifest,
    updated_state: team_runtime::PersistedSubrunState,
    now: u64,
) -> Result<(RuntimeRunSnapshot, String, String), AppError> {
    let mut sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    let aggregate = sessions
        .get_mut(session_id)
        .ok_or_else(|| AppError::not_found("runtime session"))?;
    aggregate
        .metadata
        .subrun_states
        .insert(updated_state.run.id.clone(), updated_state);
    team_runtime::apply_team_runtime_state(
        &mut aggregate.detail,
        team_manifest,
        &aggregate.metadata.subrun_states,
        now,
    );
    aggregate.detail.run.updated_at = now;
    aggregate.detail.summary.updated_at = now;
    sync_runtime_session_detail(&mut aggregate.detail);
    let run = aggregate.detail.run.clone();
    let conversation_id = aggregate.detail.summary.conversation_id.clone();
    let project_id = aggregate.detail.summary.project_id.clone();
    adapter.persist_runtime_projections(aggregate)?;
    Ok((run, conversation_id, project_id))
}

fn load_pending_team_subrun_approval(
    adapter: &RuntimeAdapter,
    session_id: &str,
    approval_id: &str,
) -> Result<Option<(ApprovalRequestRecord, team_runtime::PersistedSubrunState)>, AppError> {
    let (approval, state) = {
        let sessions = adapter
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        let approval = aggregate
            .detail
            .pending_approval
            .clone()
            .ok_or_else(|| runtime_approval_lookup_error(aggregate, approval_id))?;
        if approval.id != approval_id {
            return Err(runtime_approval_lookup_error(aggregate, approval_id));
        }
        if approval.run_id == aggregate.detail.run.id {
            return Ok(None);
        }
        let state = aggregate
            .metadata
            .subrun_states
            .get(&approval.run_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("runtime subrun state"))?;
        (approval, state)
    };
    let subrun_approval = state
        .run
        .approval_target
        .clone()
        .or_else(|| state.run.checkpoint.pending_approval.clone())
        .ok_or_else(|| {
            AppError::conflict(format!(
                "runtime approval `{approval_id}` is already consumed"
            ))
        })?;
    if subrun_approval.id != approval_id {
        return Err(AppError::conflict(format!(
            "runtime approval `{approval_id}` is already consumed"
        )));
    }
    if approval.status != "pending" {
        return Err(AppError::conflict(format!(
            "runtime approval `{approval_id}` is already {}",
            approval.status
        )));
    }
    if !resumable_approval_target(approval.target_kind.as_deref()) {
        return Err(AppError::invalid_input(format!(
            "runtime approval `{approval_id}` does not target a resumable checkpoint"
        )));
    }
    Ok(Some((approval, state)))
}

fn load_pending_team_subrun_auth_challenge(
    adapter: &RuntimeAdapter,
    session_id: &str,
    challenge_id: &str,
) -> Result<
    Option<(
        RuntimeAuthChallengeSummary,
        team_runtime::PersistedSubrunState,
    )>,
    AppError,
> {
    let (challenge, state) = {
        let sessions = adapter
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        let challenge = aggregate
            .detail
            .run
            .auth_target
            .clone()
            .or_else(|| {
                aggregate
                    .detail
                    .run
                    .checkpoint
                    .pending_auth_challenge
                    .clone()
            })
            .ok_or_else(|| AppError::not_found("runtime auth challenge"))?;
        if challenge.id != challenge_id {
            return Err(AppError::not_found("runtime auth challenge"));
        }
        if challenge.run_id == aggregate.detail.run.id {
            return Ok(None);
        }
        let state = aggregate
            .metadata
            .subrun_states
            .get(&challenge.run_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("runtime subrun state"))?;
        (challenge, state)
    };
    let subrun_challenge = state
        .run
        .auth_target
        .clone()
        .or_else(|| state.run.checkpoint.pending_auth_challenge.clone())
        .ok_or_else(|| AppError::not_found("runtime auth challenge"))?;
    if subrun_challenge.id != challenge_id {
        return Err(AppError::not_found("runtime auth challenge"));
    }
    if challenge.status != "pending" {
        return Err(AppError::invalid_input(format!(
            "runtime auth challenge `{challenge_id}` is already {}",
            challenge.status
        )));
    }
    if !resumable_auth_target(&challenge.target_kind) {
        return Err(AppError::invalid_input(format!(
            "runtime auth challenge `{challenge_id}` does not target a resumable auth checkpoint"
        )));
    }
    Ok(Some((challenge, state)))
}

fn load_cancelable_team_subrun_state(
    adapter: &RuntimeAdapter,
    session_id: &str,
    subrun_id: &str,
) -> Result<team_runtime::PersistedSubrunState, AppError> {
    let state = {
        let sessions = adapter
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        if aggregate.detail.run.id == subrun_id {
            return Err(AppError::invalid_input(
                "primary run cannot be cancelled through the subrun API",
            ));
        }
        aggregate
            .metadata
            .subrun_states
            .get(subrun_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("runtime subrun"))?
    };

    if !resumable_subrun_status(&state.run.status) {
        return Err(AppError::invalid_input(format!(
            "runtime subrun `{subrun_id}` is already {}",
            state.run.status
        )));
    }

    Ok(state)
}

fn cancelled_subrun_state(
    state: &team_runtime::PersistedSubrunState,
    note: Option<&str>,
    now: u64,
) -> team_runtime::PersistedSubrunState {
    let mut updated = state.clone();
    let note = note
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let detail = note
        .clone()
        .map(|value| format!("subrun cancelled: {value}"))
        .or_else(|| Some("subrun cancelled explicitly".into()));
    let requires_approval =
        state.run.approval_target.is_some() || state.run.checkpoint.pending_approval.is_some();
    let requires_auth =
        state.run.auth_target.is_some() || state.run.checkpoint.pending_auth_challenge.is_some();
    let last_execution_outcome = Some(capability_execution_outcome(
        "cancelled",
        detail.clone(),
        requires_approval,
        requires_auth,
    ));
    let last_mediation_outcome = state
        .run
        .pending_mediation
        .clone()
        .or_else(|| state.run.checkpoint.pending_mediation.clone())
        .map(|mediation| RuntimeMediationOutcome {
            approval_layer: mediation.approval_layer.clone(),
            capability_id: mediation.capability_id.clone(),
            checkpoint_ref: mediation.checkpoint_ref.clone(),
            detail: detail.clone().or_else(|| mediation.detail.clone()),
            mediation_id: mediation.mediation_id.clone(),
            mediation_kind: mediation.mediation_kind.clone(),
            outcome: "cancelled".into(),
            provider_key: mediation.provider_key.clone(),
            reason: note
                .clone()
                .or_else(|| mediation.reason.clone())
                .or_else(|| mediation.escalation_reason.clone()),
            requires_approval: mediation.requires_approval,
            requires_auth: mediation.requires_auth,
            resolved_at: Some(now),
            target_kind: mediation.target_kind.clone(),
            target_ref: mediation.target_ref.clone(),
            tool_name: mediation.tool_name.clone(),
        });

    updated.run.status = "cancelled".into();
    updated.run.current_step = "cancelled".into();
    updated.run.updated_at = now;
    updated.run.next_action = Some("idle".into());
    updated.run.approval_state = "cancelled".into();
    updated.run.approval_target = None;
    updated.run.auth_target = None;
    updated.run.pending_mediation = None;
    updated.run.last_execution_outcome = last_execution_outcome.clone();
    updated.run.last_mediation_outcome = last_mediation_outcome.clone();
    updated.run.checkpoint.pending_approval = None;
    updated.run.checkpoint.pending_auth_challenge = None;
    updated.run.checkpoint.pending_mediation = None;
    updated.run.checkpoint.last_execution_outcome = last_execution_outcome;
    updated.run.checkpoint.last_mediation_outcome = last_mediation_outcome;

    updated
}

fn rejected_subrun_approval_state(
    state: &team_runtime::PersistedSubrunState,
    approval: &ApprovalRequestRecord,
    decision_status: &str,
    now: u64,
) -> team_runtime::PersistedSubrunState {
    let mut updated = state.clone();
    let last_execution_outcome = Some(RuntimeCapabilityExecutionOutcome {
        capability_id: approval.capability_id.clone(),
        tool_name: Some(approval.tool_name.clone()),
        provider_key: approval.provider_key.clone(),
        dispatch_kind: approval.target_kind.clone(),
        outcome: "deny".into(),
        detail: Some("approval request was rejected".into()),
        requires_approval: approval.requires_approval,
        requires_auth: approval.requires_auth,
        concurrency_policy: Some("serialized".into()),
    });
    let last_mediation_outcome = Some(RuntimeMediationOutcome {
        approval_layer: approval.approval_layer.clone(),
        capability_id: approval.capability_id.clone(),
        checkpoint_ref: approval.checkpoint_ref.clone(),
        detail: Some(approval.detail.clone()),
        mediation_id: Some(approval.id.clone()),
        mediation_kind: "approval".into(),
        outcome: decision_status.into(),
        provider_key: approval.provider_key.clone(),
        reason: approval.escalation_reason.clone(),
        requires_approval: approval.requires_approval,
        requires_auth: approval.requires_auth,
        resolved_at: Some(now),
        target_kind: approval.target_kind.clone().unwrap_or_default(),
        target_ref: approval.target_ref.clone().unwrap_or_default(),
        tool_name: Some(approval.tool_name.clone()),
    });

    updated.run.status = "failed".into();
    updated.run.current_step = "failed".into();
    updated.run.updated_at = now;
    updated.run.next_action = Some("idle".into());
    updated.run.approval_state = decision_status.into();
    updated.run.approval_target = None;
    updated.run.pending_mediation = None;
    updated.run.last_execution_outcome = last_execution_outcome.clone();
    updated.run.last_mediation_outcome = last_mediation_outcome.clone();
    updated.run.checkpoint.pending_approval = None;
    updated.run.checkpoint.pending_mediation = None;
    updated.run.checkpoint.last_execution_outcome = last_execution_outcome;
    updated.run.checkpoint.last_mediation_outcome = last_mediation_outcome;

    updated
}

fn resolved_subrun_approval_mediation_outcome(
    approval: &ApprovalRequestRecord,
    decision_status: &str,
    now: u64,
) -> RuntimeMediationOutcome {
    RuntimeMediationOutcome {
        approval_layer: approval.approval_layer.clone(),
        capability_id: approval.capability_id.clone(),
        checkpoint_ref: approval.checkpoint_ref.clone(),
        detail: Some(approval.detail.clone()),
        mediation_id: Some(approval.id.clone()),
        mediation_kind: "approval".into(),
        outcome: decision_status.into(),
        provider_key: approval.provider_key.clone(),
        reason: approval.escalation_reason.clone(),
        requires_approval: approval.requires_approval,
        requires_auth: approval.requires_auth,
        resolved_at: Some(now),
        target_kind: approval.target_kind.clone().unwrap_or_default(),
        target_ref: approval.target_ref.clone().unwrap_or_default(),
        tool_name: Some(approval.tool_name.clone()),
    }
}

fn resolved_subrun_auth_mediation_outcome(
    challenge: &RuntimeAuthChallengeSummary,
    resolution: &str,
    now: u64,
) -> RuntimeMediationOutcome {
    RuntimeMediationOutcome {
        approval_layer: Some(challenge.approval_layer.clone()),
        capability_id: challenge.capability_id.clone(),
        checkpoint_ref: challenge.checkpoint_ref.clone(),
        detail: Some(challenge.detail.clone()),
        mediation_id: Some(challenge.id.clone()),
        mediation_kind: "auth".into(),
        outcome: resolution.into(),
        provider_key: challenge.provider_key.clone(),
        reason: Some(challenge.escalation_reason.clone()),
        requires_approval: challenge.requires_approval,
        requires_auth: challenge.requires_auth,
        resolved_at: Some(now),
        target_kind: challenge.target_kind.clone(),
        target_ref: challenge.target_ref.clone(),
        tool_name: challenge.tool_name.clone(),
    }
}

fn resolved_subrun_auth_state(
    state: &team_runtime::PersistedSubrunState,
    challenge: &RuntimeAuthChallengeSummary,
    resolution: &str,
    now: u64,
) -> team_runtime::PersistedSubrunState {
    let mut updated = state.clone();
    let last_execution_outcome = Some(capability_execution_outcome(
        if resolution == "cancelled" {
            "cancelled"
        } else {
            "deny"
        },
        Some(format!("auth challenge ended with status `{resolution}`")),
        false,
        true,
    ));
    let last_mediation_outcome = Some(resolved_subrun_auth_mediation_outcome(
        challenge, resolution, now,
    ));

    updated.run.status = if resolution == "cancelled" {
        "cancelled".into()
    } else {
        "failed".into()
    };
    updated.run.current_step = if resolution == "cancelled" {
        "cancelled".into()
    } else {
        "failed".into()
    };
    updated.run.updated_at = now;
    updated.run.next_action = Some("idle".into());
    updated.run.approval_state = resolution.into();
    updated.run.auth_target = None;
    updated.run.pending_mediation = None;
    updated.run.last_execution_outcome = last_execution_outcome.clone();
    updated.run.last_mediation_outcome = last_mediation_outcome.clone();
    updated.run.checkpoint.pending_auth_challenge = None;
    updated.run.checkpoint.pending_mediation = None;
    updated.run.checkpoint.last_execution_outcome = last_execution_outcome;
    updated.run.checkpoint.last_mediation_outcome = last_mediation_outcome;

    updated
}

async fn resolve_team_subrun_approval(
    adapter: &RuntimeAdapter,
    session_id: &str,
    approval_id: &str,
    decision_status: &str,
) -> Result<TeamSubrunApprovalResolutionState, AppError> {
    let now = timestamp_now();
    let team_manifest = load_session_team_manifest(adapter, session_id)?;
    let (approval, state) = load_pending_team_subrun_approval(adapter, session_id, approval_id)?
        .ok_or_else(|| AppError::not_found("runtime approval"))?;
    let mut resolved_approval = approval.clone();
    resolved_approval.status = decision_status.into();
    let resolution_mediation_outcome =
        resolved_subrun_approval_mediation_outcome(&approval, decision_status, now);

    let execution_outcome = if decision_status == "approved" {
        if approval.target_kind.as_deref() == Some("capability-call") {
            let capability_state_ref = state
                .run
                .capability_state_ref
                .clone()
                .unwrap_or_else(|| format!("{}-capability-state", state.run.id));
            let capability_store = adapter.load_capability_store(Some(&capability_state_ref))?;
            capability_store.approve_tool(approval.tool_name.clone());
            adapter.persist_capability_store(&capability_state_ref, &capability_store)?;
        }
        if approval_replays_runtime_loop(approval.target_kind.as_deref()) {
            let parent_run_id = state.run.parent_run_id.clone().ok_or_else(|| {
                AppError::runtime("team subrun parent run id is unavailable for approval resume")
            })?;
            execute_team_subrun(
                adapter,
                session_id,
                &state.run.conversation_id,
                &parent_run_id,
                &state,
                Some(resolution_mediation_outcome.clone()),
            )
            .await?
        } else {
            let mut updated = state.clone();
            updated.run.status = "completed".into();
            updated.run.current_step = "completed".into();
            updated.run.updated_at = now;
            updated.run.next_action = Some("idle".into());
            updated.run.approval_state = "not-required".into();
            updated.run.approval_target = None;
            updated.run.pending_mediation = None;
            updated.run.last_mediation_outcome = Some(resolution_mediation_outcome.clone());
            updated.run.checkpoint.pending_approval = None;
            updated.run.checkpoint.pending_mediation = None;
            updated.run.checkpoint.last_mediation_outcome = Some(resolution_mediation_outcome);
            TeamSubrunExecutionOutcome {
                state: updated,
                planner_events: Vec::new(),
                model_iterations: Vec::new(),
                capability_events: Vec::new(),
                runtime_error: None,
            }
        }
    } else {
        TeamSubrunExecutionOutcome {
            state: rejected_subrun_approval_state(&state, &approval, decision_status, now),
            planner_events: Vec::new(),
            model_iterations: Vec::new(),
            capability_events: Vec::new(),
            runtime_error: None,
        }
    };

    let (mut run, mut conversation_id, mut project_id) = persist_team_subrun_state(
        adapter,
        session_id,
        &team_manifest,
        execution_outcome.state,
        now,
    )?;
    if let Some((updated_run, updated_conversation_id, updated_project_id)) =
        continue_team_runtime_subruns(adapter, session_id, &team_manifest).await?
    {
        run = updated_run;
        conversation_id = updated_conversation_id;
        project_id = updated_project_id;
    }

    Ok((
        resolved_approval,
        None,
        None,
        run,
        conversation_id,
        project_id,
        execution_outcome.planner_events,
        execution_outcome.model_iterations,
        execution_outcome.capability_events,
        execution_outcome.runtime_error,
    ))
}

async fn resolve_team_subrun_auth_challenge(
    adapter: &RuntimeAdapter,
    session_id: &str,
    challenge_id: &str,
    resolution: &str,
) -> Result<TeamSubrunAuthChallengeResolutionState, AppError> {
    let now = timestamp_now();
    let team_manifest = load_session_team_manifest(adapter, session_id)?;
    let (challenge, state) =
        load_pending_team_subrun_auth_challenge(adapter, session_id, challenge_id)?
            .ok_or_else(|| AppError::not_found("runtime auth challenge"))?;
    let mut resolved_challenge = challenge.clone();
    resolved_challenge.status = resolution.into();
    resolved_challenge.resolution = Some(resolution.into());
    let resolution_mediation_outcome =
        resolved_subrun_auth_mediation_outcome(&challenge, resolution, now);

    let execution_outcome = if resolution == "resolved" {
        let capability_state_ref = state
            .run
            .capability_state_ref
            .clone()
            .unwrap_or_else(|| format!("{}-capability-state", state.run.id));
        let capability_store = adapter.load_capability_store(Some(&capability_state_ref))?;
        capability_store.resolve_tool_auth(
            challenge
                .tool_name
                .clone()
                .unwrap_or_else(|| worker_runtime::worker_label(&state.run.actor_ref)),
        );
        adapter.persist_capability_store(&capability_state_ref, &capability_store)?;
        let parent_run_id = state.run.parent_run_id.clone().ok_or_else(|| {
            AppError::runtime("team subrun parent run id is unavailable for auth resume")
        })?;
        execute_team_subrun(
            adapter,
            session_id,
            &state.run.conversation_id,
            &parent_run_id,
            &state,
            Some(resolution_mediation_outcome),
        )
        .await?
    } else {
        TeamSubrunExecutionOutcome {
            state: resolved_subrun_auth_state(&state, &challenge, resolution, now),
            planner_events: Vec::new(),
            model_iterations: Vec::new(),
            capability_events: Vec::new(),
            runtime_error: None,
        }
    };

    let (mut run, mut conversation_id, mut project_id) = persist_team_subrun_state(
        adapter,
        session_id,
        &team_manifest,
        execution_outcome.state,
        now,
    )?;
    if let Some((updated_run, updated_conversation_id, updated_project_id)) =
        continue_team_runtime_subruns(adapter, session_id, &team_manifest).await?
    {
        run = updated_run;
        conversation_id = updated_conversation_id;
        project_id = updated_project_id;
    }

    Ok((
        resolved_challenge,
        None,
        None,
        run,
        conversation_id,
        project_id,
        execution_outcome.planner_events,
        execution_outcome.model_iterations,
        execution_outcome.capability_events,
        execution_outcome.runtime_error,
    ))
}

impl AgentRuntimeCore {
    pub(crate) async fn submit_turn(
        adapter: &RuntimeAdapter,
        session_id: &str,
        input: SubmitRuntimeTurnInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let now = timestamp_now();
        let mut run_context = adapter.build_run_context(session_id, &input, now).await?;
        let requires_approval = run_context
            .execution_policy_decision
            .as_ref()
            .map(|decision| decision.requires_approval)
            .unwrap_or(false);
        let provider_auth_required = !run_context
            .auth_state_summary
            .challenged_provider_keys
            .is_empty();
        let execution_mediation_request = runtime_execution_mediation_request(
            &run_context,
            &input.content,
            if requires_approval {
                "Turn requires approval".into()
            } else {
                "Execution allowed".into()
            },
            if requires_approval {
                format!(
                    "Permission mode {} requires explicit approval before execution.",
                    run_context.requested_permission_mode
                )
            } else {
                format!(
                    "Permission mode {} is within the frozen session ceiling.",
                    run_context.requested_permission_mode
                )
            },
            requires_approval,
            None,
            now,
        );
        let auth_mediation_request = (!requires_approval && provider_auth_required)
            .then(|| runtime_provider_auth_mediation_request(&run_context, None, now))
            .flatten();
        let mediation_request = auth_mediation_request
            .clone()
            .unwrap_or_else(|| execution_mediation_request.clone());
        let broker_decision = auth_mediation_request
            .as_ref()
            .map(approval_broker::mediate)
            .unwrap_or_else(|| approval_broker::mediate(&execution_mediation_request));
        let runtime_loop = if broker_decision.state == "allow" {
            let session = initial_runtime_session(&input.content)?;
            Some(
                execute_runtime_turn_loop_with_budget_reservation(
                    adapter,
                    &run_context.session_id,
                    &run_context.conversation_id,
                    &run_context.run_id,
                    &run_context.resolved_target,
                    &run_context.configured_model,
                    &run_context.actor_manifest,
                    &run_context.session_policy,
                    &run_context.requested_permission_mode,
                    &run_context.capability_state_ref,
                    &input.content,
                    &run_context.trace_context,
                    session,
                    Vec::new(),
                    0,
                    RuntimeUsageSummary::default(),
                )
                .await?,
            )
        } else {
            None
        };
        if let Some(runtime_loop) = runtime_loop.as_ref() {
            let capability_projection = match runtime_loop {
                RuntimeLoopExit::Completed(loop_result) => &loop_result.capability_projection,
                RuntimeLoopExit::Failed(loop_failure) => &loop_failure.capability_projection,
            };
            run_context.capability_plan_summary = capability_projection.plan_summary.clone();
            run_context.provider_state_summary =
                capability_projection.provider_state_summary.clone();
            run_context.auth_state_summary = capability_projection.auth_state_summary.clone();
            run_context.policy_decision_summary =
                capability_projection.policy_decision_summary.clone();
            run_context.capability_state_ref = capability_projection.capability_state_ref.clone();
        }
        let runtime_loop_completed = runtime_loop.as_ref().and_then(|step| match step {
            RuntimeLoopExit::Completed(loop_result) => Some(loop_result),
            RuntimeLoopExit::Failed(_) => None,
        });
        let runtime_loop_failed = runtime_loop.as_ref().and_then(|step| match step {
            RuntimeLoopExit::Failed(loop_failure) => Some(loop_failure),
            RuntimeLoopExit::Completed(_) => None,
        });
        let execution = runtime_loop_completed.map(|step| &step.response);
        let consumed_tokens = runtime_loop_completed.and_then(|step| step.consumed_tokens);
        let current_iteration_index = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.current_iteration_index,
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.current_iteration_index,
            })
            .unwrap_or(0);
        let usage_summary = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.usage_summary.clone(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.usage_summary.clone(),
            })
            .unwrap_or_default();
        let serialized_session = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.serialized_session.clone(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.serialized_session.clone(),
            })
            .unwrap_or_else(|| {
                serialized_runtime_session(
                    &input.content,
                    &run_context.trace_context,
                    &run_context.requested_permission_mode,
                )
            });
        let blocking_mediation_request = runtime_loop
            .as_ref()
            .and_then(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.mediation_request.as_ref(),
                RuntimeLoopExit::Failed(_) => None,
            })
            .unwrap_or(&mediation_request);
        let blocking_broker_decision = runtime_loop
            .as_ref()
            .and_then(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.broker_decision.clone(),
                RuntimeLoopExit::Failed(_) => None,
            })
            .unwrap_or(broker_decision);
        let team_spawn_mediation_request = (blocking_broker_decision.state == "allow")
            .then(|| team_spawn_mediation_request(&run_context, None, now))
            .flatten();
        let team_spawn_broker_decision = team_spawn_mediation_request
            .as_ref()
            .map(approval_broker::mediate);
        let blocking_mediation_request = team_spawn_mediation_request
            .as_ref()
            .unwrap_or(blocking_mediation_request);
        let blocking_broker_decision =
            team_spawn_broker_decision.unwrap_or(blocking_broker_decision);
        let workflow_continuation_mediation_request = (blocking_broker_decision.state == "allow")
            .then(|| {
                workflow_continuation_mediation_request(
                    &run_context.session_id,
                    &run_context.conversation_id,
                    &run_context.run_id,
                    &run_context.actor_manifest,
                    &run_context.session_policy,
                    None,
                    now,
                )
            })
            .flatten();
        let workflow_continuation_broker_decision = workflow_continuation_mediation_request
            .as_ref()
            .map(approval_broker::mediate);
        let blocking_mediation_request = workflow_continuation_mediation_request
            .as_ref()
            .unwrap_or(blocking_mediation_request);
        let blocking_broker_decision =
            workflow_continuation_broker_decision.unwrap_or(blocking_broker_decision);
        let empty_model_iterations = Vec::new();
        let model_iterations = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.model_iterations.as_slice(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.model_iterations.as_slice(),
            })
            .unwrap_or(empty_model_iterations.as_slice());
        let empty_planner_events = Vec::new();
        let planner_events = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.planner_events.as_slice(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.planner_events.as_slice(),
            })
            .unwrap_or(empty_planner_events.as_slice());
        let empty_capability_events = Vec::new();
        let capability_events = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.capability_events.as_slice(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.capability_events.as_slice(),
            })
            .unwrap_or(empty_capability_events.as_slice());

        let (
            user_message,
            submitted_trace,
            execution_trace,
            assistant_message,
            approval,
            mut run,
            mut conversation_id,
            mut project_id,
        ) = if let Some(loop_failure) = runtime_loop_failed {
            apply_submit_failure_state(
                adapter,
                &run_context,
                &input,
                current_iteration_index,
                usage_summary,
                serialized_session,
                &loop_failure.error,
            )?
        } else {
            apply_submit_state(
                adapter,
                &run_context,
                &input,
                blocking_mediation_request,
                blocking_broker_decision,
                execution,
                consumed_tokens,
                current_iteration_index,
                usage_summary,
                serialized_session,
            )?
        };
        let runtime_error = runtime_loop_failed.map(|failure| failure.error.as_str());
        if let actor_manifest::CompiledActorManifest::Team(team_manifest) =
            &run_context.actor_manifest
        {
            if let Some((updated_run, updated_conversation_id, updated_project_id)) =
                continue_team_runtime_subruns(adapter, session_id, team_manifest).await?
            {
                run = updated_run;
                conversation_id = updated_conversation_id;
                project_id = updated_project_id;
            }
        }

        execution_events::record_submit_turn_activity(
            adapter,
            session_id,
            now,
            &project_id,
            &run,
            &run_context.resolved_target,
            &submitted_trace,
            execution_trace.as_ref(),
            execution,
            consumed_tokens,
        )
        .await?;
        execution_events::emit_submit_turn_events(
            adapter,
            session_id,
            now,
            conversation_id,
            project_id,
            run.clone(),
            run_context.memory_selection.selection_summary.clone(),
            user_message,
            submitted_trace,
            assistant_message,
            execution_trace,
            approval,
            planner_events,
            model_iterations,
            capability_events,
            runtime_error,
        )
        .await?;

        Ok(run)
    }

    pub(crate) async fn resume_after_approval(
        adapter: &RuntimeAdapter,
        session_id: &str,
        approval_id: &str,
        input: ResolveRuntimeApprovalInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let now = timestamp_now();
        let decision_status = approval_flow::approval_decision_status(&input.decision)?;
        if load_pending_team_subrun_approval(adapter, session_id, approval_id)?.is_some() {
            let (
                approval,
                execution_trace,
                assistant_message,
                run,
                conversation_id,
                project_id,
                planner_events,
                model_iterations,
                capability_events,
                runtime_error,
            ) = resolve_team_subrun_approval(adapter, session_id, approval_id, decision_status)
                .await?;
            execution_events::record_approval_resolution_activity(
                adapter,
                session_id,
                now,
                &project_id,
                &run,
                &approval,
                &input.decision,
                execution_trace.as_ref(),
                None,
                None,
            )
            .await?;
            execution_events::emit_approval_resolution_events(
                adapter,
                session_id,
                now,
                conversation_id,
                project_id,
                run.clone(),
                approval,
                input.decision,
                assistant_message,
                execution_trace,
                planner_events.as_slice(),
                model_iterations.as_slice(),
                capability_events.as_slice(),
                runtime_error.as_deref(),
            )
            .await?;
            return Ok(run);
        }
        let (
            approval,
            actor_manifest,
            session_policy,
            checkpoint,
            checkpoint_serialized_session,
            resolved_target,
            configured_model,
            capability_state_ref,
        ) = load_pending_checkpoint(adapter, session_id, approval_id)?;
        let capability_store = adapter.load_capability_store(Some(&capability_state_ref))?;
        if decision_status == "approved"
            && approval.target_kind.as_deref() == Some("capability-call")
        {
            capability_store.approve_tool(approval.tool_name.clone());
        }
        let capability_projection = adapter
            .project_capability_state_async(
                &actor_manifest,
                &session_policy,
                &session_policy.config_snapshot_id,
                capability_state_ref,
                &capability_store,
            )
            .await?;

        let runtime_loop = if decision_status == "approved"
            && approval_replays_runtime_loop(approval.target_kind.as_deref())
            && !provider_auth_required(&capability_projection)
        {
            let content = checkpoint_serialized_session
                .get("content")
                .or_else(|| checkpoint_serialized_session.get("pendingContent"))
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::runtime("pending approval content is unavailable"))?;
            let session = restore_runtime_session(&checkpoint_serialized_session, content)?;
            let pending_tool_uses =
                pending_tool_uses_from_serialized_session(&checkpoint_serialized_session)?;
            let requested_permission_mode = requested_permission_mode_from_serialized_session(
                &checkpoint_serialized_session,
                &session_policy.execution_permission_mode,
            );
            Some(
                execute_runtime_turn_loop_with_budget_reservation(
                    adapter,
                    session_id,
                    &approval.conversation_id,
                    &approval.run_id,
                    &resolved_target,
                    &configured_model,
                    &actor_manifest,
                    &session_policy,
                    &requested_permission_mode,
                    &capability_projection.capability_state_ref,
                    content,
                    &trace_context_from_serialized_session(&checkpoint_serialized_session),
                    session,
                    pending_tool_uses,
                    checkpoint.current_iteration_index,
                    checkpoint.usage_summary.clone(),
                )
                .await?,
            )
        } else {
            None
        };
        let capability_projection = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => {
                    loop_result.capability_projection.clone()
                }
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.capability_projection.clone(),
            })
            .unwrap_or(capability_projection);
        let execution = runtime_loop.as_ref().and_then(|step| match step {
            RuntimeLoopExit::Completed(loop_result) => Some(&loop_result.response),
            RuntimeLoopExit::Failed(_) => None,
        });
        let consumed_tokens = runtime_loop.as_ref().and_then(|step| match step {
            RuntimeLoopExit::Completed(loop_result) => loop_result.consumed_tokens,
            RuntimeLoopExit::Failed(_) => None,
        });
        let runtime_error = runtime_loop.as_ref().and_then(|step| match step {
            RuntimeLoopExit::Completed(_) => None,
            RuntimeLoopExit::Failed(loop_failure) => Some(loop_failure.error.as_str()),
        });
        let current_iteration_index = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.current_iteration_index,
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.current_iteration_index,
            })
            .unwrap_or(checkpoint.current_iteration_index);
        let usage_summary = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.usage_summary.clone(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.usage_summary.clone(),
            })
            .unwrap_or_else(|| checkpoint.usage_summary.clone());
        let serialized_session = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.serialized_session.clone(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.serialized_session.clone(),
            })
            .unwrap_or_else(|| checkpoint_serialized_session.clone());
        let empty_model_iterations = Vec::new();
        let model_iterations = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.model_iterations.as_slice(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.model_iterations.as_slice(),
            })
            .unwrap_or(empty_model_iterations.as_slice());
        let empty_planner_events = Vec::new();
        let planner_events = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.planner_events.as_slice(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.planner_events.as_slice(),
            })
            .unwrap_or(empty_planner_events.as_slice());
        let empty_capability_events = Vec::new();
        let capability_events = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.capability_events.as_slice(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.capability_events.as_slice(),
            })
            .unwrap_or(empty_capability_events.as_slice());
        let (
            approval,
            execution_trace,
            assistant_message,
            mut run,
            mut conversation_id,
            mut project_id,
        ) = apply_approval_resolution_state(
            adapter,
            session_id,
            approval_id,
            now,
            decision_status,
            &actor_manifest,
            &session_policy,
            capability_projection,
            execution,
            consumed_tokens,
            current_iteration_index,
            usage_summary,
            serialized_session,
            runtime_error,
        )?;
        if decision_status == "approved" && runtime_error.is_none() {
            if let actor_manifest::CompiledActorManifest::Team(team_manifest) = &actor_manifest {
                if let Some((updated_run, updated_conversation_id, updated_project_id)) =
                    continue_team_runtime_subruns(adapter, session_id, team_manifest).await?
                {
                    run = updated_run;
                    conversation_id = updated_conversation_id;
                    project_id = updated_project_id;
                }
            }
        }

        execution_events::record_approval_resolution_activity(
            adapter,
            session_id,
            now,
            &project_id,
            &run,
            &approval,
            &input.decision,
            execution_trace.as_ref(),
            execution,
            consumed_tokens,
        )
        .await?;
        execution_events::emit_approval_resolution_events(
            adapter,
            session_id,
            now,
            conversation_id,
            project_id,
            run.clone(),
            approval,
            input.decision,
            assistant_message,
            execution_trace,
            planner_events,
            model_iterations,
            capability_events,
            runtime_error,
        )
        .await?;

        Ok(run)
    }

    pub(crate) async fn resolve_auth_challenge(
        adapter: &RuntimeAdapter,
        session_id: &str,
        challenge_id: &str,
        input: ResolveRuntimeAuthChallengeInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let now = timestamp_now();
        let resolution = approval_flow::auth_challenge_resolution_status(&input.resolution)?;
        if load_pending_team_subrun_auth_challenge(adapter, session_id, challenge_id)?.is_some() {
            let (
                challenge,
                execution_trace,
                assistant_message,
                run,
                conversation_id,
                project_id,
                planner_events,
                model_iterations,
                capability_events,
                runtime_error,
            ) = resolve_team_subrun_auth_challenge(adapter, session_id, challenge_id, resolution)
                .await?;
            execution_events::record_auth_challenge_resolution_activity(
                adapter,
                session_id,
                now,
                &project_id,
                &run,
                &challenge,
                &input.resolution,
                execution_trace.as_ref(),
                None,
                None,
            )
            .await?;
            execution_events::emit_auth_resolution_events(
                adapter,
                session_id,
                now,
                conversation_id,
                project_id,
                run.clone(),
                challenge,
                input.resolution,
                assistant_message,
                execution_trace,
                planner_events.as_slice(),
                model_iterations.as_slice(),
                capability_events.as_slice(),
                runtime_error.as_deref(),
            )
            .await?;
            return Ok(run);
        }
        let (
            challenge,
            actor_manifest,
            session_policy,
            checkpoint,
            checkpoint_serialized_session,
            resolved_target,
            configured_model,
            capability_state_ref,
        ) = load_pending_auth_checkpoint(adapter, session_id, challenge_id)?;
        let capability_store = adapter.load_capability_store(Some(&capability_state_ref))?;
        if resolution == "resolved" {
            capability_store.resolve_tool_auth(
                challenge
                    .tool_name
                    .clone()
                    .unwrap_or_else(|| actor_manifest.label().to_string()),
            );
        }
        let capability_projection = adapter
            .project_capability_state_async(
                &actor_manifest,
                &session_policy,
                &session_policy.config_snapshot_id,
                capability_state_ref,
                &capability_store,
            )
            .await?;

        let runtime_loop = if resolution == "resolved" {
            let content = checkpoint_serialized_session
                .get("content")
                .or_else(|| checkpoint_serialized_session.get("pendingContent"))
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::runtime("pending auth content is unavailable"))?;
            let session = restore_runtime_session(&checkpoint_serialized_session, content)?;
            let pending_tool_uses =
                pending_tool_uses_from_serialized_session(&checkpoint_serialized_session)?;
            let requested_permission_mode = requested_permission_mode_from_serialized_session(
                &checkpoint_serialized_session,
                &session_policy.execution_permission_mode,
            );
            Some(
                execute_runtime_turn_loop_with_budget_reservation(
                    adapter,
                    session_id,
                    &challenge.conversation_id,
                    &challenge.run_id,
                    &resolved_target,
                    &configured_model,
                    &actor_manifest,
                    &session_policy,
                    &requested_permission_mode,
                    &capability_projection.capability_state_ref,
                    content,
                    &trace_context_from_serialized_session(&checkpoint_serialized_session),
                    session,
                    pending_tool_uses,
                    checkpoint.current_iteration_index,
                    checkpoint.usage_summary.clone(),
                )
                .await?,
            )
        } else {
            None
        };
        let capability_projection = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => {
                    loop_result.capability_projection.clone()
                }
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.capability_projection.clone(),
            })
            .unwrap_or(capability_projection);
        let execution = runtime_loop.as_ref().and_then(|step| match step {
            RuntimeLoopExit::Completed(loop_result) => Some(&loop_result.response),
            RuntimeLoopExit::Failed(_) => None,
        });
        let consumed_tokens = runtime_loop.as_ref().and_then(|step| match step {
            RuntimeLoopExit::Completed(loop_result) => loop_result.consumed_tokens,
            RuntimeLoopExit::Failed(_) => None,
        });
        let runtime_error = runtime_loop.as_ref().and_then(|step| match step {
            RuntimeLoopExit::Completed(_) => None,
            RuntimeLoopExit::Failed(loop_failure) => Some(loop_failure.error.as_str()),
        });
        let current_iteration_index = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.current_iteration_index,
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.current_iteration_index,
            })
            .unwrap_or(checkpoint.current_iteration_index);
        let usage_summary = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.usage_summary.clone(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.usage_summary.clone(),
            })
            .unwrap_or_else(|| checkpoint.usage_summary.clone());
        let serialized_session = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.serialized_session.clone(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.serialized_session.clone(),
            })
            .unwrap_or_else(|| checkpoint_serialized_session.clone());
        let empty_model_iterations = Vec::new();
        let model_iterations = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.model_iterations.as_slice(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.model_iterations.as_slice(),
            })
            .unwrap_or(empty_model_iterations.as_slice());
        let empty_planner_events = Vec::new();
        let planner_events = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.planner_events.as_slice(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.planner_events.as_slice(),
            })
            .unwrap_or(empty_planner_events.as_slice());
        let empty_capability_events = Vec::new();
        let capability_events = runtime_loop
            .as_ref()
            .map(|step| match step {
                RuntimeLoopExit::Completed(loop_result) => loop_result.capability_events.as_slice(),
                RuntimeLoopExit::Failed(loop_failure) => loop_failure.capability_events.as_slice(),
            })
            .unwrap_or(empty_capability_events.as_slice());
        let (challenge, execution_trace, assistant_message, run, conversation_id, project_id) =
            apply_auth_challenge_resolution_state(
                adapter,
                session_id,
                challenge_id,
                now,
                resolution,
                &input,
                &actor_manifest,
                &session_policy,
                capability_projection,
                execution,
                consumed_tokens,
                current_iteration_index,
                usage_summary,
                serialized_session,
                runtime_error,
            )?;

        execution_events::record_auth_challenge_resolution_activity(
            adapter,
            session_id,
            now,
            &project_id,
            &run,
            &challenge,
            &input.resolution,
            execution_trace.as_ref(),
            execution,
            consumed_tokens,
        )
        .await?;
        execution_events::emit_auth_resolution_events(
            adapter,
            session_id,
            now,
            conversation_id,
            project_id,
            run.clone(),
            challenge,
            input.resolution,
            assistant_message,
            execution_trace,
            planner_events,
            model_iterations,
            capability_events,
            runtime_error,
        )
        .await?;

        Ok(run)
    }

    pub(crate) async fn cancel_subrun(
        adapter: &RuntimeAdapter,
        session_id: &str,
        subrun_id: &str,
        input: CancelRuntimeSubrunInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let now = timestamp_now();
        let note = input
            .note
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let team_manifest = load_session_team_manifest(adapter, session_id)?;
        let state = load_cancelable_team_subrun_state(adapter, session_id, subrun_id)?;
        let updated_state = cancelled_subrun_state(&state, note.as_deref(), now);

        let (mut run, mut conversation_id, mut project_id) =
            persist_team_subrun_state(adapter, session_id, &team_manifest, updated_state, now)?;
        if let Some((updated_run, updated_conversation_id, updated_project_id)) =
            continue_team_runtime_subruns(adapter, session_id, &team_manifest).await?
        {
            run = updated_run;
            conversation_id = updated_conversation_id;
            project_id = updated_project_id;
        }

        execution_events::record_subrun_cancellation_activity(
            adapter,
            session_id,
            now,
            &project_id,
            &run,
            subrun_id,
            note.as_deref(),
        )
        .await?;
        execution_events::emit_subrun_cancellation_events(
            adapter,
            session_id,
            now,
            conversation_id,
            project_id,
            run.clone(),
            subrun_id,
            note,
        )
        .await?;

        Ok(run)
    }

    pub(crate) async fn resolve_memory_proposal(
        adapter: &RuntimeAdapter,
        session_id: &str,
        proposal_id: &str,
        input: ResolveRuntimeMemoryProposalInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let now = timestamp_now();
        let decision_status = approval_flow::memory_proposal_decision_status(&input.decision)?;
        let (proposal, run, conversation_id, project_id) = apply_memory_proposal_resolution_state(
            adapter,
            session_id,
            proposal_id,
            now,
            decision_status,
            &input,
        )?;

        execution_events::record_memory_proposal_resolution_activity(
            adapter,
            session_id,
            now,
            &project_id,
            &run,
            &proposal,
            &input.decision,
        )
        .await?;
        execution_events::emit_memory_proposal_resolution_events(
            adapter,
            session_id,
            now,
            conversation_id,
            project_id,
            run.clone(),
            proposal,
            input.decision,
        )
        .await?;

        Ok(run)
    }
}

fn load_pending_checkpoint(
    adapter: &RuntimeAdapter,
    session_id: &str,
    approval_id: &str,
) -> Result<
    (
        ApprovalRequestRecord,
        actor_manifest::CompiledActorManifest,
        session_policy::CompiledSessionPolicy,
        RuntimeRunCheckpoint,
        Value,
        ResolvedExecutionTarget,
        ConfiguredModelRecord,
        String,
    ),
    AppError,
> {
    let (
        approval,
        session_policy_snapshot_ref,
        persisted_checkpoint,
        primary_run_serialized_session,
        configured_model_id,
        capability_state_ref,
    ) = {
        let sessions = adapter
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        let approval = aggregate
            .detail
            .pending_approval
            .clone()
            .ok_or_else(|| runtime_approval_lookup_error(aggregate, approval_id))?;
        if approval.id != approval_id {
            return Err(runtime_approval_lookup_error(aggregate, approval_id));
        }
        let checkpoint = aggregate.detail.run.checkpoint.clone();
        (
            approval,
            aggregate.metadata.session_policy_snapshot_ref.clone(),
            checkpoint.clone(),
            aggregate.metadata.primary_run_serialized_session.clone(),
            aggregate.detail.run.configured_model_id.clone(),
            aggregate
                .detail
                .run
                .capability_state_ref
                .clone()
                .or_else(|| aggregate.detail.capability_state_ref.clone())
                .unwrap_or_else(|| format!("{}-capability-state", aggregate.detail.run.id)),
        )
    };
    let (checkpoint, checkpoint_serialized_session) = if let Some(checkpoint_artifact) =
        adapter.load_runtime_artifact::<persistence::PersistedRuntimeCheckpointArtifact>(
            persisted_checkpoint.checkpoint_artifact_ref.as_deref(),
        )? {
        (
            checkpoint_artifact.checkpoint,
            checkpoint_artifact.serialized_session,
        )
    } else {
        (persisted_checkpoint, primary_run_serialized_session)
    };
    let capability_state_ref = checkpoint
        .capability_state_ref
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(capability_state_ref);
    let session_policy = adapter.load_session_policy_snapshot(&session_policy_snapshot_ref)?;
    let actor_manifest =
        adapter.load_actor_manifest_snapshot(&session_policy.manifest_snapshot_ref)?;
    let configured_model_id = configured_model_id
        .or_else(|| session_policy.selected_configured_model_id.clone())
        .filter(|configured_model_id| !configured_model_id.is_empty())
        .ok_or_else(|| AppError::runtime("configured model is unavailable for approval resume"))?;
    let (resolved_target, configured_model) = adapter
        .resolve_approved_execution(&session_policy.config_snapshot_id, &configured_model_id)?;
    if approval.status != "pending" {
        return Err(AppError::conflict(format!(
            "runtime approval `{approval_id}` is already {}",
            approval.status
        )));
    }
    if !resumable_approval_target(approval.target_kind.as_deref()) {
        return Err(AppError::invalid_input(format!(
            "runtime approval `{approval_id}` does not target a resumable checkpoint"
        )));
    }
    Ok((
        approval,
        actor_manifest,
        session_policy,
        checkpoint,
        checkpoint_serialized_session,
        resolved_target,
        configured_model,
        capability_state_ref,
    ))
}

fn load_pending_auth_checkpoint(
    adapter: &RuntimeAdapter,
    session_id: &str,
    challenge_id: &str,
) -> Result<
    (
        RuntimeAuthChallengeSummary,
        actor_manifest::CompiledActorManifest,
        session_policy::CompiledSessionPolicy,
        RuntimeRunCheckpoint,
        Value,
        ResolvedExecutionTarget,
        ConfiguredModelRecord,
        String,
    ),
    AppError,
> {
    let (
        challenge,
        session_policy_snapshot_ref,
        persisted_checkpoint,
        primary_run_serialized_session,
        configured_model_id,
        capability_state_ref,
    ) = {
        let sessions = adapter
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        let challenge = aggregate
            .detail
            .run
            .checkpoint
            .pending_auth_challenge
            .clone()
            .or_else(|| aggregate.detail.run.auth_target.clone())
            .ok_or_else(|| AppError::not_found("runtime auth challenge"))?;
        if challenge.id != challenge_id {
            return Err(AppError::not_found("runtime auth challenge"));
        }
        let checkpoint = aggregate.detail.run.checkpoint.clone();
        (
            challenge,
            aggregate.metadata.session_policy_snapshot_ref.clone(),
            checkpoint.clone(),
            aggregate.metadata.primary_run_serialized_session.clone(),
            aggregate.detail.run.configured_model_id.clone(),
            aggregate
                .detail
                .run
                .capability_state_ref
                .clone()
                .or_else(|| aggregate.detail.capability_state_ref.clone())
                .unwrap_or_else(|| format!("{}-capability-state", aggregate.detail.run.id)),
        )
    };
    let (checkpoint, checkpoint_serialized_session) = if let Some(checkpoint_artifact) =
        adapter.load_runtime_artifact::<persistence::PersistedRuntimeCheckpointArtifact>(
            persisted_checkpoint.checkpoint_artifact_ref.as_deref(),
        )? {
        (
            checkpoint_artifact.checkpoint,
            checkpoint_artifact.serialized_session,
        )
    } else {
        (persisted_checkpoint, primary_run_serialized_session)
    };
    let capability_state_ref = checkpoint
        .capability_state_ref
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(capability_state_ref);
    let session_policy = adapter.load_session_policy_snapshot(&session_policy_snapshot_ref)?;
    let actor_manifest =
        adapter.load_actor_manifest_snapshot(&session_policy.manifest_snapshot_ref)?;
    let configured_model_id = configured_model_id
        .or_else(|| session_policy.selected_configured_model_id.clone())
        .filter(|configured_model_id| !configured_model_id.is_empty())
        .ok_or_else(|| AppError::runtime("configured model is unavailable for auth resume"))?;
    let (resolved_target, configured_model) = adapter
        .resolve_approved_execution(&session_policy.config_snapshot_id, &configured_model_id)?;
    if challenge.status != "pending" {
        return Err(AppError::invalid_input(format!(
            "runtime auth challenge `{challenge_id}` is already {}",
            challenge.status
        )));
    }
    if !resumable_auth_target(&challenge.target_kind) {
        return Err(AppError::invalid_input(format!(
            "runtime auth challenge `{challenge_id}` does not target a resumable auth checkpoint"
        )));
    }
    Ok((
        challenge,
        actor_manifest,
        session_policy,
        checkpoint,
        checkpoint_serialized_session,
        resolved_target,
        configured_model,
        capability_state_ref,
    ))
}

fn apply_submit_failure_state(
    adapter: &RuntimeAdapter,
    run_context: &run_context::RunContext,
    input: &SubmitRuntimeTurnInput,
    current_iteration_index: u32,
    usage_summary: RuntimeUsageSummary,
    serialized_session: Value,
    runtime_error: &str,
) -> Result<SubmitState, AppError> {
    let mut sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    let aggregate = sessions
        .get_mut(&run_context.session_id)
        .ok_or_else(|| AppError::not_found("runtime session"))?;
    let actor_ref = run_context.actor_manifest.actor_ref().to_string();
    let actor_label = run_context.actor_manifest.label().to_string();
    let requested_actor_kind = Some(run_context.actor_manifest.actor_kind_label().to_string());
    let requested_actor_id = Some(actor_ref.clone());
    let last_execution_outcome = Some(interrupted_model_execution_outcome(
        &run_context.actor_manifest,
        &run_context.resolved_target,
        &run_context.run_id,
        runtime_error,
    ));

    let user_message = RuntimeMessage {
        id: format!("msg-{}", Uuid::new_v4()),
        session_id: run_context.session_id.clone(),
        conversation_id: run_context.conversation_id.clone(),
        sender_type: "user".into(),
        sender_label: "User".into(),
        content: input.content.clone(),
        timestamp: run_context.now,
        configured_model_id: Some(run_context.resolved_target.configured_model_id.clone()),
        configured_model_name: Some(run_context.resolved_target.configured_model_name.clone()),
        model_id: Some(run_context.resolved_target.registry_model_id.clone()),
        status: "failed".into(),
        requested_actor_kind: requested_actor_kind.clone(),
        requested_actor_id: requested_actor_id.clone(),
        resolved_actor_kind: requested_actor_kind.clone(),
        resolved_actor_id: requested_actor_id.clone(),
        resolved_actor_label: Some(actor_label.clone()),
        used_default_actor: Some(false),
        resource_ids: Some(Vec::new()),
        attachments: Some(Vec::new()),
        artifacts: Some(Vec::new()),
        deliverable_refs: None,
        usage: None,
        tool_calls: None,
        process_entries: None,
    };
    aggregate.detail.messages.push(user_message.clone());

    let submitted_trace = build_submit_trace(
        &run_context.session_id,
        &aggregate.detail.run.id,
        &run_context.conversation_id,
        &run_context.actor_manifest,
        run_context.now,
        format!(
            "Capability plan prepared for {}. Turn failed before assistant completion: {}.",
            actor_label, runtime_error
        ),
        "error",
    );
    aggregate.detail.trace.push(submitted_trace.clone());

    let mut checkpoint = build_runtime_checkpoint(
        current_iteration_index,
        usage_summary.clone(),
        None,
        None,
        None,
        Some(run_context.capability_state_ref.clone()),
        run_context.capability_plan_summary.clone(),
        last_execution_outcome.clone(),
        None,
        None,
        None,
        None,
    );
    let checkpoint_artifact =
        persistence::PersistedRuntimeCheckpointArtifact::from_public_checkpoint(
            checkpoint.clone(),
            serialized_session.clone(),
            json!({}),
        );
    let (storage_path, _) = adapter.persist_runtime_failed_checkpoint(
        &run_context.session_id,
        &run_context.run_id,
        &checkpoint_artifact,
    )?;
    checkpoint.checkpoint_artifact_ref = Some(storage_path.clone());

    aggregate.detail.run.checkpoint = checkpoint;
    aggregate.metadata.primary_run_serialized_session = serialized_session;

    aggregate.detail.summary.status = "failed".into();
    aggregate.detail.summary.updated_at = run_context.now;
    aggregate.detail.summary.last_message_preview = Some(input.content.clone());
    aggregate.detail.summary.active_run_id = aggregate.detail.run.id.clone();
    aggregate.detail.summary.capability_summary = run_context.capability_plan_summary.clone();
    aggregate.detail.summary.memory_summary = run_context.memory_selection.summary.clone();
    aggregate.detail.summary.memory_selection_summary =
        run_context.memory_selection.selection_summary.clone();
    aggregate.detail.summary.pending_memory_proposal_count = 0;
    aggregate.detail.summary.memory_state_ref =
        run_context.memory_selection.memory_state_ref.clone();
    aggregate.detail.summary.provider_state_summary = run_context.provider_state_summary.clone();
    aggregate.detail.summary.auth_state_summary = run_context.auth_state_summary.clone();
    aggregate.detail.summary.pending_mediation = None;
    aggregate.detail.summary.policy_decision_summary = run_context.policy_decision_summary.clone();
    aggregate.detail.summary.capability_state_ref = Some(run_context.capability_state_ref.clone());
    aggregate.detail.summary.last_execution_outcome = last_execution_outcome.clone();

    aggregate.detail.run.status = "failed".into();
    aggregate.detail.run.current_step = "failed".into();
    aggregate.detail.run.updated_at = run_context.now;
    aggregate.detail.run.configured_model_id =
        Some(run_context.resolved_target.configured_model_id.clone());
    aggregate.detail.run.configured_model_name =
        Some(run_context.resolved_target.configured_model_name.clone());
    aggregate.detail.run.model_id = Some(run_context.resolved_target.registry_model_id.clone());
    aggregate.detail.run.consumed_tokens = None;
    aggregate.detail.run.next_action = Some("idle".into());
    aggregate.detail.run.selected_memory = run_context.memory_selection.selected_memory.clone();
    aggregate.detail.run.freshness_summary =
        Some(run_context.memory_selection.freshness_summary.clone());
    aggregate.detail.run.pending_memory_proposal = None;
    aggregate.detail.run.memory_state_ref = run_context.memory_selection.memory_state_ref.clone();
    aggregate.detail.run.actor_ref = actor_ref.clone();
    aggregate.detail.run.approval_state = "not-required".into();
    aggregate.detail.run.approval_target = None;
    aggregate.detail.run.auth_target = None;
    aggregate.detail.run.usage_summary = usage_summary;
    aggregate.detail.run.artifact_refs = Vec::new();
    aggregate.detail.run.deliverable_refs = Vec::new();
    aggregate.detail.run.trace_context = run_context.trace_context.clone();
    aggregate.detail.run.capability_plan_summary = run_context.capability_plan_summary.clone();
    aggregate.detail.run.provider_state_summary = run_context.provider_state_summary.clone();
    aggregate.detail.run.pending_mediation = None;
    aggregate.detail.run.capability_state_ref = Some(run_context.capability_state_ref.clone());
    aggregate.detail.run.last_execution_outcome = last_execution_outcome.clone();
    aggregate.detail.run.last_mediation_outcome = None;
    aggregate.detail.pending_approval = None;
    aggregate.detail.run.resolved_target = Some(run_context.resolved_target.clone());
    aggregate.detail.run.requested_actor_kind = requested_actor_kind.clone();
    aggregate.detail.run.requested_actor_id = requested_actor_id.clone();
    aggregate.detail.run.resolved_actor_kind = requested_actor_kind;
    aggregate.detail.run.resolved_actor_id = requested_actor_id;
    aggregate.detail.run.resolved_actor_label = Some(actor_label);
    aggregate.detail.memory_summary = run_context.memory_selection.summary.clone();
    aggregate.detail.memory_selection_summary =
        run_context.memory_selection.selection_summary.clone();
    aggregate.detail.pending_memory_proposal_count = 0;
    aggregate.detail.memory_state_ref = run_context.memory_selection.memory_state_ref.clone();
    aggregate.detail.capability_summary = run_context.capability_plan_summary.clone();
    aggregate.detail.provider_state_summary = run_context.provider_state_summary.clone();
    aggregate.detail.auth_state_summary = run_context.auth_state_summary.clone();
    aggregate.detail.pending_mediation = None;
    aggregate.detail.policy_decision_summary = run_context.policy_decision_summary.clone();
    aggregate.detail.capability_state_ref = Some(run_context.capability_state_ref.clone());
    aggregate.detail.last_execution_outcome = last_execution_outcome;

    if let actor_manifest::CompiledActorManifest::Team(_team_manifest) = &run_context.actor_manifest
    {
        aggregate.detail.subruns.clear();
        aggregate.detail.subrun_count = 0;
        aggregate.detail.handoffs.clear();
        aggregate.detail.pending_mailbox = None;
        aggregate.detail.workflow = None;
        aggregate.detail.background_run = None;
        aggregate.detail.run.worker_dispatch = None;
        aggregate.detail.run.workflow_run = None;
        aggregate.detail.run.workflow_run_detail = None;
        aggregate.detail.run.mailbox_ref = None;
        aggregate.detail.run.handoff_ref = None;
        aggregate.detail.run.background_state = None;
        team_runtime::sync_subrun_state_metadata(aggregate, run_context.now);
    }
    sync_runtime_session_detail(&mut aggregate.detail);

    let run = aggregate.detail.run.clone();
    let conversation_id = aggregate.detail.summary.conversation_id.clone();
    let project_id = aggregate.detail.summary.project_id.clone();
    adapter.persist_runtime_projections(aggregate)?;

    Ok((
        user_message,
        submitted_trace,
        None,
        None,
        None,
        run,
        conversation_id,
        project_id,
    ))
}

fn persist_runtime_resolution_failure_checkpoint(
    adapter: &RuntimeAdapter,
    session_id: &str,
    run_id: &str,
    current_iteration_index: u32,
    usage_summary: RuntimeUsageSummary,
    capability_state_ref: &str,
    capability_plan_summary: RuntimeCapabilityPlanSummary,
    last_execution_outcome: Option<RuntimeCapabilityExecutionOutcome>,
    last_mediation_outcome: Option<RuntimeMediationOutcome>,
    serialized_session: Value,
) -> Result<RuntimeRunCheckpoint, AppError> {
    let mut checkpoint = apply_runtime_resolution_checkpoint(
        current_iteration_index,
        usage_summary,
        None,
        None,
        None,
        Some(capability_state_ref.to_string()),
        capability_plan_summary,
        last_execution_outcome,
        last_mediation_outcome,
    );
    let checkpoint_artifact =
        persistence::PersistedRuntimeCheckpointArtifact::from_public_checkpoint(
            checkpoint.clone(),
            serialized_session,
            json!({}),
        );
    let (storage_path, _) =
        adapter.persist_runtime_failed_checkpoint(session_id, run_id, &checkpoint_artifact)?;
    checkpoint.checkpoint_artifact_ref = Some(storage_path);
    Ok(checkpoint)
}

fn apply_submit_state(
    adapter: &RuntimeAdapter,
    run_context: &run_context::RunContext,
    input: &SubmitRuntimeTurnInput,
    mediation_request: &approval_broker::MediationRequest,
    broker_decision: approval_broker::BrokerDecision,
    execution: Option<&ModelExecutionResult>,
    consumed_tokens: Option<u32>,
    current_iteration_index: u32,
    usage_summary: RuntimeUsageSummary,
    serialized_session: Value,
) -> Result<SubmitState, AppError> {
    let mut sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    let aggregate = sessions
        .get_mut(&run_context.session_id)
        .ok_or_else(|| AppError::not_found("runtime session"))?;
    let actor_ref = run_context.actor_manifest.actor_ref().to_string();
    let actor_label = run_context.actor_manifest.label().to_string();
    let requested_actor_kind = Some(run_context.actor_manifest.actor_kind_label().to_string());
    let requested_actor_id = Some(actor_ref.clone());
    let mut approval = broker_decision.approval.clone();
    let mut auth_target = broker_decision.auth_challenge.clone();
    let blocking_pending_mediation = broker_decision.pending_mediation.clone();
    let pending_memory_proposal = memory_writer::build_memory_proposal(
        &run_context.session_id,
        &run_context.run_id,
        &aggregate.detail.summary.project_id,
        &memory_runtime::parse_memory_policy(&run_context.session_policy.memory_policy),
        &run_context.actor_manifest,
        input,
        execution,
        aggregate.detail.run.workflow_run_detail.as_ref(),
        &run_context.memory_selection.candidate_memory,
    );
    let memory_broker_decision = if blocking_pending_mediation.is_none() {
        pending_memory_proposal.as_ref().map(|proposal| {
            approval_broker::mediate(&memory_proposal_pending_mediation(run_context, proposal))
        })
    } else {
        None
    };
    let memory_pending_mediation = memory_broker_decision
        .as_ref()
        .and_then(|decision| decision.pending_mediation.clone());
    let mut pending_mediation = blocking_pending_mediation
        .clone()
        .or(memory_pending_mediation.clone());
    let last_execution_outcome = Some(broker_decision.execution_outcome.clone());
    let mut last_mediation_outcome = broker_decision.mediation_outcome.clone();
    let has_blocking_mediation = approval.is_some() || auth_target.is_some();
    let (run_status, current_step, next_action) =
        blocking_mediation_state(approval.as_ref(), auth_target.as_ref());
    let user_message = RuntimeMessage {
        id: format!("msg-{}", Uuid::new_v4()),
        session_id: run_context.session_id.clone(),
        conversation_id: run_context.conversation_id.clone(),
        sender_type: "user".into(),
        sender_label: "User".into(),
        content: input.content.clone(),
        timestamp: run_context.now,
        configured_model_id: Some(run_context.resolved_target.configured_model_id.clone()),
        configured_model_name: Some(run_context.resolved_target.configured_model_name.clone()),
        model_id: Some(run_context.resolved_target.registry_model_id.clone()),
        status: run_status.into(),
        requested_actor_kind: requested_actor_kind.clone(),
        requested_actor_id: requested_actor_id.clone(),
        resolved_actor_kind: requested_actor_kind.clone(),
        resolved_actor_id: requested_actor_id.clone(),
        resolved_actor_label: Some(actor_label.clone()),
        used_default_actor: Some(false),
        resource_ids: Some(Vec::new()),
        attachments: Some(Vec::new()),
        artifacts: Some(Vec::new()),
        deliverable_refs: None,
        usage: None,
        tool_calls: None,
        process_entries: None,
    };
    aggregate.detail.messages.push(user_message.clone());

    let submitted_trace = build_submit_trace(
        &run_context.session_id,
        &aggregate.detail.run.id,
        &run_context.conversation_id,
        &run_context.actor_manifest,
        run_context.now,
        if approval.is_some() {
            format!(
                "Capability plan prepared for {}. Turn suspended pending approval for permission mode {}.",
                actor_label, run_context.requested_permission_mode
            )
        } else if auth_target.is_some() {
            format!(
                "Capability plan prepared for {}. Turn suspended pending provider authentication.",
                actor_label
            )
        } else {
            format!(
                "Capability plan prepared for {}. Turn executing with permission mode {}.",
                actor_label, run_context.requested_permission_mode
            )
        },
        if has_blocking_mediation {
            "warning"
        } else {
            "success"
        },
    );
    aggregate.detail.trace.push(submitted_trace.clone());
    let assistant_message = execution.map(|response| {
        let message_id = format!("msg-{}", Uuid::new_v4());
        let deliverable_refs = register_pending_runtime_deliverables(
            aggregate,
            build_pending_runtime_deliverables(
                &adapter.state.workspace_id,
                &aggregate.detail.summary.project_id,
                &run_context.conversation_id,
                &run_context.session_id,
                &aggregate.detail.run.id,
                &aggregate.detail.summary.title,
                run_context.now,
                &response.deliverables,
                Some(&message_id),
            ),
        );
        RuntimeMessage {
            id: message_id,
            session_id: run_context.session_id.clone(),
            conversation_id: run_context.conversation_id.clone(),
            sender_type: "assistant".into(),
            sender_label: actor_label.clone(),
            content: response.content.clone(),
            timestamp: run_context.now,
            configured_model_id: Some(run_context.resolved_target.configured_model_id.clone()),
            configured_model_name: Some(run_context.resolved_target.configured_model_name.clone()),
            model_id: Some(run_context.resolved_target.registry_model_id.clone()),
            status: "completed".into(),
            requested_actor_kind: requested_actor_kind.clone(),
            requested_actor_id: requested_actor_id.clone(),
            resolved_actor_kind: requested_actor_kind.clone(),
            resolved_actor_id: requested_actor_id.clone(),
            resolved_actor_label: Some(actor_label.clone()),
            used_default_actor: Some(false),
            resource_ids: Some(Vec::new()),
            attachments: Some(Vec::new()),
            artifacts: Some(vec![persistence::runtime_output_artifact_ref(
                &aggregate.detail.run.id,
            )]),
            deliverable_refs: (!deliverable_refs.is_empty()).then_some(deliverable_refs),
            usage: None,
            tool_calls: None,
            process_entries: None,
        }
    });
    if let Some(message) = assistant_message.as_ref() {
        aggregate.detail.messages.push(message.clone());
    }

    let execution_trace = execution.map(|response| {
        build_execution_trace(
            &run_context.session_id,
            &aggregate.detail.run.id,
            &run_context.conversation_id,
            &run_context.actor_manifest,
            &run_context.resolved_target,
            response,
            run_context.now,
            assistant_message.as_ref().map(|message| message.id.clone()),
        )
    });
    if let Some(trace) = execution_trace.as_ref() {
        aggregate.detail.trace.push(trace.clone());
    }

    let checkpoint_artifact_ref = finalize_mediation_checkpoint_ref(
        adapter,
        &run_context.session_id,
        &run_context.run_id,
        &mut approval,
        &mut auth_target,
        &mut pending_mediation,
        &mut last_mediation_outcome,
    );
    let mut checkpoint = build_runtime_checkpoint(
        current_iteration_index,
        usage_summary.clone(),
        approval.clone(),
        auth_target.clone(),
        pending_mediation.clone(),
        Some(run_context.capability_state_ref.clone()),
        run_context.capability_plan_summary.clone(),
        last_execution_outcome.clone(),
        last_mediation_outcome.clone(),
        Some(mediation_request),
        Some(&broker_decision),
        checkpoint_artifact_ref.clone(),
    );
    if let Some(mediation_id) = pending_mediation
        .as_ref()
        .and_then(|mediation| mediation.mediation_id.as_deref())
    {
        let checkpoint_artifact =
            persistence::PersistedRuntimeCheckpointArtifact::from_public_checkpoint(
                checkpoint.clone(),
                serialized_session.clone(),
                json!({}),
            );
        let (storage_path, _) = adapter.persist_runtime_mediation_checkpoint(
            &run_context.session_id,
            &run_context.run_id,
            mediation_id,
            &checkpoint_artifact,
        )?;
        checkpoint.checkpoint_artifact_ref = Some(storage_path.clone());
        apply_checkpoint_ref(
            &mut approval,
            &mut auth_target,
            &mut pending_mediation,
            &mut last_mediation_outcome,
            &storage_path,
        );
        checkpoint.pending_approval = approval.clone();
        checkpoint.pending_auth_challenge = auth_target.clone();
        checkpoint.pending_mediation = pending_mediation.clone();
        checkpoint.last_mediation_outcome = last_mediation_outcome.clone();
    }
    aggregate.detail.run.checkpoint = checkpoint;
    aggregate.metadata.primary_run_serialized_session = serialized_session;

    aggregate.detail.summary.status = run_status.into();
    aggregate.detail.summary.updated_at = run_context.now;
    aggregate.detail.summary.last_message_preview = Some(
        assistant_message
            .as_ref()
            .map(|message| message.content.clone())
            .unwrap_or_else(|| input.content.clone()),
    );
    aggregate.detail.summary.active_run_id = aggregate.detail.run.id.clone();
    aggregate.detail.summary.capability_summary = run_context.capability_plan_summary.clone();
    aggregate.detail.summary.memory_summary = run_context.memory_selection.summary.clone();
    aggregate.detail.summary.memory_selection_summary =
        run_context.memory_selection.selection_summary.clone();
    aggregate.detail.summary.pending_memory_proposal_count =
        u64::from(pending_memory_proposal.is_some());
    aggregate.detail.summary.memory_state_ref =
        run_context.memory_selection.memory_state_ref.clone();
    aggregate.detail.summary.provider_state_summary = run_context.provider_state_summary.clone();
    aggregate.detail.summary.auth_state_summary = run_context.auth_state_summary.clone();
    aggregate.detail.summary.pending_mediation = pending_mediation.clone();
    aggregate.detail.summary.policy_decision_summary = run_context.policy_decision_summary.clone();
    aggregate.detail.summary.capability_state_ref = Some(run_context.capability_state_ref.clone());
    aggregate.detail.summary.last_execution_outcome = last_execution_outcome.clone();

    aggregate.detail.run.status = run_status.into();
    aggregate.detail.run.current_step = current_step.into();
    aggregate.detail.run.updated_at = run_context.now;
    aggregate.detail.run.configured_model_id =
        Some(run_context.resolved_target.configured_model_id.clone());
    aggregate.detail.run.configured_model_name =
        Some(run_context.resolved_target.configured_model_name.clone());
    aggregate.detail.run.model_id = Some(run_context.resolved_target.registry_model_id.clone());
    aggregate.detail.run.consumed_tokens = consumed_tokens;
    aggregate.detail.run.next_action = Some(next_action.into());
    aggregate.detail.run.selected_memory = run_context.memory_selection.selected_memory.clone();
    aggregate.detail.run.freshness_summary =
        Some(run_context.memory_selection.freshness_summary.clone());
    aggregate.detail.run.pending_memory_proposal = pending_memory_proposal.clone();
    aggregate.detail.run.memory_state_ref = run_context.memory_selection.memory_state_ref.clone();
    aggregate.detail.run.actor_ref = actor_ref.clone();
    aggregate.detail.run.approval_state = if approval.is_some() {
        "pending".into()
    } else if auth_target.is_some() {
        "auth-required".into()
    } else {
        "not-required".into()
    };
    aggregate.detail.run.approval_target = approval.clone();
    aggregate.detail.run.auth_target = auth_target.clone();
    aggregate.detail.run.usage_summary = usage_summary;
    aggregate.detail.run.artifact_refs = if execution.is_some() {
        vec![persistence::runtime_output_artifact_ref(
            &aggregate.detail.run.id,
        )]
    } else {
        Vec::new()
    };
    aggregate.detail.run.deliverable_refs = assistant_message
        .as_ref()
        .and_then(|message| message.deliverable_refs.clone())
        .unwrap_or_default();
    aggregate.detail.run.trace_context = run_context.trace_context.clone();
    aggregate.detail.run.capability_plan_summary = run_context.capability_plan_summary.clone();
    aggregate.detail.run.provider_state_summary = run_context.provider_state_summary.clone();
    aggregate.detail.run.pending_mediation = pending_mediation.clone();
    aggregate.detail.run.capability_state_ref = Some(run_context.capability_state_ref.clone());
    aggregate.detail.run.last_execution_outcome = last_execution_outcome.clone();
    aggregate.detail.run.last_mediation_outcome = last_mediation_outcome.clone();
    aggregate.detail.pending_approval = approval.clone();
    aggregate.detail.run.resolved_target = Some(run_context.resolved_target.clone());
    aggregate.detail.run.requested_actor_kind = requested_actor_kind.clone();
    aggregate.detail.run.requested_actor_id = requested_actor_id.clone();
    aggregate.detail.run.resolved_actor_kind = requested_actor_kind;
    aggregate.detail.run.resolved_actor_id = requested_actor_id;
    aggregate.detail.run.resolved_actor_label = Some(actor_label);
    aggregate.detail.memory_summary = run_context.memory_selection.summary.clone();
    aggregate.detail.memory_selection_summary =
        run_context.memory_selection.selection_summary.clone();
    aggregate.detail.pending_memory_proposal_count = u64::from(pending_memory_proposal.is_some());
    aggregate.detail.memory_state_ref = run_context.memory_selection.memory_state_ref.clone();
    aggregate.detail.capability_summary = run_context.capability_plan_summary.clone();
    aggregate.detail.provider_state_summary = run_context.provider_state_summary.clone();
    aggregate.detail.auth_state_summary = run_context.auth_state_summary.clone();
    aggregate.detail.pending_mediation = pending_mediation;
    aggregate.detail.policy_decision_summary = run_context.policy_decision_summary.clone();
    aggregate.detail.capability_state_ref = Some(run_context.capability_state_ref.clone());
    aggregate.detail.last_execution_outcome = last_execution_outcome;
    if let actor_manifest::CompiledActorManifest::Team(_team_manifest) = &run_context.actor_manifest
    {
        if approval_blocks_team_projection(approval.as_ref()) {
            aggregate.detail.subruns.clear();
            aggregate.detail.subrun_count = 0;
            aggregate.detail.handoffs.clear();
            aggregate.detail.pending_mailbox = None;
            aggregate.detail.workflow = None;
            aggregate.detail.background_run = None;
            aggregate.detail.run.worker_dispatch = None;
            aggregate.detail.run.workflow_run = None;
            aggregate.detail.run.workflow_run_detail = None;
            aggregate.detail.run.mailbox_ref = None;
            aggregate.detail.run.handoff_ref = None;
            aggregate.detail.run.background_state = None;
            team_runtime::sync_subrun_state_metadata(aggregate, run_context.now);
        } else {
            team_runtime::ensure_subrun_state_metadata(adapter, aggregate, run_context)?;
        }
    }
    sync_runtime_session_detail(&mut aggregate.detail);

    let run = aggregate.detail.run.clone();
    let conversation_id = aggregate.detail.summary.conversation_id.clone();
    let project_id = aggregate.detail.summary.project_id.clone();
    adapter.persist_runtime_projections(aggregate)?;

    Ok((
        user_message,
        submitted_trace,
        execution_trace,
        assistant_message,
        approval,
        run,
        conversation_id,
        project_id,
    ))
}

fn apply_approval_resolution_state(
    adapter: &RuntimeAdapter,
    session_id: &str,
    approval_id: &str,
    now: u64,
    decision_status: &str,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    session_policy: &session_policy::CompiledSessionPolicy,
    capability_projection: capability_planner_bridge::CapabilityProjection,
    execution: Option<&ModelExecutionResult>,
    consumed_tokens: Option<u32>,
    current_iteration_index: u32,
    usage_summary: RuntimeUsageSummary,
    serialized_session: Value,
    runtime_error: Option<&str>,
) -> Result<ApprovalResolutionState, AppError> {
    let mut sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    let aggregate = sessions
        .get_mut(session_id)
        .ok_or_else(|| AppError::not_found("runtime session"))?;
    if aggregate.detail.pending_approval.is_none() {
        return Err(runtime_approval_lookup_error(&*aggregate, approval_id));
    }
    if aggregate
        .detail
        .pending_approval
        .as_ref()
        .is_some_and(|pending| pending.id != approval_id)
    {
        return Err(runtime_approval_lookup_error(&*aggregate, approval_id));
    }
    let pending = aggregate
        .detail
        .pending_approval
        .as_mut()
        .expect("pending approval checked above");
    pending.status = decision_status.into();
    let mut approval = pending.clone();
    let resolved_target_kind = approval.target_kind.clone();
    let mut auth_target = None;
    let mut pending_mediation = None;
    let mut checkpoint_artifact_ref = aggregate
        .detail
        .run
        .checkpoint
        .checkpoint_artifact_ref
        .clone()
        .or_else(|| approval.checkpoint_ref.clone());
    let mut auth_state_summary = capability_projection.auth_state_summary.clone();
    let policy_decision_summary = capability_projection.policy_decision_summary.clone();
    let approved_requires_auth =
        decision_status == "approved" && provider_auth_required(&capability_projection);
    let mut last_execution_outcome = Some(if decision_status == "approved" {
        RuntimeCapabilityExecutionOutcome {
            capability_id: approval.capability_id.clone(),
            tool_name: Some(approval.tool_name.clone()),
            provider_key: approval.provider_key.clone(),
            dispatch_kind: approval.target_kind.clone(),
            outcome: "allow".into(),
            detail: None,
            requires_approval: approval.requires_approval,
            requires_auth: approval.requires_auth,
            concurrency_policy: Some("serialized".into()),
        }
    } else {
        RuntimeCapabilityExecutionOutcome {
            capability_id: approval.capability_id.clone(),
            tool_name: Some(approval.tool_name.clone()),
            provider_key: approval.provider_key.clone(),
            dispatch_kind: approval.target_kind.clone(),
            outcome: "deny".into(),
            detail: Some("approval request was rejected".into()),
            requires_approval: approval.requires_approval,
            requires_auth: approval.requires_auth,
            concurrency_policy: Some("serialized".into()),
        }
    });
    let mut last_mediation_outcome = Some(RuntimeMediationOutcome {
        approval_layer: approval.approval_layer.clone(),
        capability_id: approval.capability_id.clone(),
        checkpoint_ref: approval.checkpoint_ref.clone(),
        detail: Some(approval.detail.clone()),
        mediation_id: Some(approval.id.clone()),
        mediation_kind: "approval".into(),
        outcome: decision_status.into(),
        provider_key: approval.provider_key.clone(),
        reason: approval.escalation_reason.clone(),
        requires_approval: approval.requires_approval,
        requires_auth: approval.requires_auth,
        resolved_at: Some(now),
        target_kind: approval.target_kind.clone().unwrap_or_default(),
        target_ref: approval.target_ref.clone().unwrap_or_default(),
        tool_name: Some(approval.tool_name.clone()),
    });
    let mut checkpoint = None;
    if let Some(runtime_error) = runtime_error {
        let resolved_target = aggregate
            .detail
            .run
            .resolved_target
            .as_ref()
            .expect("resolved target must exist when approval resume fails");
        last_execution_outcome = Some(interrupted_model_execution_outcome(
            actor_manifest,
            resolved_target,
            &aggregate.detail.run.id,
            runtime_error,
        ));
    }

    if approved_requires_auth {
        let provider_auth_target = capability_projection
            .auth_state_summary
            .challenged_provider_keys
            .first()
            .cloned()
            .or_else(|| {
                capability_projection
                    .provider_state_summary
                    .iter()
                    .find(|provider| provider.state == "auth_required")
                    .map(|provider| provider.provider_key.clone())
            });
        let provider_auth_policy_decision = session_policy.target_decisions.get(&format!(
            "provider-auth:{}",
            provider_auth_target.clone().unwrap_or_default()
        ));
        let auth_request = provider_auth_mediation_request(
            session_id,
            &aggregate.detail.summary.conversation_id,
            &aggregate.detail.run.id,
            actor_manifest,
            &capability_projection.provider_state_summary,
            &capability_projection.auth_state_summary,
            provider_auth_policy_decision,
            None,
            now,
        )
        .ok_or_else(|| AppError::runtime("provider auth mediation request missing target"))?;
        let broker_decision = approval_broker::mediate(&auth_request);
        let mut checkpoint_approval = Some(approval.clone());

        auth_target = broker_decision.auth_challenge.clone();
        pending_mediation = broker_decision.pending_mediation.clone();
        last_execution_outcome = Some(broker_decision.execution_outcome.clone());
        last_mediation_outcome = broker_decision.mediation_outcome.clone();
        checkpoint_artifact_ref = finalize_mediation_checkpoint_ref(
            adapter,
            session_id,
            &aggregate.detail.run.id,
            &mut checkpoint_approval,
            &mut auth_target,
            &mut pending_mediation,
            &mut last_mediation_outcome,
        );

        let mut next_checkpoint = build_runtime_checkpoint(
            current_iteration_index,
            usage_summary.clone(),
            None,
            auth_target.clone(),
            pending_mediation.clone(),
            Some(capability_projection.capability_state_ref.clone()),
            capability_projection.plan_summary.clone(),
            last_execution_outcome.clone(),
            last_mediation_outcome.clone(),
            Some(&auth_request),
            Some(&broker_decision),
            checkpoint_artifact_ref.clone(),
        );
        if let Some(mediation_id) = pending_mediation
            .as_ref()
            .and_then(|mediation| mediation.mediation_id.as_deref())
        {
            let checkpoint_artifact =
                persistence::PersistedRuntimeCheckpointArtifact::from_public_checkpoint(
                    next_checkpoint.clone(),
                    serialized_session.clone(),
                    json!({}),
                );
            let (storage_path, _) = adapter.persist_runtime_mediation_checkpoint(
                session_id,
                &aggregate.detail.run.id,
                mediation_id,
                &checkpoint_artifact,
            )?;
            next_checkpoint.checkpoint_artifact_ref = Some(storage_path.clone());
            apply_checkpoint_ref(
                &mut checkpoint_approval,
                &mut auth_target,
                &mut pending_mediation,
                &mut last_mediation_outcome,
                &storage_path,
            );
            next_checkpoint.pending_auth_challenge = auth_target.clone();
            next_checkpoint.pending_mediation = pending_mediation.clone();
            next_checkpoint.last_mediation_outcome = last_mediation_outcome.clone();
        }
        if let Some(next_approval) = checkpoint_approval {
            approval = next_approval;
        }
        if let Some(challenge) = auth_target.as_ref() {
            if let Some(provider_key) = challenge.provider_key.clone() {
                if !auth_state_summary
                    .challenged_provider_keys
                    .contains(&provider_key)
                {
                    auth_state_summary
                        .challenged_provider_keys
                        .push(provider_key);
                }
            }
            auth_state_summary.last_challenge_at = Some(challenge.created_at);
            auth_state_summary.pending_challenge_count =
                auth_state_summary.challenged_provider_keys.len() as u64;
        }
        checkpoint = Some(next_checkpoint);
    }

    if decision_status == "approved"
        && resolved_target_kind.as_deref() == Some("team-spawn")
        && !approved_requires_auth
    {
        if let Some(workflow_request) = workflow_continuation_mediation_request(
            session_id,
            &aggregate.detail.summary.conversation_id,
            &aggregate.detail.run.id,
            actor_manifest,
            session_policy,
            None,
            now,
        ) {
            let workflow_broker_decision = approval_broker::mediate(&workflow_request);
            let mut next_approval = workflow_broker_decision.approval.clone();
            let mut next_pending_mediation = workflow_broker_decision.pending_mediation.clone();
            let mut next_mediation_outcome = workflow_broker_decision.mediation_outcome.clone();
            let mut next_auth_target = None;
            let next_checkpoint_artifact_ref = finalize_mediation_checkpoint_ref(
                adapter,
                session_id,
                &aggregate.detail.run.id,
                &mut next_approval,
                &mut next_auth_target,
                &mut next_pending_mediation,
                &mut next_mediation_outcome,
            );
            let mut next_checkpoint = build_runtime_checkpoint(
                current_iteration_index,
                usage_summary.clone(),
                next_approval.clone(),
                None,
                next_pending_mediation.clone(),
                Some(capability_projection.capability_state_ref.clone()),
                capability_projection.plan_summary.clone(),
                Some(workflow_broker_decision.execution_outcome.clone()),
                next_mediation_outcome.clone(),
                Some(&workflow_request),
                Some(&workflow_broker_decision),
                next_checkpoint_artifact_ref.clone(),
            );
            if let Some(mediation_id) = next_pending_mediation
                .as_ref()
                .and_then(|mediation| mediation.mediation_id.as_deref())
            {
                let checkpoint_artifact =
                    persistence::PersistedRuntimeCheckpointArtifact::from_public_checkpoint(
                        next_checkpoint.clone(),
                        serialized_session.clone(),
                        json!({}),
                    );
                let (storage_path, _) = adapter.persist_runtime_mediation_checkpoint(
                    session_id,
                    &aggregate.detail.run.id,
                    mediation_id,
                    &checkpoint_artifact,
                )?;
                next_checkpoint.checkpoint_artifact_ref = Some(storage_path.clone());
                apply_checkpoint_ref(
                    &mut next_approval,
                    &mut next_auth_target,
                    &mut next_pending_mediation,
                    &mut next_mediation_outcome,
                    &storage_path,
                );
                next_checkpoint.pending_approval = next_approval.clone();
                next_checkpoint.pending_mediation = next_pending_mediation.clone();
                next_checkpoint.last_mediation_outcome = next_mediation_outcome.clone();
            }

            approval = next_approval
                .ok_or_else(|| AppError::runtime("workflow continuation approval missing"))?;
            pending_mediation = next_pending_mediation;
            last_execution_outcome = Some(workflow_broker_decision.execution_outcome);
            last_mediation_outcome = next_mediation_outcome;
            checkpoint = Some(next_checkpoint);
        }
    }

    let assistant_message = execution.map(|response| {
        let message_id = format!("msg-{}", Uuid::new_v4());
        let deliverable_refs = register_pending_runtime_deliverables(
            aggregate,
            build_pending_runtime_deliverables(
                &adapter.state.workspace_id,
                &aggregate.detail.summary.project_id,
                &aggregate.detail.summary.conversation_id,
                session_id,
                &aggregate.detail.run.id,
                &aggregate.detail.summary.title,
                now,
                &response.deliverables,
                Some(&message_id),
            ),
        );
        RuntimeMessage {
            id: message_id,
            session_id: session_id.to_string(),
            conversation_id: aggregate.detail.summary.conversation_id.clone(),
            sender_type: "assistant".into(),
            sender_label: actor_manifest.label().to_string(),
            content: response.content.clone(),
            timestamp: now,
            configured_model_id: aggregate.detail.run.configured_model_id.clone(),
            configured_model_name: aggregate.detail.run.configured_model_name.clone(),
            model_id: aggregate.detail.run.model_id.clone(),
            status: "completed".into(),
            requested_actor_kind: aggregate.detail.run.requested_actor_kind.clone(),
            requested_actor_id: aggregate.detail.run.requested_actor_id.clone(),
            resolved_actor_kind: aggregate.detail.run.resolved_actor_kind.clone(),
            resolved_actor_id: aggregate.detail.run.resolved_actor_id.clone(),
            resolved_actor_label: aggregate.detail.run.resolved_actor_label.clone(),
            used_default_actor: Some(false),
            resource_ids: Some(Vec::new()),
            attachments: Some(Vec::new()),
            artifacts: Some(vec![persistence::runtime_output_artifact_ref(
                &aggregate.detail.run.id,
            )]),
            deliverable_refs: (!deliverable_refs.is_empty()).then_some(deliverable_refs),
            usage: None,
            tool_calls: None,
            process_entries: None,
        }
    });
    if let Some(message) = assistant_message.as_ref() {
        aggregate.detail.messages.push(message.clone());
    }

    let execution_trace = execution.map(|response| {
        build_execution_trace(
            session_id,
            &aggregate.detail.run.id,
            &aggregate.detail.summary.conversation_id,
            actor_manifest,
            aggregate
                .detail
                .run
                .resolved_target
                .as_ref()
                .expect("resolved target must exist when approval resumes"),
            response,
            now,
            assistant_message.as_ref().map(|message| message.id.clone()),
        )
    });
    if let Some(trace) = execution_trace.as_ref() {
        aggregate.detail.trace.push(trace.clone());
    }

    let chained_approval_pending = decision_status == "approved" && approval.id != approval_id;
    let (run_status, current_step, next_action) = if runtime_error.is_some() {
        ("failed", "failed", "idle")
    } else if chained_approval_pending {
        blocking_mediation_state(Some(&approval), auth_target.as_ref())
    } else if approved_requires_auth {
        ("waiting_input", "awaiting_auth", "auth")
    } else if decision_status == "approved" {
        ("completed", "completed", "idle")
    } else {
        ("blocked", "approval_rejected", "blocked")
    };
    if let Some(message) = aggregate
        .detail
        .messages
        .iter_mut()
        .rev()
        .find(|message| message.sender_type == "user" && message.status == "waiting_approval")
    {
        message.status = run_status.into();
    }

    let approval_state = if run_status == "waiting_approval" {
        "pending"
    } else if approved_requires_auth {
        "auth-required"
    } else {
        decision_status
    };

    aggregate.detail.run.status = run_status.into();
    aggregate.detail.run.current_step = current_step.into();
    aggregate.detail.run.updated_at = now;
    aggregate.detail.run.consumed_tokens = consumed_tokens;
    aggregate.detail.run.next_action = Some(next_action.into());
    aggregate.detail.run.approval_state = approval_state.into();
    aggregate.detail.run.approval_target = if run_status == "waiting_approval" {
        Some(approval.clone())
    } else {
        None
    };
    aggregate.detail.run.auth_target = auth_target.clone();
    aggregate.detail.run.usage_summary = usage_summary.clone();
    aggregate.detail.run.artifact_refs = if execution.is_some() {
        vec![persistence::runtime_output_artifact_ref(
            &aggregate.detail.run.id,
        )]
    } else {
        Vec::new()
    };
    aggregate.detail.run.deliverable_refs = assistant_message
        .as_ref()
        .and_then(|message| message.deliverable_refs.clone())
        .unwrap_or_default();
    aggregate.detail.run.checkpoint = if runtime_error.is_some() {
        persist_runtime_resolution_failure_checkpoint(
            adapter,
            session_id,
            &aggregate.detail.run.id,
            current_iteration_index,
            usage_summary.clone(),
            &capability_projection.capability_state_ref,
            capability_projection.plan_summary.clone(),
            last_execution_outcome.clone(),
            last_mediation_outcome.clone(),
            serialized_session.clone(),
        )?
    } else if let Some(checkpoint) = checkpoint {
        checkpoint
    } else {
        let mut checkpoint = apply_runtime_resolution_checkpoint(
            current_iteration_index,
            usage_summary.clone(),
            None,
            None,
            pending_mediation.clone(),
            Some(capability_projection.capability_state_ref.clone()),
            capability_projection.plan_summary.clone(),
            last_execution_outcome.clone(),
            last_mediation_outcome.clone(),
        );
        checkpoint.approval_layer = approval.approval_layer.clone();
        checkpoint.broker_decision = Some(decision_status.into());
        checkpoint.capability_id = approval.capability_id.clone();
        checkpoint.checkpoint_artifact_ref = checkpoint_artifact_ref;
        checkpoint.provider_key = approval.provider_key.clone();
        checkpoint.reason = approval.escalation_reason.clone();
        checkpoint.required_permission = approval.required_permission.clone();
        checkpoint.requires_approval = Some(approval.requires_approval);
        checkpoint.requires_auth = Some(approval.requires_auth);
        checkpoint.target_kind = approval.target_kind.clone();
        checkpoint.target_ref = approval.target_ref.clone();
        checkpoint
    };
    aggregate.metadata.primary_run_serialized_session = serialized_session;

    aggregate.detail.summary.status = aggregate.detail.run.status.clone();
    aggregate.detail.summary.updated_at = now;
    aggregate.detail.summary.last_message_preview = Some(
        assistant_message
            .as_ref()
            .map(|message| message.content.clone())
            .or_else(|| {
                aggregate
                    .detail
                    .messages
                    .iter()
                    .rev()
                    .find(|message| message.sender_type == "user")
                    .map(|message| message.content.clone())
            })
            .unwrap_or_default(),
    );
    aggregate.detail.summary.session_policy = session_policy.contract_snapshot();
    aggregate.detail.summary.capability_summary = capability_projection.plan_summary.clone();
    aggregate.detail.summary.memory_summary = aggregate.detail.memory_summary.clone();
    aggregate.detail.summary.memory_selection_summary =
        aggregate.detail.memory_selection_summary.clone();
    aggregate.detail.summary.pending_memory_proposal_count =
        aggregate.detail.pending_memory_proposal_count;
    aggregate.detail.summary.memory_state_ref = aggregate.detail.memory_state_ref.clone();
    aggregate.detail.summary.provider_state_summary =
        capability_projection.provider_state_summary.clone();
    aggregate.detail.summary.auth_state_summary = auth_state_summary.clone();
    aggregate.detail.summary.pending_mediation = pending_mediation.clone();
    aggregate.detail.summary.policy_decision_summary = policy_decision_summary.clone();
    aggregate.detail.summary.capability_state_ref =
        Some(capability_projection.capability_state_ref.clone());
    aggregate.detail.summary.last_execution_outcome = last_execution_outcome.clone();

    aggregate.detail.pending_approval = if run_status == "waiting_approval" {
        Some(approval.clone())
    } else {
        None
    };
    aggregate.detail.run.capability_plan_summary = capability_projection.plan_summary.clone();
    aggregate.detail.run.provider_state_summary =
        capability_projection.provider_state_summary.clone();
    aggregate.detail.run.pending_mediation = pending_mediation.clone();
    aggregate.detail.run.capability_state_ref = Some(capability_projection.capability_state_ref);
    aggregate.detail.run.last_execution_outcome = last_execution_outcome.clone();
    aggregate.detail.run.last_mediation_outcome = last_mediation_outcome.clone();
    aggregate.detail.capability_summary = capability_projection.plan_summary;
    aggregate.detail.provider_state_summary = capability_projection.provider_state_summary;
    aggregate.detail.auth_state_summary = auth_state_summary;
    aggregate.detail.pending_mediation = pending_mediation;
    aggregate.detail.policy_decision_summary = policy_decision_summary;
    aggregate.detail.capability_state_ref = aggregate.detail.run.capability_state_ref.clone();
    aggregate.detail.last_execution_outcome = last_execution_outcome;
    if let actor_manifest::CompiledActorManifest::Team(team_manifest) = actor_manifest {
        if resolved_target_kind.as_deref() == Some("team-spawn") && decision_status != "approved" {
            aggregate.detail.subruns.clear();
            aggregate.detail.subrun_count = 0;
            aggregate.detail.handoffs.clear();
            aggregate.detail.pending_mailbox = None;
            aggregate.detail.workflow = None;
            aggregate.detail.background_run = None;
            aggregate.detail.run.worker_dispatch = None;
            aggregate.detail.run.workflow_run = None;
            aggregate.detail.run.workflow_run_detail = None;
            aggregate.detail.run.mailbox_ref = None;
            aggregate.detail.run.handoff_ref = None;
            aggregate.detail.run.background_state = None;
            team_runtime::sync_subrun_state_metadata(aggregate, now);
        } else {
            team_runtime::apply_team_runtime_state(
                &mut aggregate.detail,
                team_manifest,
                &aggregate.metadata.subrun_states,
                now,
            );
            team_runtime::ensure_subrun_state_metadata_for_session(
                adapter,
                aggregate,
                team_manifest,
                session_policy,
                now,
            )?;
        }
    }
    sync_runtime_session_detail(&mut aggregate.detail);
    let run = aggregate.detail.run.clone();
    let conversation_id = aggregate.detail.summary.conversation_id.clone();
    let project_id = aggregate.detail.summary.project_id.clone();
    adapter.persist_runtime_projections(aggregate)?;

    Ok((
        approval,
        execution_trace,
        assistant_message,
        run,
        conversation_id,
        project_id,
    ))
}

#[allow(clippy::too_many_arguments)]
fn apply_auth_challenge_resolution_state(
    adapter: &RuntimeAdapter,
    session_id: &str,
    challenge_id: &str,
    now: u64,
    resolution: &str,
    input: &ResolveRuntimeAuthChallengeInput,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    session_policy: &session_policy::CompiledSessionPolicy,
    capability_projection: capability_planner_bridge::CapabilityProjection,
    execution: Option<&ModelExecutionResult>,
    consumed_tokens: Option<u32>,
    current_iteration_index: u32,
    usage_summary: RuntimeUsageSummary,
    serialized_session: Value,
    runtime_error: Option<&str>,
) -> Result<AuthChallengeResolutionState, AppError> {
    let mut sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    let aggregate = sessions
        .get_mut(session_id)
        .ok_or_else(|| AppError::not_found("runtime session"))?;
    let pending = aggregate
        .detail
        .run
        .checkpoint
        .pending_auth_challenge
        .as_mut()
        .ok_or_else(|| AppError::not_found("runtime auth challenge"))?;
    if pending.id != challenge_id {
        return Err(AppError::not_found("runtime auth challenge"));
    }
    pending.status = resolution.into();
    pending.resolution = Some(input.resolution.clone());
    let challenge = pending.clone();
    let pending_mediation = None;
    let mut last_execution_outcome = Some(if resolution == "resolved" {
        capability_execution_outcome("allow", None, false, false)
    } else {
        capability_execution_outcome(
            "deny",
            Some(format!("auth challenge ended with status `{resolution}`")),
            false,
            true,
        )
    });
    let last_mediation_outcome = Some(RuntimeMediationOutcome {
        approval_layer: Some(challenge.approval_layer.clone()),
        capability_id: challenge.capability_id.clone(),
        checkpoint_ref: challenge.checkpoint_ref.clone(),
        detail: Some(challenge.detail.clone()),
        mediation_id: Some(challenge.id.clone()),
        mediation_kind: "auth".into(),
        outcome: resolution.into(),
        provider_key: challenge.provider_key.clone(),
        reason: Some(challenge.escalation_reason.clone()),
        requires_approval: challenge.requires_approval,
        requires_auth: challenge.requires_auth,
        resolved_at: Some(now),
        target_kind: challenge.target_kind.clone(),
        target_ref: challenge.target_ref.clone(),
        tool_name: challenge.tool_name.clone(),
    });
    if let Some(runtime_error) = runtime_error {
        let resolved_target = aggregate
            .detail
            .run
            .resolved_target
            .as_ref()
            .expect("resolved target must exist when auth resume fails");
        last_execution_outcome = Some(interrupted_model_execution_outcome(
            actor_manifest,
            resolved_target,
            &aggregate.detail.run.id,
            runtime_error,
        ));
    }

    let assistant_message = execution.map(|response| {
        let message_id = format!("msg-{}", Uuid::new_v4());
        let deliverable_refs = register_pending_runtime_deliverables(
            aggregate,
            build_pending_runtime_deliverables(
                &adapter.state.workspace_id,
                &aggregate.detail.summary.project_id,
                &aggregate.detail.summary.conversation_id,
                session_id,
                &aggregate.detail.run.id,
                &aggregate.detail.summary.title,
                now,
                &response.deliverables,
                Some(&message_id),
            ),
        );
        RuntimeMessage {
            id: message_id,
            session_id: session_id.to_string(),
            conversation_id: aggregate.detail.summary.conversation_id.clone(),
            sender_type: "assistant".into(),
            sender_label: actor_manifest.label().to_string(),
            content: response.content.clone(),
            timestamp: now,
            configured_model_id: aggregate.detail.run.configured_model_id.clone(),
            configured_model_name: aggregate.detail.run.configured_model_name.clone(),
            model_id: aggregate.detail.run.model_id.clone(),
            status: "completed".into(),
            requested_actor_kind: aggregate.detail.run.requested_actor_kind.clone(),
            requested_actor_id: aggregate.detail.run.requested_actor_id.clone(),
            resolved_actor_kind: aggregate.detail.run.resolved_actor_kind.clone(),
            resolved_actor_id: aggregate.detail.run.resolved_actor_id.clone(),
            resolved_actor_label: aggregate.detail.run.resolved_actor_label.clone(),
            used_default_actor: Some(false),
            resource_ids: Some(Vec::new()),
            attachments: Some(Vec::new()),
            artifacts: Some(vec![persistence::runtime_output_artifact_ref(
                &aggregate.detail.run.id,
            )]),
            deliverable_refs: (!deliverable_refs.is_empty()).then_some(deliverable_refs),
            usage: None,
            tool_calls: None,
            process_entries: None,
        }
    });
    if let Some(message) = assistant_message.as_ref() {
        aggregate.detail.messages.push(message.clone());
    }

    let execution_trace = execution.map(|response| {
        build_execution_trace(
            session_id,
            &aggregate.detail.run.id,
            &aggregate.detail.summary.conversation_id,
            actor_manifest,
            aggregate
                .detail
                .run
                .resolved_target
                .as_ref()
                .expect("resolved target must exist when auth challenge resumes"),
            response,
            now,
            assistant_message.as_ref().map(|message| message.id.clone()),
        )
    });
    if let Some(trace) = execution_trace.as_ref() {
        aggregate.detail.trace.push(trace.clone());
    }

    aggregate.detail.run.status = if runtime_error.is_some() {
        "failed".into()
    } else if resolution == "resolved" {
        "completed".into()
    } else {
        "blocked".into()
    };
    aggregate.detail.run.current_step = if runtime_error.is_some() {
        "failed".into()
    } else if resolution == "resolved" {
        "completed".into()
    } else {
        "auth_challenge_blocked".into()
    };
    aggregate.detail.run.updated_at = now;
    aggregate.detail.run.consumed_tokens = consumed_tokens;
    aggregate.detail.run.next_action = Some(if runtime_error.is_some() || resolution == "resolved" {
        "idle".into()
    } else {
        "blocked".into()
    });
    aggregate.detail.run.approval_state = resolution.into();
    aggregate.detail.run.auth_target = None;
    aggregate.detail.run.usage_summary = usage_summary.clone();
    aggregate.detail.run.artifact_refs = if execution.is_some() {
        vec![persistence::runtime_output_artifact_ref(
            &aggregate.detail.run.id,
        )]
    } else {
        Vec::new()
    };
    aggregate.detail.run.deliverable_refs = assistant_message
        .as_ref()
        .and_then(|message| message.deliverable_refs.clone())
        .unwrap_or_default();
    if runtime_error.is_some() {
        aggregate.detail.run.checkpoint = persist_runtime_resolution_failure_checkpoint(
            adapter,
            session_id,
            &aggregate.detail.run.id,
            current_iteration_index,
            usage_summary.clone(),
            &capability_projection.capability_state_ref,
            capability_projection.plan_summary.clone(),
            last_execution_outcome.clone(),
            last_mediation_outcome.clone(),
            serialized_session.clone(),
        )?;
    } else {
        aggregate.detail.run.checkpoint.pending_auth_challenge = None;
        aggregate.detail.run.checkpoint.current_iteration_index = current_iteration_index;
        aggregate.detail.run.checkpoint.usage_summary = usage_summary.clone();
        aggregate.detail.run.checkpoint.pending_mediation = pending_mediation.clone();
        aggregate.detail.run.checkpoint.capability_state_ref =
            Some(capability_projection.capability_state_ref.clone());
        aggregate.detail.run.checkpoint.capability_plan_summary =
            capability_projection.plan_summary.clone();
        aggregate.detail.run.checkpoint.last_execution_outcome = last_execution_outcome.clone();
        aggregate.detail.run.checkpoint.last_mediation_outcome = last_mediation_outcome.clone();
    }
    aggregate.metadata.primary_run_serialized_session = serialized_session;

    aggregate.detail.summary.status = aggregate.detail.run.status.clone();
    aggregate.detail.summary.updated_at = now;
    aggregate.detail.summary.last_message_preview = Some(
        assistant_message
            .as_ref()
            .map(|message| message.content.clone())
            .or_else(|| {
                aggregate
                    .detail
                    .messages
                    .iter()
                    .rev()
                    .find(|message| message.sender_type == "user")
                    .map(|message| message.content.clone())
            })
            .unwrap_or_default(),
    );
    aggregate.detail.summary.session_policy = session_policy.contract_snapshot();
    aggregate.detail.summary.capability_summary = capability_projection.plan_summary.clone();
    aggregate.detail.summary.memory_summary = aggregate.detail.memory_summary.clone();
    aggregate.detail.summary.memory_selection_summary =
        aggregate.detail.memory_selection_summary.clone();
    aggregate.detail.summary.pending_memory_proposal_count =
        aggregate.detail.pending_memory_proposal_count;
    aggregate.detail.summary.memory_state_ref = aggregate.detail.memory_state_ref.clone();
    aggregate.detail.summary.provider_state_summary =
        capability_projection.provider_state_summary.clone();
    aggregate.detail.summary.pending_mediation = pending_mediation.clone();
    aggregate.detail.summary.capability_state_ref =
        Some(capability_projection.capability_state_ref.clone());
    aggregate.detail.summary.last_execution_outcome = last_execution_outcome.clone();
    aggregate
        .detail
        .summary
        .auth_state_summary
        .last_challenge_at = Some(now);
    if let Some(provider_key) = challenge.provider_key.clone() {
        aggregate
            .detail
            .summary
            .auth_state_summary
            .challenged_provider_keys
            .retain(|value| value != &provider_key);
        aggregate
            .detail
            .summary
            .auth_state_summary
            .pending_challenge_count = aggregate
            .detail
            .summary
            .auth_state_summary
            .challenged_provider_keys
            .len() as u64;
        if resolution == "resolved" {
            if !aggregate
                .detail
                .summary
                .auth_state_summary
                .resolved_provider_keys
                .contains(&provider_key)
            {
                aggregate
                    .detail
                    .summary
                    .auth_state_summary
                    .resolved_provider_keys
                    .push(provider_key);
            }
        } else if !aggregate
            .detail
            .summary
            .auth_state_summary
            .failed_provider_keys
            .contains(&provider_key)
        {
            aggregate
                .detail
                .summary
                .auth_state_summary
                .failed_provider_keys
                .push(provider_key);
        }
    }

    aggregate.detail.run.capability_plan_summary = capability_projection.plan_summary.clone();
    aggregate.detail.run.provider_state_summary =
        capability_projection.provider_state_summary.clone();
    aggregate.detail.run.pending_mediation = pending_mediation.clone();
    aggregate.detail.run.capability_state_ref = Some(capability_projection.capability_state_ref);
    aggregate.detail.run.last_execution_outcome = last_execution_outcome.clone();
    aggregate.detail.run.last_mediation_outcome = last_mediation_outcome.clone();
    aggregate.detail.capability_summary = capability_projection.plan_summary;
    aggregate.detail.provider_state_summary = capability_projection.provider_state_summary;
    aggregate.detail.auth_state_summary = aggregate.detail.summary.auth_state_summary.clone();
    aggregate.detail.pending_mediation = pending_mediation;
    aggregate.detail.capability_state_ref = aggregate.detail.run.capability_state_ref.clone();
    aggregate.detail.last_execution_outcome = last_execution_outcome;
    if let actor_manifest::CompiledActorManifest::Team(team_manifest) = actor_manifest {
        team_runtime::apply_team_runtime_state(
            &mut aggregate.detail,
            team_manifest,
            &aggregate.metadata.subrun_states,
            now,
        );
    }
    sync_runtime_session_detail(&mut aggregate.detail);
    let run = aggregate.detail.run.clone();
    let conversation_id = aggregate.detail.summary.conversation_id.clone();
    let project_id = aggregate.detail.summary.project_id.clone();
    adapter.persist_runtime_projections(aggregate)?;

    Ok((
        challenge,
        execution_trace,
        assistant_message,
        run,
        conversation_id,
        project_id,
    ))
}

fn apply_memory_proposal_resolution_state(
    adapter: &RuntimeAdapter,
    session_id: &str,
    proposal_id: &str,
    now: u64,
    decision_status: &str,
    input: &ResolveRuntimeMemoryProposalInput,
) -> Result<MemoryProposalResolutionState, AppError> {
    let mut sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    let aggregate = sessions
        .get_mut(session_id)
        .ok_or_else(|| AppError::not_found("runtime session"))?;
    let proposal = aggregate
        .detail
        .run
        .pending_memory_proposal
        .as_mut()
        .ok_or_else(|| AppError::not_found("runtime memory proposal"))?;
    if proposal.proposal_id != proposal_id {
        return Err(AppError::not_found("runtime memory proposal"));
    }
    if proposal.proposal_state != "pending" {
        return Err(AppError::invalid_input(format!(
            "runtime memory proposal `{proposal_id}` is already {}",
            proposal.proposal_state
        )));
    }

    let revalidates_existing_memory = aggregate
        .detail
        .run
        .selected_memory
        .iter()
        .any(|item| item.memory_id == proposal.memory_id)
        || adapter
            .load_runtime_memory_records(&aggregate.detail.summary.project_id)?
            .iter()
            .any(|record| record.memory_id == proposal.memory_id);
    let effective_decision_status = if decision_status == "approved" && revalidates_existing_memory
    {
        "revalidated"
    } else {
        decision_status
    };

    proposal.proposal_state = effective_decision_status.into();
    proposal.review = Some(RuntimeMemoryProposalReview {
        decision: input.decision.clone(),
        reviewed_at: now,
        reviewer_ref: Some(format!("session:{session_id}")),
        note: input.note.clone(),
    });
    let resolved_proposal = proposal.clone();
    let last_mediation_outcome = Some(memory_proposal_mediation_outcome(
        &resolved_proposal,
        effective_decision_status,
        now,
    ));

    if matches!(effective_decision_status, "approved" | "revalidated") {
        let session_policy = adapter
            .load_session_policy_snapshot(&aggregate.metadata.session_policy_snapshot_ref)?;
        let record = memory_writer::build_persisted_memory_record(
            &resolved_proposal,
            &aggregate.detail.summary.project_id,
            &adapter.state.workspace_id,
            &aggregate.detail.selected_actor_ref,
            Some(session_policy.user_id.as_str()),
            now,
        );
        let body = memory_writer::build_persisted_memory_body(
            &resolved_proposal,
            input.note.as_deref(),
            now,
        );
        adapter.persist_runtime_memory_record(&record, &body)?;
    }

    aggregate.detail.summary.updated_at = now;
    aggregate.detail.run.updated_at = now;
    aggregate.detail.summary.pending_memory_proposal_count = 0;
    aggregate.detail.pending_memory_proposal_count = 0;
    aggregate.detail.summary.pending_mediation = None;
    aggregate.detail.pending_mediation = None;
    let next_memory_state_ref =
        memory_runtime::runtime_memory_state_ref(&aggregate.detail.run.id, now);
    aggregate.detail.summary.memory_state_ref = next_memory_state_ref.clone();
    aggregate.detail.memory_state_ref = next_memory_state_ref.clone();
    aggregate.detail.run.memory_state_ref = next_memory_state_ref;

    if matches!(effective_decision_status, "approved" | "revalidated") {
        if !revalidates_existing_memory {
            aggregate.detail.memory_summary.durable_memory_count += 1;
            aggregate.detail.summary.memory_summary.durable_memory_count += 1;
        }
        if let Some(item) = aggregate
            .detail
            .run
            .selected_memory
            .iter_mut()
            .find(|item| item.memory_id == resolved_proposal.memory_id)
        {
            item.title = resolved_proposal.title.clone();
            item.summary = resolved_proposal.summary.clone();
            item.kind = resolved_proposal.kind.clone();
            item.scope = resolved_proposal.scope.clone();
            item.freshness_state = if effective_decision_status == "revalidated" {
                "revalidated".into()
            } else {
                "fresh".into()
            };
            item.last_validated_at = Some(now);
        }
        if let Some(freshness_summary) = aggregate.detail.run.freshness_summary.as_mut() {
            freshness_summary.fresh_count = aggregate
                .detail
                .run
                .selected_memory
                .iter()
                .filter(|item| matches!(item.freshness_state.as_str(), "fresh" | "revalidated"))
                .count() as u64;
            freshness_summary.stale_count =
                aggregate.detail.run.selected_memory.len() as u64 - freshness_summary.fresh_count;
        }
    }

    aggregate.detail.run.pending_memory_proposal = None;
    aggregate.detail.run.pending_mediation = None;
    aggregate.detail.run.last_mediation_outcome = last_mediation_outcome.clone();
    aggregate.detail.run.checkpoint.pending_mediation = None;
    aggregate.detail.run.checkpoint.last_mediation_outcome = last_mediation_outcome;

    sync_runtime_session_detail(&mut aggregate.detail);
    let run = aggregate.detail.run.clone();
    let conversation_id = aggregate.detail.summary.conversation_id.clone();
    let project_id = aggregate.detail.summary.project_id.clone();
    adapter.persist_runtime_projections(aggregate)?;

    Ok((resolved_proposal, run, conversation_id, project_id))
}
