use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use octopus_core::{
    timestamp_now, AppError, CreateRuntimeSessionInput, ResolveRuntimeApprovalInput,
    ResolvedExecutionTarget, ResolvedRequestPolicy, SubmitRuntimeTurnInput,
};
use octopus_infra::build_infra_bundle;
use octopus_platform::{RuntimeExecutionService, RuntimeSessionService};
use octopus_runtime_adapter::{
    ModelExecutionResult, RuntimeAdapter, RuntimeConversationExecution, RuntimeConversationRequest,
    RuntimeModelDriver,
};
use runtime::{AssistantEvent, ContentBlock, MessageRole, TokenUsage};
use rusqlite::{params, Connection};
use serde_json::json;
use uuid::Uuid;

fn test_root() -> PathBuf {
    for env_key in [
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "XAI_API_KEY",
        "DEEPSEEK_API_KEY",
        "MINIMAX_API_KEY",
        "MOONSHOT_API_KEY",
        "BIGMODEL_API_KEY",
        "DASHSCOPE_API_KEY",
        "ARK_API_KEY",
        "GOOGLE_API_KEY",
    ] {
        std::env::set_var(env_key, format!("test-{env_key}"));
    }

    let root = std::env::temp_dir().join(format!("octopus-runtime-turn-loop-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).expect("test root");
    root
}

fn write_json(path: &Path, value: serde_json::Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("config dir");
    }
    fs::write(path, serde_json::to_vec_pretty(&value).expect("json")).expect("write config");
}

fn write_workspace_config(path: &Path, total_tokens: Option<u64>) {
    let configured_model = if let Some(total_tokens) = total_tokens {
        json!({
            "configuredModelId": "quota-model",
            "name": "Quota Model",
            "providerId": "anthropic",
            "modelId": "claude-sonnet-4-5",
            "credentialRef": "env:ANTHROPIC_API_KEY",
            "budgetPolicy": {
                "totalBudgetTokens": total_tokens
            },
            "enabled": true,
            "source": "workspace"
        })
    } else {
        json!({
            "configuredModelId": "quota-model",
            "name": "Quota Model",
            "providerId": "anthropic",
            "modelId": "claude-sonnet-4-5",
            "credentialRef": "env:ANTHROPIC_API_KEY",
            "enabled": true,
            "source": "workspace"
        })
    };

    write_json(
        path,
        json!({
            "configuredModels": {
                "quota-model": configured_model
            }
        }),
    );
}

fn session_input(
    conversation_id: &str,
    project_id: &str,
    title: &str,
    selected_actor_ref: &str,
    selected_configured_model_id: Option<&str>,
    execution_permission_mode: &str,
) -> CreateRuntimeSessionInput {
    CreateRuntimeSessionInput {
        conversation_id: conversation_id.into(),
        project_id: Some(project_id.into()),
        title: title.into(),
        session_kind: None,
        selected_actor_ref: selected_actor_ref.into(),
        selected_configured_model_id: selected_configured_model_id.map(str::to_string),
        execution_permission_mode: execution_permission_mode.into(),
    }
}

fn turn_input(content: &str, permission_mode: Option<&str>) -> SubmitRuntimeTurnInput {
    SubmitRuntimeTurnInput {
        content: content.into(),
        permission_mode: permission_mode.map(str::to_string),
        recall_mode: None,
        ignored_memory_ids: Vec::new(),
        memory_intent: None,
    }
}

fn grant_owner_permissions(db_path: &Path, user_id: &str) {
    let connection = Connection::open(db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO role_bindings (id, role_id, subject_type, subject_id, effect)
             VALUES (?1, 'system.owner', 'user', ?2, 'allow')",
            params![format!("binding-user-{user_id}-owner"), user_id],
        )
        .expect("grant owner permissions");
}

#[derive(Debug)]
struct ScriptedConversationRuntimeModelDriver {
    responses: Mutex<Vec<Vec<AssistantEvent>>>,
    requests: Mutex<Vec<RuntimeConversationRequest>>,
}

impl ScriptedConversationRuntimeModelDriver {
    fn new(responses: Vec<Vec<AssistantEvent>>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().rev().collect()),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn request_count(&self) -> usize {
        self.requests.lock().expect("requests mutex").len()
    }

    fn requests(&self) -> Vec<RuntimeConversationRequest> {
        self.requests.lock().expect("requests mutex").clone()
    }
}

fn last_user_text(request: &RuntimeConversationRequest) -> Option<&str> {
    request.messages.iter().rev().find_map(|message| {
        if message.role != MessageRole::User {
            return None;
        }
        message.blocks.iter().find_map(|block| match block {
            ContentBlock::Text { text } if !text.trim().is_empty() => Some(text.as_str()),
            _ => None,
        })
    })
}

#[async_trait]
impl RuntimeModelDriver for ScriptedConversationRuntimeModelDriver {
    async fn execute_prompt(
        &self,
        _target: &ResolvedExecutionTarget,
        _request_policy: &ResolvedRequestPolicy,
        _input: &str,
        _system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError> {
        Err(AppError::runtime(
            "scripted conversation executor should use execute_conversation",
        ))
    }

    async fn execute_conversation(
        &self,
        _target: &ResolvedExecutionTarget,
        _request_policy: &ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<Vec<AssistantEvent>, AppError> {
        let fallback_input =
            last_user_text(request).expect("conversation request should contain user text");
        assert!(!fallback_input.trim().is_empty());
        self.requests
            .lock()
            .expect("requests mutex")
            .push(request.clone());
        self.responses
            .lock()
            .expect("responses mutex")
            .pop()
            .ok_or_else(|| AppError::runtime("scripted conversation response missing"))
    }

    async fn execute_conversation_execution(
        &self,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<RuntimeConversationExecution, AppError> {
        Ok(RuntimeConversationExecution {
            events: self
                .execute_conversation(target, request_policy, request)
                .await?,
            deliverables: Vec::new(),
        })
    }
}

#[tokio::test]
async fn submit_turn_executes_runtime_tool_loop_on_main_path() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra.paths.db_path, "user-owner");

    let note_path = root.join("loop-note.txt");
    fs::write(&note_path, "runtime loop content\n").expect("seed note");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-runtime-loop",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Runtime Loop Agent",
                Option::<String>::None,
                "Reader",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Read files through the runtime capability loop.",
                serde_json::to_string(&vec!["read_file"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for runtime loop tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert runtime loop agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            AssistantEvent::TextDelta("Inspecting the note.".into()),
            AssistantEvent::ToolUse {
                id: "tool-read-note".into(),
                name: "read_file".into(),
                input: serde_json::json!({
                    "path": note_path.display().to_string()
                })
                .to_string(),
            },
            AssistantEvent::Usage(TokenUsage {
                input_tokens: 6,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            AssistantEvent::MessageStop,
        ],
        vec![
            AssistantEvent::TextDelta("Summary: runtime loop content.".into()),
            AssistantEvent::Usage(TokenUsage {
                input_tokens: 8,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            AssistantEvent::MessageStop,
        ],
    ]));
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-runtime-loop",
                octopus_core::DEFAULT_PROJECT_ID,
                "Runtime Loop Session",
                "agent:agent-runtime-loop",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(&session.summary.id, turn_input("Read the note", None))
        .await
        .expect("run");

    assert_eq!(run.status, "completed");
    assert_eq!(run.current_step, "completed");
    assert_eq!(run.next_action.as_deref(), Some("idle"));
    assert_eq!(run.checkpoint.current_iteration_index, 2);
    assert_eq!(run.consumed_tokens, Some(23));
    assert_eq!(executor.request_count(), 2);

    let requests = executor.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        Vec::<&str>::new()
    );
    assert!(requests[0]
        .messages
        .iter()
        .any(|message| matches!(message.role, MessageRole::User)));
    assert!(requests[1]
        .messages
        .iter()
        .any(|message| matches!(message.role, MessageRole::Tool)));

    let public_run_json = serde_json::to_value(&run).expect("public run json");
    assert!(
        public_run_json
            .pointer("/checkpoint/serializedSession")
            .is_none(),
        "public run payload should not expose serialized checkpoint state"
    );

    let output_artifact_ref = run
        .artifact_refs
        .first()
        .cloned()
        .expect("runtime output artifact ref");
    let output_artifact_path = root
        .join("data")
        .join("artifacts")
        .join("runtime")
        .join(format!("{output_artifact_ref}.json"));
    let output_artifact: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&output_artifact_path).expect("runtime output artifact"),
    )
    .expect("runtime output artifact json");
    let serialized_session = output_artifact
        .get("serializedSession")
        .and_then(|value| value.get("session"))
        .expect("serialized runtime session");
    let serialized_messages = serialized_session
        .get("messages")
        .and_then(serde_json::Value::as_array)
        .expect("serialized session messages");
    assert_eq!(serialized_messages.len(), 4);
    assert_eq!(
        output_artifact
            .get("serializedSession")
            .and_then(|value| value.get("content"))
            .and_then(serde_json::Value::as_str),
        Some("Read the note")
    );

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    let event_kinds = events
        .iter()
        .map(|event| event.kind.as_deref().unwrap_or(event.event_type.as_str()))
        .collect::<Vec<_>>();
    assert!(event_kinds.contains(&"planner.started"));
    assert!(event_kinds.contains(&"planner.completed"));
    assert!(event_kinds.contains(&"model.started"));
    assert!(event_kinds.contains(&"model.delta"));
    assert!(event_kinds.contains(&"model.tool_use"));
    assert!(event_kinds.contains(&"model.usage"));
    assert!(event_kinds.contains(&"model.completed"));
    assert!(!event_kinds.contains(&"model.streaming"));
    let model_delta_index = event_kinds
        .iter()
        .position(|kind| *kind == "model.delta")
        .expect("model delta index");
    let model_tool_use_index = event_kinds
        .iter()
        .position(|kind| *kind == "model.tool_use")
        .expect("model tool use index");
    let model_usage_index = event_kinds
        .iter()
        .position(|kind| *kind == "model.usage")
        .expect("model usage index");
    assert!(model_delta_index < model_tool_use_index);
    assert!(model_tool_use_index < model_usage_index);

    let model_delta = events
        .iter()
        .find(|event| event.kind.as_deref() == Some("model.delta"))
        .expect("model delta event");
    assert_eq!(
        model_delta
            .message
            .as_ref()
            .map(|message| message.content.as_str()),
        Some("Inspecting the note.")
    );

    let model_tool_use = events
        .iter()
        .find(|event| event.kind.as_deref() == Some("model.tool_use"))
        .expect("model tool use event");
    assert_eq!(
        model_tool_use.tool_use_id.as_deref(),
        Some("tool-read-note")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_persists_failed_checkpoint_when_assistant_stream_disconnects() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra.paths.db_path, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-interrupted-stream",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Interrupted Stream Agent",
                Option::<String>::None,
                "Responder",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Respond directly without tools.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for interrupted-stream runtime tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert interrupted-stream agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![vec![
        AssistantEvent::TextDelta("Partial answer before disconnect.".into()),
        AssistantEvent::Usage(TokenUsage {
            input_tokens: 4,
            output_tokens: 3,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        }),
    ]]));
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor,
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-interrupted-stream",
                octopus_core::DEFAULT_PROJECT_ID,
                "Interrupted Stream Session",
                "agent:agent-interrupted-stream",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Give me the interrupted answer", None),
        )
        .await
        .expect("failed run snapshot");

    assert_eq!(run.status, "failed");
    assert_eq!(run.current_step, "failed");
    assert_eq!(run.next_action.as_deref(), Some("idle"));
    assert_eq!(run.checkpoint.current_iteration_index, 1);
    assert_eq!(run.checkpoint.usage_summary.total_tokens, 7);
    assert!(run.artifact_refs.is_empty());
    assert!(run.deliverable_refs.is_empty());
    assert!(
        run.checkpoint.checkpoint_artifact_ref.is_some(),
        "interrupted stream should persist a resumable checkpoint artifact"
    );

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert_eq!(detail.messages.len(), 1);
    assert_eq!(detail.messages[0].sender_type, "user");

    let checkpoint_ref = run
        .checkpoint
        .checkpoint_artifact_ref
        .clone()
        .expect("checkpoint ref");
    let checkpoint_artifact: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join(&checkpoint_ref)).expect("checkpoint artifact"),
    )
    .expect("checkpoint json");
    assert_eq!(
        checkpoint_artifact
            .get("serializedSession")
            .and_then(|value| value.get("session"))
            .and_then(|value| value.get("messages"))
            .and_then(serde_json::Value::as_array)
            .map(|messages| messages.len()),
        Some(1)
    );
    assert_eq!(
        checkpoint_artifact
            .get("serializedSession")
            .and_then(|value| value.get("partialOutput"))
            .and_then(|value| value.get("content"))
            .and_then(serde_json::Value::as_str),
        Some("Partial answer before disconnect.")
    );
    assert_eq!(
        checkpoint_artifact
            .get("serializedSession")
            .and_then(|value| value.get("partialOutput"))
            .and_then(|value| value.get("status"))
            .and_then(serde_json::Value::as_str),
        Some("interrupted")
    );

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    let event_kinds = events
        .iter()
        .map(|event| event.kind.as_deref().unwrap_or(event.event_type.as_str()))
        .collect::<Vec<_>>();
    assert!(event_kinds.contains(&"model.started"));
    assert!(event_kinds.contains(&"model.delta"));
    assert!(event_kinds.contains(&"model.usage"));
    assert!(event_kinds.contains(&"runtime.error"));
    assert!(!event_kinds.contains(&"model.completed"));
    assert_eq!(
        event_kinds
            .iter()
            .filter(|kind| **kind == "runtime.message.created")
            .count(),
        1,
        "only the user message should be projected"
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn approval_resume_uses_runtime_tool_loop_instead_of_one_shot_execution() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra.paths.db_path, "user-owner");

    let note_path = root.join("approval-loop-note.txt");
    fs::write(&note_path, "approval loop content\n").expect("seed note");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "agent-runtime-approval-loop",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Runtime Approval Loop Agent",
                Option::<String>::None,
                "Reader",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Resume approval into the runtime capability loop.",
                serde_json::to_string(&vec!["read_file"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for approval loop tests.",
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert runtime approval loop agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            AssistantEvent::TextDelta("Inspecting the approved note.".into()),
            AssistantEvent::ToolUse {
                id: "tool-approved-read-note".into(),
                name: "read_file".into(),
                input: serde_json::json!({
                    "path": note_path.display().to_string()
                })
                .to_string(),
            },
            AssistantEvent::Usage(TokenUsage {
                input_tokens: 5,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            AssistantEvent::MessageStop,
        ],
        vec![
            AssistantEvent::TextDelta("Approved summary: approval loop content.".into()),
            AssistantEvent::Usage(TokenUsage {
                input_tokens: 7,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            AssistantEvent::MessageStop,
        ],
    ]));
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-runtime-approval-loop",
                octopus_core::DEFAULT_PROJECT_ID,
                "Runtime Approval Loop Session",
                "agent:agent-runtime-approval-loop",
                Some("quota-model"),
                "workspace-write",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Read the note after approval", Some("workspace-write")),
        )
        .await
        .expect("pending run");
    assert_eq!(pending_run.status, "waiting_approval");
    assert_eq!(executor.request_count(), 0);

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    let approval_id = detail
        .pending_approval
        .as_ref()
        .map(|approval| approval.id.clone())
        .expect("approval id");

    let resolved = adapter
        .resolve_approval(
            &session.summary.id,
            &approval_id,
            ResolveRuntimeApprovalInput {
                decision: "approve".into(),
            },
        )
        .await
        .expect("resolved approval");

    assert_eq!(resolved.status, "completed");
    assert_eq!(resolved.current_step, "completed");
    assert_eq!(resolved.checkpoint.current_iteration_index, 2);
    assert_eq!(resolved.consumed_tokens, Some(21));
    assert_eq!(executor.request_count(), 2);

    let requests = executor.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests[1]
        .messages
        .iter()
        .any(|message| matches!(message.role, MessageRole::Tool)));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn approval_resume_persists_failed_checkpoint_when_assistant_stream_disconnects() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra.paths.db_path, "user-owner");

    let note_path = root.join("approval-loop-interrupted-note.txt");
    fs::write(&note_path, "approval interruption content\n").expect("seed note");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "agent-runtime-approval-loop-interrupted",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Runtime Approval Loop Interrupted Agent",
                Option::<String>::None,
                "Reader",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Resume approval into the runtime capability loop and handle disconnects.",
                serde_json::to_string(&vec!["read_file"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for interrupted approval loop tests.",
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert interrupted approval loop agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            AssistantEvent::TextDelta("Inspecting the approved note.".into()),
            AssistantEvent::ToolUse {
                id: "tool-approved-read-note-interrupted".into(),
                name: "read_file".into(),
                input: serde_json::json!({
                    "path": note_path.display().to_string()
                })
                .to_string(),
            },
            AssistantEvent::Usage(TokenUsage {
                input_tokens: 5,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            AssistantEvent::MessageStop,
        ],
        vec![
            AssistantEvent::TextDelta("Approved summary before disconnect.".into()),
            AssistantEvent::Usage(TokenUsage {
                input_tokens: 7,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
        ],
    ]));
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-runtime-approval-loop-interrupted",
                octopus_core::DEFAULT_PROJECT_ID,
                "Runtime Approval Loop Interrupted Session",
                "agent:agent-runtime-approval-loop-interrupted",
                Some("quota-model"),
                "workspace-write",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input(
                "Read the note after approval and handle interruption",
                Some("workspace-write"),
            ),
        )
        .await
        .expect("pending run");
    assert_eq!(pending_run.status, "waiting_approval");
    assert_eq!(executor.request_count(), 0);

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    let approval_id = detail
        .pending_approval
        .as_ref()
        .map(|approval| approval.id.clone())
        .expect("approval id");

    let resolved = adapter
        .resolve_approval(
            &session.summary.id,
            &approval_id,
            ResolveRuntimeApprovalInput {
                decision: "approve".into(),
            },
        )
        .await
        .expect("failed approval resolution should still project a run snapshot");

    assert_eq!(resolved.status, "failed");
    assert_eq!(resolved.current_step, "failed");
    assert_eq!(resolved.next_action.as_deref(), Some("idle"));
    assert_eq!(resolved.checkpoint.current_iteration_index, 2);
    assert_eq!(resolved.checkpoint.usage_summary.total_tokens, 21);
    assert!(resolved.artifact_refs.is_empty());
    assert!(resolved.deliverable_refs.is_empty());
    assert!(
        resolved.checkpoint.checkpoint_artifact_ref.is_some(),
        "approval replay interruption should persist a failed checkpoint artifact"
    );
    assert_eq!(executor.request_count(), 2);

    let resolved_detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("resolved detail");
    assert_eq!(resolved_detail.messages.len(), 1);
    assert_eq!(resolved_detail.messages[0].sender_type, "user");
    assert_eq!(resolved_detail.messages[0].status, "failed");

    let checkpoint_ref = resolved
        .checkpoint
        .checkpoint_artifact_ref
        .clone()
        .expect("checkpoint ref");
    let checkpoint_artifact: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join(&checkpoint_ref)).expect("checkpoint artifact"),
    )
    .expect("checkpoint json");
    assert_eq!(
        checkpoint_artifact
            .get("serializedSession")
            .and_then(|value| value.get("partialOutput"))
            .and_then(|value| value.get("content"))
            .and_then(serde_json::Value::as_str),
        Some("Approved summary before disconnect.")
    );
    assert_eq!(
        checkpoint_artifact
            .get("serializedSession")
            .and_then(|value| value.get("partialOutput"))
            .and_then(|value| value.get("status"))
            .and_then(serde_json::Value::as_str),
        Some("interrupted")
    );

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    let event_kinds = events
        .iter()
        .map(|event| event.kind.as_deref().unwrap_or(event.event_type.as_str()))
        .collect::<Vec<_>>();
    assert!(event_kinds.contains(&"approval.resolved"));
    assert!(event_kinds.contains(&"model.started"));
    assert!(event_kinds.contains(&"model.delta"));
    assert!(event_kinds.contains(&"model.usage"));
    assert!(event_kinds.contains(&"runtime.error"));
    assert_eq!(
        event_kinds
            .iter()
            .filter(|kind| **kind == "model.completed")
            .count(),
        1,
        "only the completed first iteration should emit model.completed"
    );
    assert_eq!(
        event_kinds
            .iter()
            .filter(|kind| **kind == "runtime.message.created")
            .count(),
        1,
        "only the completed first iteration should create an assistant message"
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn capability_call_approval_resume_replays_only_the_blocked_tool_use() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra.paths.db_path, "user-owner");

    let output_path = root.join("capability-call-approval.txt");
    let output_path_string = output_path.display().to_string();

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, approval_preference_json, default_model_strategy_json, capability_policy_json, permission_envelope_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                "agent-capability-call-approval-loop",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Capability Call Approval Loop Agent",
                Option::<String>::None,
                "Writer",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Resume a blocked capability call without replaying the whole turn.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for capability-call approval resume tests.",
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert capability-call approval loop agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            AssistantEvent::TextDelta("Running the requested danger tool.".into()),
            AssistantEvent::ToolUse {
                id: "tool-write-approved-note".into(),
                name: "bash".into(),
                input: serde_json::json!({
                    "command": format!(
                        "printf 'capability approval content\\n' > '{}'",
                        output_path_string
                    ),
                    "run_in_background": true
                })
                .to_string(),
            },
            AssistantEvent::Usage(TokenUsage {
                input_tokens: 5,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            AssistantEvent::MessageStop,
        ],
        vec![
            AssistantEvent::TextDelta("Completed the approved danger tool.".into()),
            AssistantEvent::Usage(TokenUsage {
                input_tokens: 7,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            AssistantEvent::MessageStop,
        ],
    ]));
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-capability-call-approval-loop",
                octopus_core::DEFAULT_PROJECT_ID,
                "Capability Call Approval Loop Session",
                "agent:agent-capability-call-approval-loop",
                Some("quota-model"),
                octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Write the file after capability approval", None),
        )
        .await
        .expect("pending capability-call approval run");

    assert_eq!(pending_run.status, "waiting_approval");
    assert_eq!(pending_run.current_step, "awaiting_approval");
    assert_eq!(pending_run.checkpoint.current_iteration_index, 0);
    assert_eq!(executor.request_count(), 0);
    assert_eq!(
        pending_run
            .approval_target
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("model-execution")
    );
    let pending_run_json = serde_json::to_value(&pending_run).expect("pending run json");
    assert!(
        pending_run_json
            .pointer("/checkpoint/serializedSession")
            .is_none(),
        "public pending run payload should not expose serialized checkpoint state"
    );

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    let approval_id = detail
        .pending_approval
        .as_ref()
        .map(|approval| approval.id.clone())
        .expect("approval id");

    let resolved = adapter
        .resolve_approval(
            &session.summary.id,
            &approval_id,
            ResolveRuntimeApprovalInput {
                decision: "approve".into(),
            },
        )
        .await
        .expect("resolved capability-call approval");

    assert_eq!(resolved.status, "completed");
    assert_eq!(resolved.current_step, "completed");
    assert_eq!(resolved.checkpoint.current_iteration_index, 2);
    assert_eq!(executor.request_count(), 2);
    let resolved_run_json = serde_json::to_value(&resolved).expect("resolved run json");
    assert!(
        resolved_run_json
            .pointer("/checkpoint/serializedSession")
            .is_none(),
        "resolved public run payload should not expose serialized checkpoint state"
    );
    let resolved_output_artifact_ref = resolved
        .artifact_refs
        .first()
        .cloned()
        .expect("resolved runtime output artifact ref");
    let resolved_output_artifact: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            root.join("data")
                .join("artifacts")
                .join("runtime")
                .join(format!("{resolved_output_artifact_ref}.json")),
        )
        .expect("resolved runtime output artifact"),
    )
    .expect("resolved runtime output artifact json");
    assert_eq!(
        resolved_output_artifact
            .get("serializedSession")
            .and_then(|value| value.get("pendingToolUses"))
            .and_then(serde_json::Value::as_array)
            .map(|items| items.len()),
        Some(0)
    );

    let requests = executor.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests[1]
        .messages
        .iter()
        .any(|message| matches!(message.role, MessageRole::Tool)));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
