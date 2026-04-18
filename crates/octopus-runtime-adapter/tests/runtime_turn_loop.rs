use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
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
            "tokenQuota": {
                "totalTokens": total_tokens
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
        vec!["read_file"]
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
    assert!(event_kinds.contains(&"model.streaming"));
    assert!(event_kinds.contains(&"model.completed"));
    assert!(event_kinds.contains(&"tool.started"));
    assert!(event_kinds.contains(&"tool.completed"));
    let tool_started_index = event_kinds
        .iter()
        .position(|kind| *kind == "tool.started")
        .expect("tool started index");
    let tool_completed_index = event_kinds
        .iter()
        .position(|kind| *kind == "tool.completed")
        .expect("tool completed index");
    assert!(tool_started_index < tool_completed_index);

    let tool_started = events
        .iter()
        .find(|event| event.kind.as_deref() == Some("tool.started"))
        .expect("tool started event");
    let expected_target_ref = format!("capability-call:{}:tool-read-note", run.id);
    assert_eq!(tool_started.run_id.as_deref(), Some(run.id.as_str()));
    assert_eq!(tool_started.parent_run_id, None);
    assert_eq!(
        tool_started.actor_ref.as_deref(),
        Some(run.actor_ref.as_str())
    );
    assert_eq!(tool_started.tool_use_id.as_deref(), Some("tool-read-note"));
    assert_eq!(tool_started.target_kind.as_deref(), Some("capability-call"));
    assert_eq!(
        tool_started.target_ref.as_deref(),
        Some(expected_target_ref.as_str())
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
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
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
    assert_eq!(pending_run.checkpoint.current_iteration_index, 1);
    assert_eq!(executor.request_count(), 1);
    assert_eq!(
        pending_run
            .approval_target
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("capability-call")
    );
    assert_eq!(
        pending_run
            .approval_target
            .as_ref()
            .map(|approval| approval.tool_name.as_str()),
        Some("bash")
    );
    let pending_run_json = serde_json::to_value(&pending_run).expect("pending run json");
    assert!(
        pending_run_json
            .pointer("/checkpoint/serializedSession")
            .is_none(),
        "public pending run payload should not expose serialized checkpoint state"
    );
    let pending_checkpoint_ref = pending_run
        .checkpoint
        .checkpoint_artifact_ref
        .clone()
        .expect("pending checkpoint artifact ref");
    let pending_checkpoint_artifact: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join(&pending_checkpoint_ref))
            .expect("pending checkpoint artifact"),
    )
    .expect("pending checkpoint artifact json");
    assert_eq!(
        pending_checkpoint_artifact
            .get("serializedSession")
            .and_then(|value| value.get("pendingToolUses"))
            .and_then(serde_json::Value::as_array)
            .map(|items| items.len()),
        Some(1)
    );
    assert_eq!(
        pending_checkpoint_artifact
            .pointer("/serializedSession/pendingToolUses/0/toolUseId")
            .and_then(serde_json::Value::as_str),
        Some("tool-write-approved-note")
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
    for _ in 0..20 {
        if fs::read_to_string(&output_path)
            .map(|content| content == "capability approval content\n")
            .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        fs::read_to_string(&output_path).expect("written file"),
        "capability approval content\n"
    );
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
