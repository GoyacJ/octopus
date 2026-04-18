use super::*;

use std::{fs, path::Path};

use async_trait::async_trait;
use octopus_core::{
    CancelRuntimeSubrunInput, CreateDeliverableVersionInput, CreateRuntimeSessionInput,
    PromoteDeliverableInput, ResolveRuntimeMemoryProposalInput, RuntimeCapabilityExecutionOutcome,
    RuntimePendingMediationSummary,
};
use octopus_infra::build_infra_bundle;
use octopus_platform::{
    ModelRegistryService, RuntimeConfigService, RuntimeExecutionService, RuntimeSessionService,
    WorkspaceService,
};
use rusqlite::params;
use serde_json::json;

fn test_root() -> std::path::PathBuf {
    ensure_test_provider_api_keys();
    let root = std::env::temp_dir().join(format!("octopus-runtime-adapter-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).expect("test root");
    root
}

fn ensure_test_provider_api_keys() {
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
}

async fn builtin_agent_actor_ref(infra: &octopus_infra::InfraBundle) -> String {
    let builtin_agent = infra
        .workspace
        .list_agents()
        .await
        .expect("list agents")
        .into_iter()
        .find(|agent| {
            agent
                .integration_source
                .as_ref()
                .is_some_and(|source| source.kind == "builtin-template")
        })
        .expect("builtin agent");
    format!("agent:{}", builtin_agent.id)
}

async fn builtin_team_actor_ref(infra: &octopus_infra::InfraBundle) -> String {
    let builtin_team = infra
        .workspace
        .list_teams()
        .await
        .expect("list teams")
        .into_iter()
        .find(|team| {
            team.integration_source
                .as_ref()
                .is_some_and(|source| source.kind == "builtin-template")
                && !team.member_refs.is_empty()
        })
        .expect("builtin team with members");
    format!("team:{}", builtin_team.id)
}

fn canonical_test_agent_ref(agent_id: &str) -> String {
    format!("agent:{agent_id}")
}

fn canonical_test_member_refs(agent_ids: &[&str]) -> String {
    serde_json::to_string(
        &agent_ids
            .iter()
            .map(|agent_id| canonical_test_agent_ref(agent_id))
            .collect::<Vec<_>>(),
    )
    .expect("member refs")
}

fn legacy_runtime_sessions_dir(root: &Path) -> std::path::PathBuf {
    root.join("runtime").join("sessions")
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

fn write_workspace_config_with_http_mcp(path: &Path, total_tokens: Option<u64>, server_name: &str) {
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
            },
            "mcpServers": {
                server_name: {
                    "type": "http",
                    "url": format!("https://{server_name}.example.invalid/mcp")
                }
            }
        }),
    );
}

fn write_workspace_config_with_plugins(
    path: &Path,
    total_tokens: Option<u64>,
    enabled_plugins: serde_json::Value,
    external_directories: &[&str],
) {
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
            },
            "enabledPlugins": enabled_plugins,
            "plugins": {
                "externalDirectories": external_directories
            }
        }),
    );
}

fn write_external_plugin(root: &Path, plugin_dir_name: &str, plugin_name: &str, tool_name: &str) {
    let plugin_root = root.join("external-plugins").join(plugin_dir_name);
    fs::create_dir_all(&plugin_root).expect("plugin root");
    let script_path = plugin_root.join("echo.sh");
    fs::write(&script_path, "#!/bin/sh\ncat\n").expect("plugin script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&script_path)
            .expect("plugin script metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).expect("plugin script permissions");
    }
    write_json(
        &plugin_root.join("plugin.json"),
        json!({
            "name": plugin_name,
            "version": "0.1.0",
            "description": "Adapter test plugin",
            "tools": [
                {
                    "name": tool_name,
                    "description": "Echo from plugin",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "message": { "type": "string" }
                        },
                        "required": ["message"],
                        "additionalProperties": false
                    },
                    "command": "./echo.sh",
                    "requiredPermission": "read-only"
                }
            ]
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

fn home_session_input(
    conversation_id: &str,
    title: &str,
    selected_actor_ref: &str,
    selected_configured_model_id: Option<&str>,
    execution_permission_mode: &str,
) -> CreateRuntimeSessionInput {
    CreateRuntimeSessionInput {
        conversation_id: conversation_id.into(),
        project_id: None,
        title: title.into(),
        session_kind: Some("pet".into()),
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

fn grant_owner_permissions(infra: &octopus_infra::InfraBundle, user_id: &str) {
    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO role_bindings (id, role_id, subject_type, subject_id, effect)
             VALUES (?1, 'system.owner', 'user', ?2, 'allow')",
            params![format!("binding-user-{user_id}-owner"), user_id],
        )
        .expect("grant owner permissions");
}

#[derive(Debug, Clone)]
struct FixedTokenRuntimeModelDriver {
    total_tokens: Option<u32>,
}

#[async_trait]
impl RuntimeModelDriver for FixedTokenRuntimeModelDriver {
    async fn execute_prompt(
        &self,
        _target: &ResolvedExecutionTarget,
        _request_policy: &octopus_core::ResolvedRequestPolicy,
        input: &str,
        system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError> {
        let prompt_prefix = system_prompt
            .map(|value| format!(" [{value}]"))
            .unwrap_or_default();
        Ok(ModelExecutionResult {
            content: format!("fixed token response{prompt_prefix} -> {input}"),
            request_id: Some("fixed-token-request".into()),
            total_tokens: self.total_tokens,
            deliverables: Vec::new(),
        })
    }
}

#[derive(Debug, Clone)]
struct ExplicitDeliverableRuntimeModelDriver {
    total_tokens: Option<u32>,
    content: String,
    deliverables: Vec<ModelExecutionDeliverable>,
}

#[async_trait]
impl RuntimeModelDriver for ExplicitDeliverableRuntimeModelDriver {
    async fn execute_prompt(
        &self,
        _target: &ResolvedExecutionTarget,
        _request_policy: &octopus_core::ResolvedRequestPolicy,
        input: &str,
        _system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError> {
        Ok(ModelExecutionResult {
            content: if self.content.is_empty() {
                format!("explicit deliverable response -> {input}")
            } else {
                self.content.clone()
            },
            request_id: Some("explicit-deliverable-request".into()),
            total_tokens: self.total_tokens,
            deliverables: self.deliverables.clone(),
        })
    }

    async fn execute_conversation_execution(
        &self,
        target: &ResolvedExecutionTarget,
        request_policy: &octopus_core::ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<RuntimeConversationExecution, AppError> {
        let input = last_user_text(request).unwrap_or_default();
        let response = self
            .execute_prompt(target, request_policy, input, None)
            .await?;
        let mut events = vec![runtime::AssistantEvent::TextDelta(response.content)];
        if let Some(total_tokens) = response.total_tokens {
            events.push(runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 0,
                output_tokens: total_tokens,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }));
        }
        events.push(runtime::AssistantEvent::MessageStop);
        Ok(RuntimeConversationExecution {
            events,
            deliverables: response.deliverables,
        })
    }
}

#[derive(Debug, Default)]
struct InspectingPromptRuntimeModelDriver {
    request_policies: Mutex<Vec<octopus_core::ResolvedRequestPolicy>>,
}

impl InspectingPromptRuntimeModelDriver {
    fn last_request_policy(&self) -> Option<octopus_core::ResolvedRequestPolicy> {
        self.request_policies
            .lock()
            .expect("request policies mutex")
            .last()
            .cloned()
    }
}

#[async_trait]
impl RuntimeModelDriver for InspectingPromptRuntimeModelDriver {
    async fn execute_prompt(
        &self,
        _target: &ResolvedExecutionTarget,
        request_policy: &octopus_core::ResolvedRequestPolicy,
        input: &str,
        _system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError> {
        self.request_policies
            .lock()
            .expect("request policies mutex")
            .push(request_policy.clone());
        Ok(ModelExecutionResult {
            content: format!("inspected -> {input}"),
            request_id: Some("inspect-request".into()),
            total_tokens: Some(4),
            deliverables: Vec::new(),
        })
    }
}

#[derive(Debug, Default)]
struct FailingRuntimeSecretStore;

impl secret_store::RuntimeSecretStore for FailingRuntimeSecretStore {
    fn put_secret(&self, _reference: &str, _value: &str) -> Result<(), AppError> {
        Err(AppError::runtime("simulated secret store failure"))
    }

    fn get_secret(&self, _reference: &str) -> Result<Option<String>, AppError> {
        Ok(None)
    }

    fn delete_secret(&self, _reference: &str) -> Result<(), AppError> {
        Ok(())
    }
}

#[derive(Debug)]
struct ScriptedConversationRuntimeModelDriver {
    responses: Mutex<Vec<Vec<runtime::AssistantEvent>>>,
    requests: Mutex<Vec<RuntimeConversationRequest>>,
}

impl ScriptedConversationRuntimeModelDriver {
    fn new(responses: Vec<Vec<runtime::AssistantEvent>>) -> Self {
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
        if message.role != runtime::MessageRole::User {
            return None;
        }
        message.blocks.iter().find_map(|block| match block {
            runtime::ContentBlock::Text { text } if !text.trim().is_empty() => Some(text.as_str()),
            _ => None,
        })
    })
}

#[async_trait]
impl RuntimeModelDriver for ScriptedConversationRuntimeModelDriver {
    async fn execute_prompt(
        &self,
        _target: &ResolvedExecutionTarget,
        _request_policy: &octopus_core::ResolvedRequestPolicy,
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
        _request_policy: &octopus_core::ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<Vec<runtime::AssistantEvent>, AppError> {
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
        request_policy: &octopus_core::ResolvedRequestPolicy,
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
async fn runtime_config_resolution_respects_user_workspace_project_precedence() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let user_id = "user-owner";
    let project_id = "proj-redesign";

    write_json(
        &infra
            .paths
            .runtime_user_config_dir
            .join(format!("{user_id}.json")),
        json!({
            "model": "user-model",
            "provider": {
                "defaultModel": "user-default"
            },
            "permissions": {
                "defaultMode": "readonly"
            },
            "shared": {
                "marker": "user",
                "userOnly": true
            }
        }),
    );
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "model": "workspace-model",
            "permissions": {
                "defaultMode": "plan"
            },
            "shared": {
                "marker": "workspace",
                "workspaceOnly": true
            }
        }),
    );
    write_json(
        &infra
            .paths
            .runtime_project_config_dir
            .join(format!("{project_id}.json")),
        json!({
            "model": "project-model",
            "shared": {
                "marker": "project",
                "projectOnly": true
            }
        }),
    );

    let workspace_config = adapter.get_config().await.expect("workspace config");
    assert_eq!(
        workspace_config
            .sources
            .iter()
            .map(|source| source.scope.as_str())
            .collect::<Vec<_>>(),
        vec!["workspace"]
    );
    assert_eq!(
        workspace_config.effective_config.get("model"),
        Some(&json!("workspace-model"))
    );
    assert_eq!(workspace_config.effective_config.get("provider"), None);

    let user_config = adapter.get_user_config(user_id).await.expect("user config");
    assert_eq!(
        user_config
            .sources
            .iter()
            .map(|source| source.source_key.clone())
            .collect::<Vec<_>>(),
        vec![format!("user:{user_id}"), "workspace".to_string()]
    );
    assert_eq!(
        user_config.effective_config.get("model"),
        Some(&json!("workspace-model"))
    );
    assert_eq!(
        user_config
            .effective_config
            .pointer("/permissions/defaultMode"),
        Some(&json!("plan"))
    );
    assert_eq!(
        user_config
            .effective_config
            .pointer("/provider/defaultModel"),
        Some(&json!("user-default"))
    );
    assert_eq!(
        user_config.effective_config.pointer("/shared/marker"),
        Some(&json!("workspace"))
    );
    assert_eq!(
        user_config.effective_config.pointer("/shared/userOnly"),
        Some(&json!(true))
    );
    assert_eq!(
        user_config
            .effective_config
            .pointer("/shared/workspaceOnly"),
        Some(&json!(true))
    );

    let project_config = adapter
        .get_project_config(project_id, user_id)
        .await
        .expect("project config");
    assert_eq!(
        project_config
            .sources
            .iter()
            .map(|source| source.source_key.clone())
            .collect::<Vec<_>>(),
        vec![
            format!("user:{user_id}"),
            "workspace".to_string(),
            format!("project:{project_id}"),
        ]
    );
    assert_eq!(
        project_config.effective_config.get("model"),
        Some(&json!("project-model"))
    );
    assert_eq!(
        project_config
            .effective_config
            .pointer("/permissions/defaultMode"),
        Some(&json!("plan"))
    );
    assert_eq!(
        project_config
            .effective_config
            .pointer("/provider/defaultModel"),
        Some(&json!("user-default"))
    );
    assert_eq!(
        project_config.effective_config.pointer("/shared/marker"),
        Some(&json!("project"))
    );
    assert_eq!(
        project_config.effective_config.pointer("/shared/userOnly"),
        Some(&json!(true))
    );
    assert_eq!(
        project_config
            .effective_config
            .pointer("/shared/workspaceOnly"),
        Some(&json!(true))
    );
    assert_eq!(
        project_config
            .effective_config
            .pointer("/shared/projectOnly"),
        Some(&json!(true))
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn session_policy_clamps_requested_permission_mode_to_project_runtime_max() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    grant_owner_permissions(&infra, "user-owner");

    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    write_json(
        &infra
            .paths
            .runtime_project_config_dir
            .join(format!("{}.json", octopus_core::DEFAULT_PROJECT_ID)),
        json!({
            "permissions": {
                "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                "maxMode": octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE
            }
        }),
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                "agent-permission-clamp",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Permission Clamp Agent",
                Option::<String>::None,
                "Operator",
                serde_json::to_string(&vec!["runtime"]).expect("tags"),
                "Use the runtime capability planner.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for session permission clamp tests.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({})).expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert permission clamp agent");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-permission-clamp",
                octopus_core::DEFAULT_PROJECT_ID,
                "Permission Clamp Session",
                "agent:agent-permission-clamp",
                Some("quota-model"),
                octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    assert_eq!(
        session.summary.session_policy.execution_permission_mode,
        octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE
    );

    let policy = adapter
        .load_session_policy_snapshot(&format!("{}-policy", session.summary.id))
        .expect("policy snapshot");
    assert_eq!(
        policy.execution_permission_mode,
        octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn create_session_supports_pet_home_context_without_project_id() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    grant_owner_permissions(&infra, "user-owner");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let detail = adapter
        .create_session_with_owner_ceiling(
            home_session_input(
                "conv-pet-home",
                "Pet Home Session",
                &agent_actor_ref,
                None,
                octopus_core::RUNTIME_PERMISSION_READ_ONLY,
            ),
            "user-owner",
            None,
        )
        .await
        .expect("home session");

    assert_eq!(detail.summary.project_id, "");
    assert_eq!(detail.summary.session_kind, "pet");
    assert!(detail.summary.started_from_scope_set.is_empty());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn session_policy_clamps_requested_permission_mode_to_owner_ceiling() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                "agent-owner-ceiling",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Owner Ceiling Agent",
                Option::<String>::None,
                "Operator",
                serde_json::to_string(&vec!["runtime"]).expect("tags"),
                "Use the runtime capability planner.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for owner ceiling clamp tests.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({})).expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert owner ceiling agent");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session_with_owner_ceiling(
            session_input(
                "conv-owner-ceiling",
                octopus_core::DEFAULT_PROJECT_ID,
                "Owner Ceiling Session",
                "agent:agent-owner-ceiling",
                Some("quota-model"),
                octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
            ),
            "user-owner",
            Some(octopus_core::RUNTIME_PERMISSION_READ_ONLY),
        )
        .await
        .expect("session");

    assert_eq!(
        session.summary.session_policy.execution_permission_mode,
        octopus_core::RUNTIME_PERMISSION_READ_ONLY
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn runtime_session_snapshot_uses_scope_order_from_user_to_project() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let user_id = "user-owner";
    let project_id = "proj-redesign";

    write_json(
        &infra
            .paths
            .runtime_user_config_dir
            .join(format!("{user_id}.json")),
        json!({ "model": "user-model" }),
    );
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({ "model": "workspace-model" }),
    );
    write_json(
        &infra
            .paths
            .runtime_project_config_dir
            .join(format!("{project_id}.json")),
        json!({ "model": "project-model" }),
    );

    let detail = adapter
        .create_session(
            session_input(
                "conv-1",
                project_id,
                "Runtime precedence",
                &agent_actor_ref,
                None,
                "readonly",
            ),
            user_id,
        )
        .await
        .expect("session");

    assert_eq!(
        detail.summary.started_from_scope_set,
        vec![
            "user".to_string(),
            "workspace".to_string(),
            "project".to_string()
        ]
    );
    assert_eq!(detail.summary.selected_actor_ref, agent_actor_ref);
    assert_eq!(
        detail.summary.manifest_revision,
        octopus_core::ASSET_MANIFEST_REVISION_V2
    );
    assert_eq!(
        detail.summary.session_policy.execution_permission_mode,
        octopus_core::RUNTIME_PERMISSION_READ_ONLY
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let source_refs: String = connection
        .query_row(
            "SELECT source_refs FROM runtime_config_snapshots WHERE id = ?1",
            [&detail.summary.config_snapshot_id],
            |row| row.get(0),
        )
        .expect("source refs");
    assert_eq!(
        source_refs,
        json!([
            format!("user:{user_id}"),
            "workspace",
            format!("project:{project_id}"),
        ])
        .to_string()
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn runtime_config_validation_rejects_non_positive_token_quota() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let validation = adapter
        .validate_config(RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: json!({
                "configuredModels": {
                    "quota-model": {
                        "configuredModelId": "quota-model",
                        "name": "Quota Model",
                        "providerId": "anthropic",
                        "modelId": "claude-sonnet-4-5",
                        "tokenQuota": {
                            "totalTokens": 0
                        },
                        "enabled": true,
                        "source": "workspace"
                    }
                }
            }),
            configured_model_credentials: Vec::new(),
        })
        .await
        .expect("validation result");

    assert!(!validation.valid);
    assert!(validation
        .errors
        .iter()
        .any(|error| error.contains("tokenQuota.totalTokens")));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn runtime_config_validation_accepts_backfilled_upstream_fields_across_scopes() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let patch = json!({
        "aliases": {
            "fast": "gpt-5-mini"
        },
        "providerFallbacks": {
            "primary": "anthropic",
            "fallbacks": ["openai"]
        },
        "trustedRoots": ["/tmp/octopus"],
        "plugins": {
            "maxOutputTokens": 4096
        }
    });

    let workspace = adapter
        .validate_config(RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: patch.clone(),
            configured_model_credentials: Vec::new(),
        })
        .await
        .expect("workspace validation");
    assert!(workspace.valid);
    assert!(workspace.errors.is_empty());
    assert!(workspace.warnings.is_empty());

    let project = adapter
        .validate_project_config(
            "proj-sync",
            "user-sync",
            RuntimeConfigPatch {
                scope: "project".into(),
                patch: patch.clone(),
                configured_model_credentials: Vec::new(),
            },
        )
        .await
        .expect("project validation");
    assert!(project.valid);
    assert!(project.errors.is_empty());
    assert!(project.warnings.is_empty());

    let user = adapter
        .validate_user_config(
            "user-sync",
            RuntimeConfigPatch {
                scope: "user".into(),
                patch,
                configured_model_credentials: Vec::new(),
            },
        )
        .await
        .expect("user validation");
    assert!(user.valid);
    assert!(user.errors.is_empty());
    assert!(user.warnings.is_empty());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn project_settings_validation_accepts_disabled_runtime_arrays() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let validation = adapter
        .validate_project_config(
            "proj-sync",
            "user-sync",
            RuntimeConfigPatch {
                scope: "project".into(),
                patch: json!({
                    "projectSettings": {
                        "tools": {
                            "disabledSourceKeys": ["builtin:bash"],
                            "overrides": {
                                "builtin:bash": {
                                    "permissionMode": "readonly"
                                }
                            }
                        },
                        "agents": {
                            "disabledAgentIds": ["agent-architect"],
                            "disabledTeamIds": ["team-studio"]
                        }
                    }
                }),
                configured_model_credentials: Vec::new(),
            },
        )
        .await
        .expect("project validation");

    assert!(validation.valid);
    assert!(validation.errors.is_empty());
    assert!(validation.warnings.is_empty());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn project_settings_validation_ignores_legacy_enabled_runtime_arrays() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let validation = adapter
        .validate_project_config(
            "proj-sync",
            "user-sync",
            RuntimeConfigPatch {
                scope: "project".into(),
                patch: json!({
                    "projectSettings": {
                        "tools": {
                            "enabledSourceKeys": []
                        },
                        "agents": {
                            "enabledAgentIds": [],
                            "enabledTeamIds": []
                        }
                    }
                }),
                configured_model_credentials: Vec::new(),
            },
        )
        .await
        .expect("project validation");

    assert!(validation.valid);
    assert!(validation.errors.is_empty());
    assert!(validation.warnings.is_empty());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn runtime_config_validation_warns_for_unknown_and_deprecated_top_level_keys() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let validation = adapter
        .validate_config(RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: json!({
                "telemetry": true,
                "allowedTools": ["read_file"]
            }),
            configured_model_credentials: Vec::new(),
        })
        .await
        .expect("validation result");

    assert!(validation.valid);
    assert!(validation.errors.is_empty());
    assert!(validation
        .warnings
        .iter()
        .any(|warning| warning.contains("unknown runtime config key `telemetry`")));
    assert!(validation.warnings.iter().any(|warning| {
        warning.contains("deprecated runtime config key `allowedTools`")
            && warning.contains("permissions.allow")
    }));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn runtime_config_validation_reports_wrong_type_for_backfilled_fields() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let validation = adapter
        .validate_config(RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: json!({
                "trustedRoots": "not-an-array"
            }),
            configured_model_credentials: Vec::new(),
        })
        .await
        .expect("validation result");

    assert!(!validation.valid);
    assert!(validation
        .errors
        .iter()
        .any(|error| error.contains("trustedRoots")));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn runtime_effective_config_includes_backfilled_upstream_fields() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "aliases": {
                "fast": "gpt-5-mini"
            },
            "trustedRoots": ["/workspace/root"],
            "plugins": {
                "maxOutputTokens": 2048
            }
        }),
    );
    write_json(
        &infra
            .paths
            .runtime_project_config_dir
            .join("proj-sync.json"),
        json!({
            "providerFallbacks": {
                "primary": "anthropic",
                "fallbacks": ["openai", "dashscope"]
            }
        }),
    );

    let effective = adapter
        .get_project_config("proj-sync", "")
        .await
        .expect("effective config");

    assert_eq!(
        effective.effective_config.pointer("/aliases/fast"),
        Some(&json!("gpt-5-mini"))
    );
    assert_eq!(
        effective.effective_config.pointer("/trustedRoots/0"),
        Some(&json!("/workspace/root"))
    );
    assert_eq!(
        effective
            .effective_config
            .pointer("/plugins/maxOutputTokens"),
        Some(&json!(2048))
    );
    assert_eq!(
        effective
            .effective_config
            .pointer("/providerFallbacks/primary"),
        Some(&json!("anthropic"))
    );
    assert_eq!(
        effective
            .effective_config
            .pointer("/providerFallbacks/fallbacks/1"),
        Some(&json!("dashscope"))
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn runtime_effective_config_migrates_inline_configured_model_credentials_to_secret_refs() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let workspace_config_path = infra.paths.runtime_config_dir.join("workspace.json");
    write_json(
        &workspace_config_path,
        json!({
            "configuredModels": {
                "anthropic-inline": {
                    "configuredModelId": "anthropic-inline",
                    "name": "Claude Inline",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": "sk-ant-inline-secret",
                    "enabled": true,
                    "source": "workspace"
                }
            },
            "toolCatalog": {
                "disabledSourceKeys": []
            }
        }),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let config = adapter.get_config().await.expect("runtime config");
    let workspace_source = config
        .sources
        .iter()
        .find(|source| source.scope == "workspace")
        .expect("workspace source");
    let workspace_document = workspace_source
        .document
        .as_ref()
        .expect("workspace document");
    let configured_models = workspace_document
        .get("configuredModels")
        .and_then(Value::as_object)
        .expect("configured models");
    let configured_model = configured_models
        .get("anthropic-inline")
        .and_then(Value::as_object)
        .expect("configured model");
    let stored_reference = configured_model
        .get("credentialRef")
        .and_then(Value::as_str)
        .expect("credential ref");

    assert!(stored_reference.starts_with("secret-ref:"));
    assert!(
        config.secret_references.iter().any(|entry| {
            entry.scope == "workspace"
                && entry.path == "configuredModels.anthropic-inline.credentialRef"
                && entry.status == "reference-present"
                && entry.reference.as_deref() == Some(stored_reference)
        }),
        "expected workspace secret reference status to reflect the migrated configured model credential"
    );
    assert!(
        config
            .validation
            .warnings
            .iter()
            .any(|warning| { warning.contains("unknown runtime config key `toolCatalog`") }),
        "expected unrelated unknown keys to remain warnings only"
    );

    let persisted = fs::read_to_string(&workspace_config_path).expect("persisted workspace config");
    assert!(!persisted.contains("sk-ant-inline-secret"));
    assert!(persisted.contains("secret-ref:"));
    assert!(persisted.contains("\"toolCatalog\""));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn runtime_effective_config_redacts_inline_configured_model_credentials_when_migration_fails()
{
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let workspace_config_path = infra.paths.runtime_config_dir.join("workspace.json");
    write_json(
        &workspace_config_path,
        json!({
            "configuredModels": {
                "anthropic-inline": {
                    "configuredModelId": "anthropic-inline",
                    "name": "Claude Inline",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": "sk-ant-inline-secret",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
    );

    let adapter = RuntimeAdapter::new_with_executor_and_secret_store(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
        Arc::new(FailingRuntimeSecretStore),
    );

    let config = adapter.get_config().await.expect("runtime config");
    let workspace_source = config
        .sources
        .iter()
        .find(|source| source.scope == "workspace")
        .expect("workspace source");
    let workspace_document = workspace_source
        .document
        .as_ref()
        .expect("workspace document");
    let stored_reference = workspace_document
        .pointer("/configuredModels/anthropic-inline/credentialRef")
        .and_then(Value::as_str)
        .expect("redacted credential ref");

    assert_eq!(stored_reference, "***");
    assert!(
        config.secret_references.iter().any(|entry| {
            entry.scope == "workspace"
                && entry.path == "configuredModels.anthropic-inline.credentialRef"
                && entry.status == "migration-failed"
        }),
        "expected migration failure to be reported through runtime secret reference status"
    );

    let persisted = fs::read_to_string(&workspace_config_path).expect("persisted workspace config");
    assert!(persisted.contains("sk-ant-inline-secret"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn probe_configured_model_resolves_managed_secret_refs_and_supports_api_key_override() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let workspace_config_path = infra.paths.runtime_config_dir.join("workspace.json");
    write_json(
        &workspace_config_path,
        json!({
            "configuredModels": {
                "anthropic-inline": {
                    "configuredModelId": "anthropic-inline",
                    "name": "Claude Inline",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": "sk-ant-inline-secret",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
    );

    let driver = Arc::new(InspectingPromptRuntimeModelDriver::default());
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        driver.clone(),
    );

    let migrated_probe = adapter
        .probe_configured_model(RuntimeConfiguredModelProbeInput {
            scope: "workspace".into(),
            configured_model_id: "anthropic-inline".into(),
            patch: json!({}),
            api_key: None,
        })
        .await
        .expect("probe configured model");
    assert!(migrated_probe.valid);
    assert!(migrated_probe.reachable);
    assert_eq!(
        driver
            .last_request_policy()
            .and_then(|policy| policy.auth.value),
        Some("sk-ant-inline-secret".into())
    );

    let override_probe = adapter
        .probe_configured_model(RuntimeConfiguredModelProbeInput {
            scope: "workspace".into(),
            configured_model_id: "anthropic-inline".into(),
            patch: json!({}),
            api_key: Some("sk-ant-override-secret".into()),
        })
        .await
        .expect("probe configured model with override");
    assert!(override_probe.valid);
    assert!(override_probe.reachable);
    assert_eq!(
        driver
            .last_request_policy()
            .and_then(|policy| policy.auth.value),
        Some("sk-ant-override-secret".into())
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_updates_configured_model_token_usage_and_catalog_snapshot() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(32),
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-quota",
                "",
                "Quota Session",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(&session.summary.id, turn_input("Count tokens", None))
        .await
        .expect("run");

    assert_eq!(run.consumed_tokens, Some(32));

    let catalog = adapter.catalog_snapshot().await.expect("catalog snapshot");
    let configured_model = catalog
        .configured_models
        .iter()
        .find(|model| model.configured_model_id == "quota-model")
        .expect("configured model");
    assert_eq!(
        configured_model
            .token_quota
            .as_ref()
            .and_then(|quota| quota.total_tokens),
        Some(100)
    );
    assert_eq!(configured_model.token_usage.used_tokens, 32);
    assert_eq!(configured_model.token_usage.remaining_tokens, Some(68));
    assert!(!configured_model.token_usage.exhausted);

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let used_tokens: i64 = connection
            .query_row(
                "SELECT used_tokens FROM configured_model_usage_projections WHERE configured_model_id = ?1",
                ["quota-model"],
                |row| row.get(0),
            )
            .expect("used tokens");
    assert_eq!(used_tokens, 32);
    let cost_configured_model_id: String = connection
            .query_row(
                "SELECT configured_model_id FROM cost_entries WHERE run_id = ?1 ORDER BY created_at DESC LIMIT 1",
                [&run.id],
                |row| row.get(0),
            )
            .expect("cost configured model id");
    assert_eq!(cost_configured_model_id, "quota-model");

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn configured_model_token_usage_survives_adapter_restart() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(24),
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-restart",
                "",
                "Restart Session",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");
    adapter
        .submit_turn(&session.summary.id, turn_input("Persist usage", None))
        .await
        .expect("run");

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(24),
        }),
    );
    let catalog = reloaded.catalog_snapshot().await.expect("catalog snapshot");
    let configured_model = catalog
        .configured_models
        .iter()
        .find(|model| model.configured_model_id == "quota-model")
        .expect("configured model");
    assert_eq!(configured_model.token_usage.used_tokens, 24);
    assert_eq!(configured_model.token_usage.remaining_tokens, Some(76));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_blocks_when_configured_model_token_quota_is_exhausted() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(32),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(32),
        }),
    );

    let first_session = adapter
        .create_session(
            session_input(
                "conv-first",
                "",
                "First Session",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("first session");
    let first_run = adapter
        .submit_turn(
            &first_session.summary.id,
            turn_input("Use the full quota", None),
        )
        .await
        .expect("first run");
    assert_eq!(first_run.consumed_tokens, Some(32));

    let second_session = adapter
        .create_session(
            session_input(
                "conv-second",
                "",
                "Second Session",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("second session");
    let error = adapter
        .submit_turn(
            &second_session.summary.id,
            turn_input("This should be blocked", None),
        )
        .await
        .expect_err("quota exhaustion should block new requests");
    assert!(error
        .to_string()
        .contains("has reached its total token limit"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn session_bound_agent_selection_injects_manifest_prompt_into_execution() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
            .execute(
                "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    "agent-project-delivery",
                    octopus_core::DEFAULT_WORKSPACE_ID,
                    octopus_core::DEFAULT_PROJECT_ID,
                    "project",
                    "Project Delivery Agent",
                    Option::<String>::None,
                    "Structured and pragmatic",
                    serde_json::to_string(&vec!["project", "delivery"]).expect("tags"),
                    "Always answer with an implementation plan first.",
                    serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                    serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                    serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                    "Tracks project work, runtime sessions, and follow-up actions.",
                    "active",
                    timestamp_now() as i64,
                ],
            )
            .expect("upsert agent prompt");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-agent-actor",
                octopus_core::DEFAULT_PROJECT_ID,
                "Agent Actor Session",
                "agent:agent-project-delivery",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(&session.summary.id, turn_input("Design the rollout", None))
        .await
        .expect("run");

    assert_eq!(run.actor_ref, "agent:agent-project-delivery");

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert_eq!(
        detail.summary.selected_actor_ref,
        "agent:agent-project-delivery"
    );
    let assistant_message = detail
        .messages
        .iter()
        .find(|message| message.sender_type == "assistant")
        .expect("assistant message");
    assert!(assistant_message.content.contains("You are the agent `"));
    assert!(assistant_message.content.contains("Project Delivery Agent"));
    assert!(assistant_message
        .content
        .contains("Personality: Structured and pragmatic"));
    assert!(assistant_message
        .content
        .contains("Instructions: Always answer with an implementation plan first."));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_sessions_run_through_runtime_subruns_and_workflow_projection() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
            .execute(
                "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    "agent-orchestrator",
                    octopus_core::DEFAULT_WORKSPACE_ID,
                    Option::<String>::None,
                    "workspace",
                    "Orchestrator Agent",
                    Option::<String>::None,
                    "Systems thinker",
                    serde_json::to_string(&vec!["coordination"]).expect("tags"),
                    "Coordinate the team response.",
                    serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                    serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                    serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                    "Leads team execution.",
                    "active",
                    timestamp_now() as i64,
                ],
            )
            .expect("upsert orchestrator agent");
    connection
            .execute(
                "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    "agent-project-delivery",
                    octopus_core::DEFAULT_WORKSPACE_ID,
                    Option::<String>::None,
                    "workspace",
                    "Project Delivery Agent",
                    Option::<String>::None,
                    "Structured and pragmatic",
                    serde_json::to_string(&vec!["delivery"]).expect("tags"),
                    "Keep project execution on track.",
                    serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                    serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                    serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                    "Supports cross-functional delivery.",
                    "active",
                    timestamp_now() as i64,
                ],
            )
            .expect("upsert delivery agent");
    connection
            .execute(
                "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                params![
                    "team-workspace-core",
                    octopus_core::DEFAULT_WORKSPACE_ID,
                    Option::<String>::None,
                    "workspace",
                    "Workspace Core",
                    Option::<String>::None,
                    "Cross-functional design review board",
                    serde_json::to_string(&vec!["coordination"]).expect("tags"),
                    "Debate options, then return a single aligned answer.",
                    serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                    serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                    serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                    serde_json::to_string(&json!({
                        "toolExecution": "auto",
                        "memoryWrite": "require-approval",
                        "mcpAuth": "require-approval",
                        "teamSpawn": "auto"
                        ,
                        "workflowEscalation": "auto"
                    }))
                    .expect("approval preference"),
                    canonical_test_agent_ref("agent-orchestrator"),
                    canonical_test_member_refs(&["agent-orchestrator", "agent-project-delivery"]),
                    "Core workspace decision board.",
                    "active",
                    timestamp_now() as i64,
                ],
            )
            .expect("upsert workspace core team");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-team-actor",
                octopus_core::DEFAULT_PROJECT_ID,
                "Team Actor Session",
                "team:team-workspace-core",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(&session.summary.id, turn_input("Review the proposal", None))
        .await
        .expect("team runtime should execute through the shared runtime trunk");

    assert_eq!(run.run_kind, "primary");
    assert_eq!(run.actor_ref, "team:team-workspace-core");
    assert!(run.workflow_run.is_some());
    assert!(run.worker_dispatch.is_some());
    assert!(run
        .worker_dispatch
        .as_ref()
        .is_some_and(|dispatch| dispatch.total_subruns >= 2));
    assert!(run.mailbox_ref.is_some());
    assert!(run.background_state.is_some());

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert!(detail.subrun_count >= 2);
    assert_eq!(detail.summary.subrun_count, detail.subrun_count);
    assert!(detail.workflow.is_some());
    assert!(detail.pending_mailbox.is_some());
    assert!(detail.background_run.is_some());
    assert!(detail.handoffs.len() >= 2);
    assert!(detail.subruns.len() >= 2);

    let workflow = detail.workflow.as_ref().expect("workflow summary");
    assert_eq!(workflow.status, "completed");
    assert!(workflow.total_steps >= 3);
    assert!(workflow.completed_steps >= 3);

    let first_subrun = detail.subruns.first().expect("subrun summary");
    assert_eq!(first_subrun.parent_run_id.as_deref(), Some(run.id.as_str()));
    assert!(first_subrun.actor_ref.starts_with("agent:"));
    assert_eq!(first_subrun.run_kind, "subrun");
    let first_subrun_state_path = infra
        .paths
        .runtime_state_dir
        .join("subruns")
        .join(format!("{}.json", first_subrun.run_id));
    assert!(first_subrun_state_path.exists());
    let first_subrun_state: serde_json::Value =
        serde_json::from_slice(&fs::read(&first_subrun_state_path).expect("subrun state bytes"))
            .expect("subrun state json");
    assert_eq!(
        first_subrun_state
            .get("run")
            .and_then(|value| value.get("id"))
            .and_then(serde_json::Value::as_str),
        Some(first_subrun.run_id.as_str())
    );
    assert_eq!(
        first_subrun_state
            .get("run")
            .and_then(|value| value.get("parentRunId"))
            .and_then(serde_json::Value::as_str),
        Some(run.id.as_str())
    );
    assert_eq!(
        first_subrun_state
            .get("run")
            .and_then(|value| value.get("runKind"))
            .and_then(serde_json::Value::as_str),
        Some("subrun")
    );
    assert!(first_subrun_state
        .get("run")
        .and_then(|value| value.get("capabilityStateRef"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(
        first_subrun_state
            .get("dispatch")
            .and_then(|value| value.get("dispatchKey"))
            .and_then(serde_json::Value::as_str),
        Some("team-dispatch-1")
    );
    assert_eq!(
        first_subrun_state
            .get("dispatch")
            .and_then(|value| value.get("workerInput"))
            .and_then(|value| value.get("content"))
            .and_then(serde_json::Value::as_str),
        Some("Review the proposal")
    );
    assert_eq!(
        first_subrun_state
            .get("dispatch")
            .and_then(|value| value.get("mailboxPolicy"))
            .and_then(|value| value.get("mode"))
            .and_then(serde_json::Value::as_str),
        Some("leader-hub")
    );
    assert_eq!(
        first_subrun_state
            .get("dispatch")
            .and_then(|value| value.get("artifactHandoffPolicy"))
            .and_then(|value| value.get("mode"))
            .and_then(serde_json::Value::as_str),
        Some("leader-reviewed")
    );
    assert_eq!(
        first_subrun_state
            .get("serializedSession")
            .and_then(|value| value.get("session"))
            .and_then(|value| value.get("messages"))
            .and_then(serde_json::Value::as_array)
            .and_then(|messages| messages.first())
            .and_then(|value| value.get("blocks"))
            .and_then(serde_json::Value::as_array)
            .and_then(|blocks| blocks.first())
            .and_then(|value| value.get("text"))
            .and_then(serde_json::Value::as_str),
        Some("Review the proposal")
    );

    let mailbox = detail.pending_mailbox.as_ref().expect("mailbox summary");
    assert_eq!(mailbox.channel, "leader-hub");
    assert_eq!(mailbox.status, "completed");
    assert_eq!(mailbox.pending_count, 0);
    assert!(mailbox.total_messages >= 2);
    assert!(detail
        .handoffs
        .iter()
        .all(|handoff| handoff.state == "acknowledged"));
    assert!(infra
        .paths
        .runtime_events_dir
        .join(format!("{}.jsonl", session.summary.id))
        .exists());
    assert!(!infra
        .paths
        .root
        .join("runtime")
        .join("sessions")
        .join(format!("{}.json", session.summary.id))
        .exists());
    assert!(!infra
        .paths
        .root
        .join("runtime")
        .join("sessions")
        .join(format!("{}-events.json", session.summary.id))
        .exists());

    let background = detail.background_run.as_ref().expect("background summary");
    assert_eq!(background.status, "completed");
    assert_eq!(
        background.workflow_run_id.as_deref(),
        run.workflow_run.as_deref()
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let session_projection: (
        Option<String>,
        Option<String>,
        i64,
        i64,
        Option<String>,
        i64,
        i64,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT workflow_run_id, workflow_status, workflow_total_steps, workflow_completed_steps,
                    pending_mailbox_ref, pending_mailbox_count, handoff_count, background_status
             FROM runtime_session_projections
             WHERE id = ?1",
            [&session.summary.id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .expect("phase four session projection");
    assert_eq!(session_projection.0.as_deref(), run.workflow_run.as_deref());
    assert_eq!(session_projection.1.as_deref(), Some("completed"));
    assert!(session_projection.2 >= 3);
    assert!(session_projection.3 >= 3);
    assert_eq!(
        session_projection.4.as_deref(),
        detail
            .pending_mailbox
            .as_ref()
            .map(|mailbox| mailbox.mailbox_ref.as_str())
    );
    assert_eq!(session_projection.5, mailbox.pending_count as i64);
    assert!(session_projection.6 >= 2);
    assert_eq!(session_projection.7.as_deref(), Some("completed"));

    let run_projection: (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
        i64,
        i64,
        i64,
    ) = connection
        .query_row(
            "SELECT workflow_run_id, workflow_step_id, mailbox_ref, handoff_ref, background_state,
                    worker_total_subruns, worker_active_subruns, worker_completed_subruns, worker_failed_subruns
             FROM runtime_run_projections
             WHERE id = ?1",
            [&run.id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            },
        )
        .expect("phase four run projection");
    assert_eq!(run_projection.0.as_deref(), run.workflow_run.as_deref());
    assert!(run_projection.1.is_some());
    assert_eq!(run_projection.2.as_deref(), run.mailbox_ref.as_deref());
    assert!(run_projection.3.is_some());
    assert_eq!(run_projection.4.as_deref(), Some("completed"));
    assert!(run_projection.5 >= 2);
    assert_eq!(run_projection.6, 0);
    assert!(run_projection.7 >= 2);
    assert_eq!(run_projection.8, 0);

    let subrun_projection_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM runtime_subrun_projections WHERE session_id = ?1 AND parent_run_id = ?2",
            params![session.summary.id, run.id],
            |row| row.get(0),
        )
        .expect("subrun projections");
    let handoff_projection_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM runtime_handoff_projections WHERE session_id = ?1 AND run_id = ?2",
            params![session.summary.id, run.id],
            |row| row.get(0),
        )
        .expect("handoff projections");
    let handoff_projection_rows = {
        let mut statement = connection
            .prepare(
                "SELECT handoff_ref, parent_run_id, delegated_by_tool_call_id, sender_actor_ref,
                        receiver_actor_ref, mailbox_ref, state, artifact_refs_json,
                        envelope_storage_path, envelope_content_hash
                 FROM runtime_handoff_projections
                 WHERE session_id = ?1 AND run_id = ?2
                 ORDER BY updated_at ASC, handoff_ref ASC",
            )
            .expect("handoff projection statement");
        statement
            .query_map(params![session.summary.id, run.id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                ))
            })
            .expect("handoff projection rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect handoff projection rows")
    };
    let workflow_projection: (Option<String>, Option<String>) = connection
        .query_row(
            "SELECT detail_storage_path, detail_content_hash
             FROM runtime_workflow_projections
             WHERE workflow_run_id = ?1",
            [run.workflow_run.clone().expect("workflow run id")],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("workflow projection");
    let mailbox_projection: (Option<String>, Option<String>) = connection
        .query_row(
            "SELECT body_storage_path, body_content_hash
             FROM runtime_mailbox_projections
             WHERE mailbox_ref = ?1",
            [mailbox.mailbox_ref.clone()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("mailbox projection");
    let background_projection: (Option<String>, Option<String>) = connection
        .query_row(
            "SELECT state_storage_path, state_content_hash
             FROM runtime_background_projections
             WHERE run_id = ?1",
            [background.run_id.clone()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("background projection");
    assert!(subrun_projection_count >= 2);
    assert!(handoff_projection_count >= 2);
    assert_eq!(
        handoff_projection_rows.len() as i64,
        handoff_projection_count
    );
    let subruns_by_handoff_ref = detail
        .subruns
        .iter()
        .filter_map(|subrun| {
            subrun
                .handoff_ref
                .as_ref()
                .map(|handoff_ref| (handoff_ref.as_str(), subrun))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let handoffs_by_ref = detail
        .handoffs
        .iter()
        .map(|handoff| (handoff.handoff_ref.as_str(), handoff))
        .collect::<std::collections::BTreeMap<_, _>>();
    for (
        handoff_ref,
        parent_run_id,
        delegated_by_tool_call_id,
        sender_actor_ref,
        receiver_actor_ref,
        mailbox_ref,
        state,
        artifact_refs_json,
        envelope_storage_path,
        envelope_content_hash,
    ) in &handoff_projection_rows
    {
        let subrun = subruns_by_handoff_ref
            .get(handoff_ref.as_str())
            .expect("subrun for handoff projection");
        let handoff = handoffs_by_ref
            .get(handoff_ref.as_str())
            .expect("handoff summary for projection");
        assert_eq!(parent_run_id.as_deref(), Some(run.id.as_str()));
        assert_eq!(
            delegated_by_tool_call_id.as_deref(),
            subrun.delegated_by_tool_call_id.as_deref()
        );
        assert_eq!(sender_actor_ref, &subrun.actor_ref);
        assert_eq!(receiver_actor_ref, &run.actor_ref);
        assert_eq!(
            mailbox_ref,
            subrun.mailbox_ref.as_deref().unwrap_or_default()
        );
        assert_eq!(state, &handoff.state);
        let artifact_refs: Vec<String> =
            serde_json::from_str(artifact_refs_json).expect("handoff artifact refs json");
        assert_eq!(&artifact_refs, &handoff.artifact_refs);
        assert!(envelope_storage_path
            .as_deref()
            .is_some_and(|path| root.join(path).exists()));
        assert!(envelope_content_hash
            .as_deref()
            .is_some_and(|hash| hash.starts_with("sha256-")));
    }
    assert!(workflow_projection
        .0
        .as_deref()
        .is_some_and(|path| root.join(path).exists()));
    assert!(workflow_projection
        .1
        .as_deref()
        .is_some_and(|hash| hash.starts_with("sha256-")));
    assert!(mailbox_projection
        .0
        .as_deref()
        .is_some_and(|path| root.join(path).exists()));
    assert!(mailbox_projection
        .1
        .as_deref()
        .is_some_and(|hash| hash.starts_with("sha256-")));
    let mailbox_body_path = root.join(
        mailbox_projection
            .0
            .clone()
            .expect("mailbox projection body path"),
    );
    let mailbox_body: serde_json::Value =
        serde_json::from_slice(&fs::read(&mailbox_body_path).expect("mailbox body bytes"))
            .expect("mailbox body json");
    let mailbox_handoffs = mailbox_body
        .get("handoffs")
        .and_then(serde_json::Value::as_array)
        .expect("mailbox handoff lineage array");
    assert_eq!(mailbox_handoffs.len(), detail.handoffs.len());
    for handoff_entry in mailbox_handoffs {
        let handoff_ref = handoff_entry
            .get("handoffRef")
            .and_then(serde_json::Value::as_str)
            .expect("mailbox handoff ref");
        let subrun = subruns_by_handoff_ref
            .get(handoff_ref)
            .expect("subrun for mailbox handoff");
        let handoff = handoffs_by_ref
            .get(handoff_ref)
            .expect("handoff summary for mailbox handoff");
        assert_eq!(
            handoff_entry
                .get("parentRunId")
                .and_then(serde_json::Value::as_str),
            Some(run.id.as_str())
        );
        assert_eq!(
            handoff_entry
                .get("delegatedByToolCallId")
                .and_then(serde_json::Value::as_str),
            subrun.delegated_by_tool_call_id.as_deref()
        );
        assert_eq!(
            handoff_entry
                .get("senderActorRef")
                .and_then(serde_json::Value::as_str),
            Some(subrun.actor_ref.as_str())
        );
        assert_eq!(
            handoff_entry
                .get("receiverActorRef")
                .and_then(serde_json::Value::as_str),
            Some(run.actor_ref.as_str())
        );
        assert_eq!(
            handoff_entry
                .get("mailboxRef")
                .and_then(serde_json::Value::as_str),
            subrun.mailbox_ref.as_deref()
        );
        assert_eq!(
            handoff_entry
                .get("handoffState")
                .and_then(serde_json::Value::as_str),
            Some(handoff.state.as_str())
        );
        assert_eq!(
            handoff_entry
                .get("artifactRefs")
                .and_then(serde_json::Value::as_array)
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .expect("mailbox handoff artifact refs"),
            handoff.artifact_refs
        );
    }
    assert!(background_projection
        .0
        .as_deref()
        .is_some_and(|path| root.join(path).exists()));
    assert!(background_projection
        .1
        .as_deref()
        .is_some_and(|hash| hash.starts_with("sha256-")));
    let corrupted_handoff_ref = handoff_projection_rows
        .first()
        .map(|row| row.0.clone())
        .expect("handoff projection row");
    connection
        .execute(
            "UPDATE runtime_handoff_projections
             SET summary_json = ?2
             WHERE handoff_ref = ?1",
            params![corrupted_handoff_ref, "{invalid-handoff-summary"],
        )
        .expect("corrupt handoff summary json");
    let reloaded_from_corrupt_handoff = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );
    let corrupt_handoff_detail = reloaded_from_corrupt_handoff
        .get_session(&session.summary.id)
        .await
        .expect("reload detail from handoff envelope fallback");
    assert_eq!(corrupt_handoff_detail.handoffs.len(), detail.handoffs.len());
    assert_eq!(
        corrupt_handoff_detail
            .handoffs
            .iter()
            .find(|handoff| handoff.handoff_ref == corrupted_handoff_ref)
            .expect("corrupt handoff reloaded")
            .artifact_refs,
        detail
            .handoffs
            .iter()
            .find(|handoff| handoff.handoff_ref == corrupted_handoff_ref)
            .expect("original handoff summary")
            .artifact_refs
    );
    let corrupted_handoff_envelope_path = root.join(
        handoff_projection_rows
            .first()
            .and_then(|row| row.8.clone())
            .expect("handoff envelope path"),
    );
    fs::write(
        &corrupted_handoff_envelope_path,
        b"{invalid-handoff-envelope",
    )
    .expect("corrupt handoff envelope json");
    let reloaded_from_corrupt_handoff_envelope = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );
    let corrupt_handoff_envelope_detail = reloaded_from_corrupt_handoff_envelope
        .get_session(&session.summary.id)
        .await
        .expect("reload detail from handoff projection fallback");
    assert_eq!(
        corrupt_handoff_envelope_detail.handoffs.len(),
        detail.handoffs.len()
    );
    assert_eq!(
        corrupt_handoff_envelope_detail
            .handoffs
            .iter()
            .find(|handoff| handoff.handoff_ref == corrupted_handoff_ref)
            .expect("corrupt handoff reloaded from projection row")
            .artifact_refs,
        detail
            .handoffs
            .iter()
            .find(|handoff| handoff.handoff_ref == corrupted_handoff_ref)
            .expect("original handoff summary")
            .artifact_refs
    );
    let artifact_refs = detail
        .handoffs
        .iter()
        .flat_map(|handoff| handoff.artifact_refs.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(!artifact_refs.is_empty());
    for artifact_ref in &artifact_refs {
        assert!(!artifact_ref.starts_with("artifact-"));
        let artifact_projection: (
            String,
            String,
            i64,
            String,
            String,
            Option<String>,
            Option<String>,
        ) = connection
            .query_row(
                "SELECT storage_path, content_hash, byte_size, content_type, actor_ref, parent_run_id, delegated_by_tool_call_id
                 FROM runtime_artifact_projections
                 WHERE artifact_ref = ?1",
                [artifact_ref],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .expect("runtime artifact projection");
        assert!(root.join(&artifact_projection.0).exists());
        assert!(artifact_projection.1.starts_with("sha256-"));
        assert!(artifact_projection.2 > 0);
        assert_eq!(artifact_projection.3, "application/json");
        assert!(
            artifact_projection.4.starts_with("agent:")
                || artifact_projection.4.starts_with("team:")
        );
        assert_eq!(artifact_projection.5.as_deref(), Some(run.id.as_str()));
        assert!(artifact_projection.6.is_some());
    }

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    let workflow_events = events
        .iter()
        .filter_map(|event| event.kind.clone())
        .filter(|kind| kind.starts_with("workflow."))
        .collect::<Vec<_>>();
    assert!(workflow_events
        .iter()
        .any(|kind| kind == "workflow.started"));
    assert!(workflow_events
        .iter()
        .any(|kind| kind == "workflow.step.started"));
    assert!(workflow_events
        .iter()
        .any(|kind| kind == "workflow.step.completed"));
    assert!(workflow_events
        .iter()
        .any(|kind| kind == "workflow.completed"));
    let spawned_subruns = events
        .iter()
        .filter(|event| event.kind.as_deref() == Some("subrun.spawned"))
        .collect::<Vec<_>>();
    let completed_subruns = events
        .iter()
        .filter(|event| event.kind.as_deref() == Some("subrun.completed"))
        .collect::<Vec<_>>();
    assert_eq!(spawned_subruns.len(), detail.subruns.len());
    assert_eq!(completed_subruns.len(), detail.subruns.len());
    let first_subrun_event = spawned_subruns.first().expect("subrun spawned event");
    assert_eq!(first_subrun_event.run_id.as_deref(), Some(run.id.as_str()));
    assert_eq!(
        first_subrun_event.parent_run_id.as_deref(),
        Some(run.id.as_str())
    );
    assert_eq!(
        first_subrun_event.workflow_run_id.as_deref(),
        run.workflow_run.as_deref()
    );
    assert!(first_subrun_event
        .actor_ref
        .as_deref()
        .is_some_and(|actor_ref| actor_ref.starts_with("agent:")));
    assert!(first_subrun_event.tool_use_id.is_some());

    let mut mutated_subrun_state = first_subrun_state;
    mutated_subrun_state["run"]["status"] = json!("failed");
    mutated_subrun_state["run"]["currentStep"] = json!("failed");
    mutated_subrun_state["run"]["updatedAt"] = json!(first_subrun.updated_at + 17);
    mutated_subrun_state["run"]["nextAction"] = json!("idle");
    fs::write(
        &first_subrun_state_path,
        serde_json::to_vec_pretty(&mutated_subrun_state).expect("mutated subrun state bytes"),
    )
    .expect("overwrite subrun state");

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );
    let reloaded_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("reloaded session detail");
    assert_eq!(reloaded_detail.subrun_count, detail.subrun_count);
    assert_eq!(reloaded_detail.subruns.len(), detail.subruns.len());
    assert_eq!(reloaded_detail.handoffs.len(), detail.handoffs.len());
    assert_eq!(
        reloaded_detail
            .workflow
            .as_ref()
            .map(|workflow| workflow.workflow_run_id.as_str()),
        detail
            .workflow
            .as_ref()
            .map(|workflow| workflow.workflow_run_id.as_str())
    );
    assert_eq!(
        reloaded_detail
            .pending_mailbox
            .as_ref()
            .map(|mailbox| mailbox.mailbox_ref.as_str()),
        detail
            .pending_mailbox
            .as_ref()
            .map(|mailbox| mailbox.mailbox_ref.as_str())
    );
    assert_eq!(
        reloaded_detail
            .background_run
            .as_ref()
            .and_then(|background| background.workflow_run_id.as_deref()),
        detail
            .background_run
            .as_ref()
            .and_then(|background| background.workflow_run_id.as_deref())
    );
    let reloaded_first_subrun = reloaded_detail
        .subruns
        .iter()
        .find(|subrun| subrun.run_id == first_subrun.run_id)
        .expect("reloaded first subrun");
    assert_eq!(reloaded_first_subrun.status, "failed");
    assert!(reloaded_detail
        .run
        .worker_dispatch
        .as_ref()
        .is_some_and(|dispatch| dispatch.failed_subruns >= 1));
    assert_eq!(
        reloaded_detail
            .workflow
            .as_ref()
            .map(|workflow| workflow.status.as_str()),
        Some("failed")
    );
    assert_eq!(
        reloaded_detail
            .background_run
            .as_ref()
            .map(|background| background.status.as_str()),
        Some("failed")
    );
    assert!(reloaded_detail
        .pending_mailbox
        .as_ref()
        .is_some_and(|mailbox| mailbox.status == "failed" && mailbox.pending_count == 0));
    assert!(reloaded_detail.handoffs.iter().any(|handoff| {
        handoff.handoff_ref
            == reloaded_first_subrun
                .handoff_ref
                .clone()
                .expect("handoff ref")
            && handoff.state == "failed"
    }));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_runtime_uses_manifest_mailbox_policy_and_keeps_all_worker_subruns_beyond_concurrency_ceiling(
) {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    for (id, name) in [
        ("agent-scheduler-leader", "Scheduler Leader"),
        ("agent-scheduler-worker-a", "Scheduler Worker A"),
        ("agent-scheduler-worker-b", "Scheduler Worker B"),
        ("agent-scheduler-worker-c", "Scheduler Worker C"),
    ] {
        connection
            .execute(
                "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    id,
                    octopus_core::DEFAULT_WORKSPACE_ID,
                    Option::<String>::None,
                    "workspace",
                    name,
                    Option::<String>::None,
                    "Cooperative",
                    serde_json::to_string(&vec!["coordination"]).expect("tags"),
                    "Handle delegated work.",
                    serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                    serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                    serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                    "Worker scheduling test agent.",
                    "active",
                    timestamp_now() as i64,
                ],
            )
            .expect("upsert scheduling agent");
    }
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, delegation_policy_json, approval_preference_json, leader_ref, member_refs, mailbox_policy_json, worker_concurrency_limit, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "team-scheduler-policy",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Scheduler Policy Team",
                Option::<String>::None,
                "Scheduling aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Delegate the work across all available workers.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "mode": "leader-orchestrated",
                    "allowBackgroundRuns": true,
                    "allowParallelWorkers": true,
                    "maxWorkerCount": 3
                }))
                .expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                canonical_test_agent_ref("agent-scheduler-leader"),
                canonical_test_member_refs(&[
                    "agent-scheduler-worker-a",
                    "agent-scheduler-worker-b",
                    "agent-scheduler-worker-c",
                ]),
                serde_json::to_string(&json!({
                    "mode": "worker-direct",
                    "allowWorkerToWorker": true,
                    "retainMessages": true
                }))
                .expect("mailbox policy"),
                1_i64,
                "Team for scheduler and mailbox policy tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert scheduler team");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-scheduler-policy",
                octopus_core::DEFAULT_PROJECT_ID,
                "Scheduler Policy Session",
                "team:team-scheduler-policy",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(&session.summary.id, turn_input("Distribute the task", None))
        .await
        .expect("team run");

    assert_eq!(
        run.worker_dispatch
            .as_ref()
            .map(|dispatch| dispatch.total_subruns),
        Some(3)
    );

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("detail");
    assert_eq!(detail.subruns.len(), 3);
    assert_eq!(
        detail
            .pending_mailbox
            .as_ref()
            .map(|mailbox| mailbox.channel.as_str()),
        Some("worker-direct")
    );
    assert!(detail
        .subruns
        .iter()
        .any(|subrun| subrun.actor_ref == "agent:agent-scheduler-worker-c"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn runtime_persistence_writes_jsonl_events_without_legacy_debug_session_files() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-no-legacy-debug",
                octopus_core::DEFAULT_PROJECT_ID,
                "No Legacy Debug Persistence",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Confirm runtime persistence", None),
        )
        .await
        .expect("run");

    assert!(infra
        .paths
        .runtime_events_dir
        .join(format!("{}.jsonl", session.summary.id))
        .exists());
    assert!(!infra
        .paths
        .root
        .join("runtime")
        .join("sessions")
        .join(format!("{}.json", session.summary.id))
        .exists());
    assert!(!infra
        .paths
        .root
        .join("runtime")
        .join("sessions")
        .join(format!("{}-events.json", session.summary.id))
        .exists());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn mixed_domain_team_workers_share_the_same_subrun_runtime_substrate() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_external_plugin(&root, "sample-plugin", "sample-plugin", "plugin_echo");
    write_workspace_config_with_plugins(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
        json!({
            "sample-plugin@external": true
        }),
        &["./external-plugins"],
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-mixed-domain-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Mixed Domain Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Coordinate the coding and research workers.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for mixed-domain runtime tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert mixed-domain leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-mixed-domain-coder",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Mixed Domain Coder",
                Option::<String>::None,
                "Builder",
                serde_json::to_string(&vec!["coding"]).expect("tags"),
                "Implement and validate the change.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Coding worker for mixed-domain runtime tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert mixed-domain coder");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, task_domains, description, capability_policy_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                "agent-mixed-domain-research",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Mixed Domain Researcher",
                Option::<String>::None,
                "Evidence-driven researcher",
                serde_json::to_string(&vec!["research", "docs"]).expect("tags"),
                "Discover supporting context and summarize the findings.",
                serde_json::to_string(&vec!["ToolSearch"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&vec!["research", "docs"]).expect("task domains"),
                "Research/docs worker for mixed-domain runtime tests.",
                serde_json::to_string(&json!({
                    "pluginCapabilityRefs": ["plugin_echo"]
                }))
                .expect("capability policy"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert mixed-domain research worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                "team-mixed-domain-runtime",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Mixed Domain Runtime Team",
                Option::<String>::None,
                "Cross-domain execution team",
                serde_json::to_string(&vec!["coordination", "delivery"]).expect("tags"),
                "Coordinate the coding and research workers, then return one answer.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                canonical_test_agent_ref("agent-mixed-domain-leader"),
                canonical_test_member_refs(&[
                    "agent-mixed-domain-leader",
                    "agent-mixed-domain-coder",
                    "agent-mixed-domain-research",
                ]),
                "Team for mixed-domain subrun runtime tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert mixed-domain team");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-mixed-domain-runtime",
                octopus_core::DEFAULT_PROJECT_ID,
                "Mixed Domain Runtime Session",
                "team:team-mixed-domain-runtime",
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
            turn_input("Implement the change and gather supporting research", None),
        )
        .await
        .expect("mixed-domain team run");

    assert_eq!(run.status, "completed");
    assert_eq!(run.run_kind, "primary");
    assert_eq!(run.actor_ref, "team:team-mixed-domain-runtime");
    assert!(run.workflow_run.is_some());
    assert!(run
        .worker_dispatch
        .as_ref()
        .is_some_and(|dispatch| dispatch.total_subruns >= 3));

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert!(detail.workflow.is_some());
    assert!(detail.pending_mailbox.is_some());
    assert!(detail.background_run.is_some());

    let coding_subrun = detail
        .subruns
        .iter()
        .find(|subrun| subrun.actor_ref == "agent:agent-mixed-domain-coder")
        .expect("coding subrun");
    let research_subrun = detail
        .subruns
        .iter()
        .find(|subrun| subrun.actor_ref == "agent:agent-mixed-domain-research")
        .expect("research subrun");

    assert_eq!(
        coding_subrun.parent_run_id.as_deref(),
        Some(run.id.as_str())
    );
    assert_eq!(
        research_subrun.parent_run_id.as_deref(),
        Some(run.id.as_str())
    );
    assert_eq!(coding_subrun.run_kind, "subrun");
    assert_eq!(research_subrun.run_kind, "subrun");
    assert_eq!(
        coding_subrun.workflow_run_id,
        research_subrun.workflow_run_id
    );
    assert_eq!(coding_subrun.mailbox_ref, research_subrun.mailbox_ref);
    assert!(coding_subrun.handoff_ref.is_some());
    assert!(research_subrun.handoff_ref.is_some());

    let coding_state: team_runtime::PersistedSubrunState = serde_json::from_slice(
        &fs::read(
            infra
                .paths
                .runtime_state_dir
                .join("subruns")
                .join(format!("{}.json", coding_subrun.run_id)),
        )
        .expect("coding subrun state"),
    )
    .expect("parse coding subrun state");
    let research_state: team_runtime::PersistedSubrunState = serde_json::from_slice(
        &fs::read(
            infra
                .paths
                .runtime_state_dir
                .join("subruns")
                .join(format!("{}.json", research_subrun.run_id)),
        )
        .expect("research subrun state"),
    )
    .expect("parse research subrun state");

    assert_eq!(coding_state.run.run_kind, "subrun");
    assert_eq!(research_state.run.run_kind, "subrun");
    assert_eq!(
        coding_state.dispatch.parent_actor_ref,
        "team:team-mixed-domain-runtime"
    );
    assert_eq!(
        research_state.dispatch.parent_actor_ref,
        "team:team-mixed-domain-runtime"
    );
    assert_eq!(
        coding_state.dispatch.workflow_run_id,
        research_state.dispatch.workflow_run_id
    );
    assert!(coding_state
        .run
        .capability_plan_summary
        .visible_tools
        .contains(&"bash".to_string()));
    assert!(research_state
        .run
        .capability_plan_summary
        .visible_tools
        .contains(&"ToolSearch".to_string()));
    assert!(research_state
        .run
        .capability_plan_summary
        .deferred_tools
        .contains(&"plugin_echo".to_string()));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_sessions_reload_team_state_from_subrun_artifacts_when_phase_four_projections_are_missing(
) {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-runtime-reload-worker",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Runtime Reload Worker",
                Option::<String>::None,
                "Reliable reviewer",
                serde_json::to_string(&vec!["workspace", "review"]).expect("tags"),
                "Review the proposal and report the result.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker used by runtime reload tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert runtime reload worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                "team-runtime-reload-core",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Runtime Reload Core",
                Option::<String>::None,
                "Governance team",
                serde_json::to_string(&vec!["workspace", "governance"]).expect("tags"),
                "Maintain workspace-wide standards and governance.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "auto"
                    ,
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                canonical_test_agent_ref("agent-runtime-reload-worker"),
                canonical_test_member_refs(&["agent-runtime-reload-worker"]),
                "Workspace core team for reload tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("set team spawn auto");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-team-reload-artifacts",
                octopus_core::DEFAULT_PROJECT_ID,
                "Team Artifact Reload",
                "team:team-runtime-reload-core",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(&session.summary.id, turn_input("Review the proposal", None))
        .await
        .expect("team runtime should execute through the shared runtime trunk");
    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    let original_subrun_count = detail.subruns.len();
    let original_handoff_count = detail.handoffs.len();
    assert!(original_subrun_count >= 1);
    assert!(original_handoff_count >= 1);
    let first_subrun = detail.subruns.first().expect("subrun summary");
    let first_subrun_state_path = infra
        .paths
        .runtime_state_dir
        .join("subruns")
        .join(format!("{}.json", first_subrun.run_id));
    let mut first_subrun_state: serde_json::Value =
        serde_json::from_slice(&fs::read(&first_subrun_state_path).expect("subrun state bytes"))
            .expect("subrun state json");
    first_subrun_state["run"]["status"] = json!("failed");
    first_subrun_state["run"]["currentStep"] = json!("failed");
    first_subrun_state["run"]["updatedAt"] = json!(first_subrun.updated_at + 33);
    fs::write(
        &first_subrun_state_path,
        serde_json::to_vec_pretty(&first_subrun_state).expect("mutated subrun state bytes"),
    )
    .expect("overwrite subrun state");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "DELETE FROM runtime_subrun_projections WHERE session_id = ?1 AND parent_run_id = ?2",
            params![session.summary.id, run.id],
        )
        .expect("delete subrun projections");
    connection
        .execute(
            "DELETE FROM runtime_handoff_projections WHERE session_id = ?1 AND run_id = ?2",
            params![session.summary.id, run.id],
        )
        .expect("delete handoff projections");
    connection
        .execute(
            "DELETE FROM runtime_mailbox_projections WHERE session_id = ?1 AND run_id = ?2",
            params![session.summary.id, run.id],
        )
        .expect("delete mailbox projections");
    connection
        .execute(
            "DELETE FROM runtime_workflow_projections WHERE session_id = ?1 AND run_id = ?2",
            params![session.summary.id, run.id],
        )
        .expect("delete workflow projections");
    connection
        .execute(
            "DELETE FROM runtime_background_projections WHERE session_id = ?1 AND run_id = ?2",
            params![session.summary.id, run.id],
        )
        .expect("delete background projections");

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );
    let reloaded_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("reloaded session detail");

    assert_eq!(reloaded_detail.subruns.len(), original_subrun_count);
    assert_eq!(reloaded_detail.handoffs.len(), original_handoff_count);
    assert!(reloaded_detail.pending_mailbox.is_some());
    assert!(reloaded_detail.workflow.is_some());
    assert!(reloaded_detail.background_run.is_some());
    assert!(reloaded_detail
        .subruns
        .iter()
        .any(|subrun| subrun.run_id == first_subrun.run_id && subrun.status == "failed"));
    assert_eq!(
        reloaded_detail
            .workflow
            .as_ref()
            .map(|workflow| workflow.status.as_str()),
        Some("failed")
    );
    assert_eq!(
        reloaded_detail
            .background_run
            .as_ref()
            .map(|background| background.status.as_str()),
        Some("failed")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn runtime_snapshot_loaders_ignore_legacy_runtime_sessions_artifacts() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-no-legacy-snapshot-fallback",
                octopus_core::DEFAULT_PROJECT_ID,
                "No Legacy Snapshot Fallback",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");
    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");

    let policy_ref = format!("{}-policy", session.summary.id);
    let policy_path = infra
        .paths
        .runtime_state_dir
        .join(format!("{policy_ref}.json"));
    let legacy_policy_path =
        legacy_runtime_sessions_dir(&infra.paths.root).join(format!("{policy_ref}.json"));
    if let Some(parent) = legacy_policy_path.parent() {
        fs::create_dir_all(parent).expect("legacy policy dir");
    }
    fs::copy(&policy_path, &legacy_policy_path).expect("copy legacy policy snapshot");
    fs::remove_file(&policy_path).expect("remove runtime policy snapshot");
    assert!(adapter.load_session_policy_snapshot(&policy_ref).is_err());

    let manifest_ref = format!("{}-manifest", session.summary.id);
    let manifest_path = infra
        .paths
        .runtime_state_dir
        .join(format!("{manifest_ref}.json"));
    let legacy_manifest_path =
        legacy_runtime_sessions_dir(&infra.paths.root).join(format!("{manifest_ref}.json"));
    if let Some(parent) = legacy_manifest_path.parent() {
        fs::create_dir_all(parent).expect("legacy manifest dir");
    }
    fs::copy(&manifest_path, &legacy_manifest_path).expect("copy legacy manifest snapshot");
    fs::remove_file(&manifest_path).expect("remove runtime manifest snapshot");
    assert!(adapter.load_actor_manifest_snapshot(&manifest_ref).is_err());

    let capability_state_ref = detail
        .capability_state_ref
        .clone()
        .expect("capability state ref");
    let capability_path = infra
        .paths
        .runtime_state_dir
        .join(format!("{capability_state_ref}.json"));
    let legacy_capability_path =
        legacy_runtime_sessions_dir(&infra.paths.root).join(format!("{capability_state_ref}.json"));
    if let Some(parent) = legacy_capability_path.parent() {
        fs::create_dir_all(parent).expect("legacy capability dir");
    }
    fs::copy(&capability_path, &legacy_capability_path).expect("copy legacy capability state");
    fs::remove_file(&capability_path).expect("remove runtime capability state");
    assert!(adapter
        .load_capability_state_snapshot(Some(&capability_state_ref))
        .expect("capability snapshot load")
        .is_none());
    let capability_store = adapter
        .load_capability_store(Some(&capability_state_ref))
        .expect("capability store load");
    assert!(capability_store.snapshot().granted_tools().is_empty());

    let runtime_artifact_storage_path = "runtime/state/legacy-artifact-only.json";
    fs::write(
        legacy_runtime_sessions_dir(&infra.paths.root).join("legacy-artifact-only.json"),
        serde_json::to_vec_pretty(&json!({
            "state": "legacy-only"
        }))
        .expect("legacy artifact json"),
    )
    .expect("write legacy artifact");
    assert!(adapter
        .load_runtime_artifact::<serde_json::Value>(Some(runtime_artifact_storage_path))
        .expect("runtime artifact load")
        .is_none());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_session_reload_ignores_legacy_runtime_sessions_subrun_artifacts() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-legacy-recovery-worker",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Legacy Recovery Worker",
                Option::<String>::None,
                "Reliable reviewer",
                serde_json::to_string(&vec!["workspace", "review"]).expect("tags"),
                "Review the proposal and report the result.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker used by legacy recovery fence tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert legacy recovery worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                "team-legacy-recovery-fence",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Legacy Recovery Fence Team",
                Option::<String>::None,
                "Governance team",
                serde_json::to_string(&vec!["workspace", "governance"]).expect("tags"),
                "Maintain workspace-wide standards and governance.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                canonical_test_agent_ref("agent-legacy-recovery-worker"),
                canonical_test_member_refs(&["agent-legacy-recovery-worker"]),
                "Team for legacy recovery fence tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-team-no-legacy-subrun-fallback",
                octopus_core::DEFAULT_PROJECT_ID,
                "Team No Legacy Subrun Fallback",
                "team:team-legacy-recovery-fence",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(&session.summary.id, turn_input("Review the proposal", None))
        .await
        .expect("team runtime should execute");
    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    let first_subrun = detail.subruns.first().expect("subrun summary");
    let original_status = first_subrun.status.clone();
    let original_updated_at = first_subrun.updated_at;
    let runtime_state_path = infra
        .paths
        .runtime_state_dir
        .join("subruns")
        .join(format!("{}.json", first_subrun.run_id));
    let legacy_state_path = legacy_runtime_sessions_dir(&infra.paths.root)
        .join("subruns")
        .join(format!("{}.json", first_subrun.run_id));
    let mut legacy_state: serde_json::Value =
        serde_json::from_slice(&fs::read(&runtime_state_path).expect("runtime subrun state"))
            .expect("legacy state json");
    legacy_state["run"]["status"] = json!("failed");
    legacy_state["run"]["currentStep"] = json!("failed");
    legacy_state["run"]["updatedAt"] = json!(original_updated_at + 99);
    if let Some(parent) = legacy_state_path.parent() {
        fs::create_dir_all(parent).expect("legacy subrun dir");
    }
    fs::write(
        &legacy_state_path,
        serde_json::to_vec_pretty(&legacy_state).expect("legacy subrun state bytes"),
    )
    .expect("write legacy subrun state");
    fs::remove_file(&runtime_state_path).expect("remove runtime subrun state");

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );
    let reloaded_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("reloaded session detail");
    let reloaded_subrun = reloaded_detail
        .subruns
        .iter()
        .find(|subrun| subrun.run_id == first_subrun.run_id)
        .expect("reloaded subrun");

    assert_eq!(reloaded_subrun.status, original_status);
    assert_ne!(reloaded_subrun.status, "failed");
    assert_eq!(
        reloaded_detail
            .run
            .worker_dispatch
            .as_ref()
            .map(|dispatch| dispatch.total_subruns),
        Some(detail.subruns.len() as u64)
    );

    let _ = run;
    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_subrun_policy_snapshots_recompile_worker_target_decisions() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "agent-team-policy-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Policy Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead team execution.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for team policy snapshot tests.",
                serde_json::to_string(&json!({})).expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert policy leader agent");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "agent-team-policy-worker",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Policy Worker",
                Option::<String>::None,
                "Reviewer",
                serde_json::to_string(&vec!["delivery"]).expect("tags"),
                "Run delegated execution with approval for tool execution.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker for team policy snapshot tests.",
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "require-approval",
                    "workflowEscalation": "require-approval"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert policy worker agent");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                "team-policy-snapshot",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Policy Snapshot Team",
                Option::<String>::None,
                "Policy aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Delegate work to the worker.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "auto"
                    ,
                    "workflowEscalation": "require-approval"
                }))
                .expect("approval preference"),
                canonical_test_agent_ref("agent-team-policy-leader"),
                canonical_test_member_refs(&[
                    "agent-team-policy-leader",
                    "agent-team-policy-worker",
                ]),
                "Team for worker policy snapshot tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert policy snapshot team");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-team-policy-snapshot",
                octopus_core::DEFAULT_PROJECT_ID,
                "Team Policy Snapshot Session",
                "team:team-policy-snapshot",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Delegate the review to the worker", None),
        )
        .await
        .expect("run");

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    let worker_subrun = detail
        .subruns
        .iter()
        .find(|subrun| subrun.actor_ref == "agent:agent-team-policy-worker")
        .expect("worker subrun");

    let worker_policy = adapter
        .load_session_policy_snapshot(&format!("{}-policy", worker_subrun.run_id))
        .expect("worker policy snapshot");
    assert_eq!(
        worker_policy.selected_actor_ref,
        "agent:agent-team-policy-worker"
    );
    assert!(worker_policy
        .target_decisions
        .contains_key("team-spawn:agent:agent-team-policy-worker"));
    assert!(!worker_policy
        .target_decisions
        .contains_key("team-spawn:agent:agent-team-policy-leader"));

    let execution_policy = worker_policy
        .target_decisions
        .get("model-execution:quota-model")
        .expect("model execution target policy");
    assert_eq!(execution_policy.target_kind, "model-execution");
    assert_eq!(execution_policy.action, "requireApproval");
    assert!(execution_policy.deferred);
    assert!(execution_policy.requires_approval);
    assert_eq!(
        execution_policy.required_permission.as_deref(),
        Some("read-only")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_spawn_approval_blocks_subrun_dispatch_until_resolved() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-spawn-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Spawn Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the team.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for team spawn approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team spawn leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-spawn-worker",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Spawn Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["delivery"]).expect("tags"),
                "Do the delegated work.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker for team spawn approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team spawn worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                "team-spawn-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Spawn Approval Team",
                Option::<String>::None,
                "Approval aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Delegate after approval.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "require-approval"
                    ,
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                canonical_test_agent_ref("agent-team-spawn-leader"),
                canonical_test_member_refs(&[
                    "agent-team-spawn-leader",
                    "agent-team-spawn-worker",
                ]),
                "Team for team spawn approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team spawn approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Delegation plan ready.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 3,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Leader subrun completed the delegated task.".into(),
            ),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Worker subrun completed the delegated task.".into(),
            ),
            runtime::AssistantEvent::MessageStop,
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
                "conv-team-spawn-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "Team Spawn Approval",
                "team:team-spawn-approval",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(&session.summary.id, turn_input("Delegate the task", None))
        .await
        .expect("pending team spawn approval");

    assert_eq!(run.status, "waiting_approval");
    assert_eq!(run.current_step, "awaiting_approval");
    assert_eq!(
        run.pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("team-spawn")
    );
    assert_eq!(executor.request_count(), 1);

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert!(detail.subruns.is_empty());
    assert_eq!(
        detail
            .pending_approval
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("team-spawn")
    );
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
    assert_eq!(executor.request_count(), 3);

    let resolved_detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("resolved session detail");
    assert!(resolved_detail.pending_approval.is_none());
    assert!(resolved_detail.subruns.len() >= 2);
    assert!(resolved_detail
        .subruns
        .iter()
        .all(|subrun| subrun.status == "completed"));
    assert!(resolved_detail
        .subruns
        .iter()
        .any(|subrun| subrun.actor_ref == "agent:agent-team-spawn-worker"));
    assert!(resolved_detail.workflow.is_some());
    assert!(resolved_detail.background_run.is_some());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn workflow_continuation_approval_blocks_workflow_projection_until_resolved() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-workflow-approval-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Workflow Approval Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the workflow.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for workflow continuation approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workflow approval leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-workflow-approval-worker",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Workflow Approval Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["delivery"]).expect("tags"),
                "Do the delegated work.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker for workflow continuation approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workflow approval worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                "team-workflow-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Workflow Approval Team",
                Option::<String>::None,
                "Approval aware workflow team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Delegate, then continue the workflow after approval.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "auto",
                    "workflowEscalation": "require-approval"
                }))
                .expect("approval preference"),
                canonical_test_agent_ref("agent-workflow-approval-leader"),
                canonical_test_member_refs(&[
                    "agent-workflow-approval-leader",
                    "agent-workflow-approval-worker",
                ]),
                "Team for workflow continuation approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workflow approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Workflow plan ready.".into()),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Workflow leader subrun completed its step.".into()),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Workflow worker subrun completed its step.".into()),
            runtime::AssistantEvent::MessageStop,
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
                "conv-workflow-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "Workflow Continuation Approval",
                "team:team-workflow-approval",
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
            turn_input("Continue the workflow", None),
        )
        .await
        .expect("pending workflow continuation approval");

    assert_eq!(run.status, "waiting_approval");
    assert_eq!(run.current_step, "awaiting_approval");
    assert_eq!(
        run.pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("workflow-continuation")
    );
    assert_eq!(executor.request_count(), 1);
    assert!(run
        .worker_dispatch
        .as_ref()
        .is_some_and(|dispatch| dispatch.total_subruns >= 1));
    assert!(run.workflow_run.is_some());
    assert!(run.background_state.is_some());

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert!(detail.subrun_count >= 1);
    assert!(!detail.subruns.is_empty());
    assert!(detail.pending_mailbox.is_some());
    assert!(detail.workflow.is_some());
    assert!(detail.background_run.is_some());
    let initial_events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("initial workflow events");
    assert!(initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.started")));
    assert!(initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.step.started")));
    let initial_background_started = initial_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("background.started"))
        .expect("background started event");
    assert_eq!(
        initial_background_started.run_id.as_deref(),
        Some(run.id.as_str())
    );
    assert_eq!(
        initial_background_started.workflow_run_id.as_deref(),
        run.workflow_run.as_deref()
    );
    assert_eq!(
        initial_background_started.outcome.as_deref(),
        Some("paused")
    );
    let initial_background_paused = initial_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("background.paused"))
        .expect("background paused event");
    assert_eq!(
        initial_background_paused.run_id.as_deref(),
        Some(run.id.as_str())
    );
    assert_eq!(
        initial_background_paused.workflow_run_id.as_deref(),
        run.workflow_run.as_deref()
    );
    assert_eq!(initial_background_paused.outcome.as_deref(), Some("paused"));
    assert!(!initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.step.completed")));
    assert!(!initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.completed")));
    assert_eq!(
        initial_events
            .iter()
            .filter(|event| event.kind.as_deref() == Some("subrun.completed"))
            .count(),
        0
    );
    let replay_after = initial_events
        .last()
        .map(|event| event.id.clone())
        .expect("last initial event id");
    assert_eq!(
        detail
            .pending_approval
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("workflow-continuation")
    );
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
        .expect("resolved workflow continuation approval");

    assert_eq!(resolved.status, "completed");
    assert_eq!(executor.request_count(), 3);
    assert!(resolved.workflow_run.is_some());
    assert!(resolved.background_state.is_some());
    assert_eq!(
        resolved
            .last_mediation_outcome
            .as_ref()
            .map(|outcome| outcome.target_kind.as_str()),
        Some("workflow-continuation")
    );

    let resolved_detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("resolved detail");
    assert!(resolved_detail.workflow.is_some());
    assert!(resolved_detail.background_run.is_some());
    assert!(resolved_detail
        .subruns
        .iter()
        .all(|subrun| subrun.status == "completed"));
    let resolved_events = adapter
        .list_events(&session.summary.id, Some(&replay_after))
        .await
        .expect("resolved workflow events");
    assert!(resolved_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.step.completed")));
    assert!(resolved_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.completed")));
    let background_completed = resolved_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("background.completed"))
        .expect("background completed event");
    assert_eq!(
        background_completed.run_id.as_deref(),
        Some(resolved.id.as_str())
    );
    assert_eq!(
        background_completed.workflow_run_id.as_deref(),
        resolved.workflow_run.as_deref()
    );
    assert_eq!(background_completed.outcome.as_deref(), Some("completed"));
    assert_eq!(
        resolved_events
            .iter()
            .filter(|event| event.kind.as_deref() == Some("subrun.completed"))
            .count(),
        resolved_detail.subruns.len()
    );
    let workflow_terminal = resolved_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("workflow.completed"))
        .expect("workflow completed event");
    assert_eq!(
        workflow_terminal.run_id.as_deref(),
        Some(resolved.id.as_str())
    );
    assert_eq!(
        workflow_terminal.parent_run_id.as_deref(),
        Some(resolved.id.as_str())
    );
    assert_eq!(
        workflow_terminal.workflow_run_id.as_deref(),
        resolved.workflow_run.as_deref()
    );
    assert!(workflow_terminal.workflow_step_id.is_some());
    assert_eq!(workflow_terminal.outcome.as_deref(), Some("completed"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn workflow_continuation_approval_resume_survives_adapter_restart() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-workflow-restart-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Workflow Restart Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the workflow after restart.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for workflow restart approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workflow restart leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-workflow-restart-worker",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Workflow Restart Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["delivery"]).expect("tags"),
                "Do the delegated work after restart.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker for workflow restart approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workflow restart worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                "team-workflow-restart-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Workflow Restart Approval Team",
                Option::<String>::None,
                "Approval aware workflow team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Delegate, pause for workflow approval, then resume after restart.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "auto",
                    "workflowEscalation": "require-approval"
                }))
                .expect("approval preference"),
                canonical_test_agent_ref("agent-workflow-restart-leader"),
                canonical_test_member_refs(&[
                    "agent-workflow-restart-leader",
                    "agent-workflow-restart-worker",
                ]),
                "Team for workflow restart approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workflow restart approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Workflow plan ready for restart.".into()),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Workflow leader subrun resumed after restart.".into(),
            ),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Workflow worker subrun resumed after restart.".into(),
            ),
            runtime::AssistantEvent::MessageStop,
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
                "conv-workflow-restart-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "Workflow Restart Approval",
                "team:team-workflow-restart-approval",
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
            turn_input("Continue the workflow after restart", None),
        )
        .await
        .expect("pending workflow continuation approval");

    assert_eq!(run.status, "waiting_approval");
    assert_eq!(
        run.pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("workflow-continuation")
    );
    assert_eq!(executor.request_count(), 1);

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert!(!detail.subruns.is_empty());
    assert!(detail.pending_mailbox.is_some());
    assert!(detail.workflow.is_some());
    assert!(detail.background_run.is_some());
    let pending_subrun_count = detail.subruns.len();
    let workflow_run_id = run.workflow_run.clone().expect("workflow run id");
    let workflow_detail = detail
        .run
        .workflow_run_detail
        .as_ref()
        .expect("workflow run detail");
    assert_eq!(workflow_detail.steps.len(), pending_subrun_count + 1);
    assert_eq!(
        workflow_detail.current_step_id.as_deref(),
        Some(run.id.as_str())
    );
    assert_eq!(
        workflow_detail
            .blocking
            .as_ref()
            .map(|blocking| blocking.run_id.as_str()),
        Some(run.id.as_str())
    );
    assert_eq!(
        workflow_detail
            .blocking
            .as_ref()
            .map(|blocking| blocking.target_kind.as_str()),
        Some("workflow-continuation")
    );
    assert_eq!(
        detail
            .background_run
            .as_ref()
            .map(|background| background.continuation_state.as_str()),
        Some("paused")
    );
    assert_eq!(
        detail
            .background_run
            .as_ref()
            .and_then(|background| background.blocking.as_ref())
            .map(|blocking| blocking.target_kind.as_str()),
        Some("workflow-continuation")
    );
    let workflow_state_path = infra
        .paths
        .runtime_state_dir
        .join("workflows")
        .join(format!("{workflow_run_id}.json"));
    let workflow_state: serde_json::Value =
        serde_json::from_slice(&fs::read(&workflow_state_path).expect("workflow state bytes"))
            .expect("workflow state json");
    assert_eq!(
        workflow_state
            .pointer("/detail/steps/0/stepId")
            .and_then(serde_json::Value::as_str),
        Some(run.id.as_str())
    );
    assert_eq!(
        workflow_state
            .pointer("/detail/steps/0/status")
            .and_then(serde_json::Value::as_str),
        Some("waiting_approval")
    );
    assert_eq!(
        workflow_state
            .pointer("/detail/blocking/targetKind")
            .and_then(serde_json::Value::as_str),
        Some("workflow-continuation")
    );
    assert_eq!(
        workflow_state
            .pointer("/background/continuationState")
            .and_then(serde_json::Value::as_str),
        Some("paused")
    );
    let background_state_path = infra
        .paths
        .runtime_state_dir
        .join("background")
        .join(format!(
            "{}.json",
            detail.background_run.as_ref().expect("background").run_id
        ));
    let background_state: serde_json::Value =
        serde_json::from_slice(&fs::read(&background_state_path).expect("background state bytes"))
            .expect("background state json");
    assert_eq!(
        background_state
            .pointer("/summary/continuationState")
            .and_then(serde_json::Value::as_str),
        Some("paused")
    );
    assert_eq!(
        background_state
            .pointer("/summary/blocking/targetKind")
            .and_then(serde_json::Value::as_str),
        Some("workflow-continuation")
    );
    let initial_events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("initial background events");
    assert!(initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("background.started")));
    assert!(initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("background.paused")));
    let workflow_started = initial_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("workflow.started"))
        .expect("workflow started event");
    assert_eq!(
        workflow_started.workflow_step_id.as_deref(),
        Some(run.id.as_str())
    );
    let workflow_step_started = initial_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("workflow.step.started"))
        .expect("workflow step started event");
    assert_eq!(
        workflow_step_started.workflow_step_id.as_deref(),
        Some(run.id.as_str())
    );
    let replay_after = initial_events
        .last()
        .map(|event| event.id.clone())
        .expect("initial event id");
    let approval_id = detail
        .pending_approval
        .as_ref()
        .map(|approval| approval.id.clone())
        .expect("approval id");

    let mut persisted_workflow_state = workflow_state.clone();
    persisted_workflow_state["detail"]["steps"][0]["label"] = json!("Persisted leader node");
    fs::write(
        &workflow_state_path,
        serde_json::to_vec_pretty(&persisted_workflow_state)
            .expect("persisted workflow state bytes"),
    )
    .expect("overwrite workflow state");
    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "UPDATE runtime_workflow_projections
             SET detail_json = ?2
             WHERE workflow_run_id = ?1",
            params![workflow_run_id, "{invalid-workflow-detail"],
        )
        .expect("corrupt workflow detail json");

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );
    let reloaded_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("reloaded session detail");

    assert_eq!(reloaded_detail.subruns.len(), pending_subrun_count);
    assert!(reloaded_detail.pending_mailbox.is_some());
    assert!(reloaded_detail.workflow.is_some());
    assert!(reloaded_detail.background_run.is_some());
    assert_eq!(
        reloaded_detail
            .run
            .workflow_run_detail
            .as_ref()
            .map(|workflow| workflow.steps.len()),
        Some(pending_subrun_count + 1)
    );
    assert_eq!(
        reloaded_detail
            .run
            .workflow_run_detail
            .as_ref()
            .and_then(|workflow| workflow.blocking.as_ref())
            .map(|blocking| blocking.target_kind.as_str()),
        Some("workflow-continuation")
    );
    assert_eq!(
        reloaded_detail
            .run
            .workflow_run_detail
            .as_ref()
            .and_then(|workflow| workflow.steps.first())
            .map(|step| step.label.as_str()),
        Some("Persisted leader node")
    );
    assert_eq!(
        reloaded_detail
            .run
            .workflow_run_detail
            .as_ref()
            .and_then(|workflow| workflow.current_step_label.as_deref()),
        Some("Persisted leader node")
    );
    assert_eq!(
        reloaded_detail
            .background_run
            .as_ref()
            .map(|background| background.continuation_state.as_str()),
        Some("paused")
    );
    assert_eq!(
        reloaded_detail
            .pending_approval
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("workflow-continuation")
    );
    let reloaded_events = reloaded
        .list_events(&session.summary.id, None)
        .await
        .expect("reloaded background events");
    assert!(reloaded_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("background.paused")));

    let resolved = reloaded
        .resolve_approval(
            &session.summary.id,
            &approval_id,
            ResolveRuntimeApprovalInput {
                decision: "approve".into(),
            },
        )
        .await
        .expect("resolved workflow continuation approval after restart");

    assert_eq!(resolved.status, "completed");
    assert_eq!(executor.request_count(), 3);
    assert!(resolved.workflow_run.is_some());
    assert!(resolved.background_state.is_some());

    let resolved_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("resolved detail after restart");
    assert!(resolved_detail.workflow.is_some());
    assert!(resolved_detail.background_run.is_some());
    assert_eq!(resolved_detail.subruns.len(), pending_subrun_count);
    assert!(resolved_detail
        .subruns
        .iter()
        .all(|subrun| subrun.status == "completed"));
    assert_eq!(
        resolved_detail
            .run
            .workflow_run_detail
            .as_ref()
            .map(|workflow| workflow.steps.len()),
        Some(pending_subrun_count + 1)
    );
    assert!(resolved_detail
        .run
        .workflow_run_detail
        .as_ref()
        .and_then(|workflow| workflow.blocking.as_ref())
        .is_none());
    assert!(resolved_detail
        .run
        .workflow_run_detail
        .as_ref()
        .is_some_and(|workflow| workflow.steps.iter().all(|step| step.status == "completed")));
    let resolved_workflow_detail = resolved_detail
        .run
        .workflow_run_detail
        .as_ref()
        .expect("resolved workflow detail");
    let resolved_current_step_id = resolved_workflow_detail
        .current_step_id
        .as_deref()
        .expect("resolved workflow current step id");
    assert_ne!(resolved_current_step_id, "workflow-complete");
    assert!(resolved_workflow_detail
        .steps
        .iter()
        .any(|step| step.step_id == resolved_current_step_id));
    assert_eq!(
        resolved_detail
            .background_run
            .as_ref()
            .map(|background| background.continuation_state.as_str()),
        Some("completed")
    );
    let resolved_events = reloaded
        .list_events(&session.summary.id, Some(&replay_after))
        .await
        .expect("resolved background events");
    assert!(resolved_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("background.completed")));
    let workflow_terminal = resolved_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("workflow.completed"))
        .expect("workflow completed event");
    assert_eq!(
        workflow_terminal.workflow_step_id.as_deref(),
        Some(resolved_current_step_id)
    );
    let workflow_step_completed = resolved_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("workflow.step.completed"))
        .expect("workflow step completed event");
    assert_eq!(
        workflow_step_completed.workflow_step_id.as_deref(),
        Some(resolved_current_step_id)
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_worker_subrun_approval_resume_survives_restart_and_respects_scheduler_queue() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "configuredModels": {
                "scheduler-model": {
                    "configuredModelId": "scheduler-model",
                    "name": "Scheduler Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": "env:ANTHROPIC_API_KEY",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
    );
    grant_owner_permissions(&infra, "user-owner");

    let output_path = root.join("team-subrun-approval-output.txt");
    let output_path_string = output_path.display().to_string();

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Team Scheduler Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the queued workers.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for team subrun scheduler tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team scheduler leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                "agent-team-subrun-scheduler-worker-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Approval Worker",
                Option::<String>::None,
                "Writer",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Write the delegated file after approval.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker that blocks on capability approval.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
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
        .expect("upsert approval worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-worker-queued",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Second Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Finish after the approval worker resumes.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Second queued worker for scheduler tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, permission_envelope_json, delegation_policy_json, approval_preference_json, leader_ref, member_refs, worker_concurrency_limit, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "team-subrun-scheduler-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Subrun Scheduler Approval Team",
                Option::<String>::None,
                "Queue aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Run workers through the runtime scheduler.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({
                    "mode": "leader-orchestrated",
                    "allowBackgroundRuns": true,
                    "allowParallelWorkers": true,
                    "maxWorkerCount": 2
                }))
                .expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "agent:agent-team-subrun-scheduler-leader",
                serde_json::to_string(&vec![
                    "agent:agent-team-subrun-scheduler-worker-approval",
                    "agent:agent-team-subrun-scheduler-worker-queued"
                ])
                .expect("member refs"),
                1_i64,
                "Team for queued subrun approval restart tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Delegation plan ready.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 3,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Writing the queued worker file.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-team-subrun-write".into(),
                name: "bash".into(),
                input: serde_json::json!({
                    "command": format!(
                        "printf 'team subrun approval content\\n' > '{}'",
                        output_path_string
                    ),
                    "run_in_background": true
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 5,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Approval worker completed after restart.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 7,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Queued worker completed after slot release.".into(),
            ),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
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
                "conv-team-subrun-scheduler-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "Queued Subrun Approval Session",
                "team:team-subrun-scheduler-approval",
                Some("scheduler-model"),
                octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Queue the workers and pause the first on approval", None),
        )
        .await
        .expect("pending queued subrun approval");

    assert_eq!(pending_run.status, "waiting_approval");

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert_eq!(detail.subruns.len(), 2);
    assert_eq!(
        detail
            .pending_approval
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("capability-call")
    );
    assert!(detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-approval"
            && subrun.status == "waiting_approval"
    }));
    assert!(detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-queued"
            && subrun.status == "queued"
    }));
    assert_eq!(executor.request_count(), 2);
    let approval_id = detail
        .pending_approval
        .as_ref()
        .map(|approval| approval.id.clone())
        .expect("approval id");
    let blocked_subrun_run_id = detail
        .subruns
        .iter()
        .find(|subrun| subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-approval")
        .map(|subrun| subrun.run_id.clone())
        .expect("blocked subrun run id");

    {
        let mut sessions = adapter.state.sessions.lock().expect("sessions mutex");
        let aggregate = sessions
            .get_mut(&session.summary.id)
            .expect("runtime aggregate");
        aggregate.detail.messages.push(RuntimeMessage {
            id: "msg-parent-drift".into(),
            session_id: session.summary.id.clone(),
            conversation_id: session.summary.conversation_id.clone(),
            sender_type: "user".into(),
            sender_label: "User".into(),
            content: "MUTATED parent prompt drift".into(),
            timestamp: timestamp_now(),
            configured_model_id: aggregate.detail.run.configured_model_id.clone(),
            configured_model_name: aggregate.detail.run.configured_model_name.clone(),
            model_id: aggregate.detail.run.model_id.clone(),
            status: aggregate.detail.run.status.clone(),
            requested_actor_kind: aggregate.detail.run.requested_actor_kind.clone(),
            requested_actor_id: aggregate.detail.run.requested_actor_id.clone(),
            resolved_actor_kind: aggregate.detail.run.resolved_actor_kind.clone(),
            resolved_actor_id: aggregate.detail.run.resolved_actor_id.clone(),
            resolved_actor_label: aggregate.detail.run.resolved_actor_label.clone(),
            used_default_actor: Some(false),
            resource_ids: Some(Vec::new()),
            attachments: Some(Vec::new()),
            artifacts: Some(Vec::new()),
            deliverable_refs: None,
            usage: None,
            tool_calls: None,
            process_entries: None,
        });
        aggregate.detail.summary.last_message_preview = Some("MUTATED parent prompt drift".into());

        let session_policy = adapter
            .load_session_policy_snapshot(&aggregate.metadata.session_policy_snapshot_ref)
            .expect("session policy");
        let actor_manifest = adapter
            .load_actor_manifest_snapshot(&session_policy.manifest_snapshot_ref)
            .expect("actor manifest");
        let actor_manifest::CompiledActorManifest::Team(team_manifest) = actor_manifest else {
            panic!("expected team manifest");
        };
        team_runtime::ensure_subrun_state_metadata_for_session(
            &adapter,
            aggregate,
            &team_manifest,
            &session_policy,
            timestamp_now(),
        )
        .expect("refresh subrun metadata");
        assert_eq!(
            aggregate
                .metadata
                .subrun_states
                .get(&blocked_subrun_run_id)
                .expect("blocked subrun state")
                .dispatch
                .worker_input
                .content,
            "Queue the workers and pause the first on approval"
        );
        adapter
            .persist_runtime_projections(aggregate)
            .expect("persist mutated aggregate");
    }

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );
    let reloaded_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("reloaded detail");

    assert_eq!(reloaded_detail.subruns.len(), 2);
    assert_eq!(
        reloaded_detail
            .pending_approval
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("capability-call")
    );
    assert!(reloaded_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-approval"
            && subrun.status == "waiting_approval"
    }));
    assert!(reloaded_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-queued"
            && subrun.status == "queued"
    }));

    let resolved = reloaded
        .resolve_approval(
            &session.summary.id,
            &approval_id,
            ResolveRuntimeApprovalInput {
                decision: "approve".into(),
            },
        )
        .await
        .expect("resolved queued subrun approval after restart");

    assert_eq!(resolved.status, "completed");
    assert_eq!(executor.request_count(), 4);
    let requests = executor.requests();
    assert_eq!(
        last_user_text(&requests[2]),
        Some("Queue the workers and pause the first on approval")
    );
    for _ in 0..20 {
        if fs::read_to_string(&output_path)
            .map(|content| content == "team subrun approval content\n")
            .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        fs::read_to_string(&output_path).expect("written output"),
        "team subrun approval content\n"
    );

    let resolved_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("resolved detail");
    assert!(resolved_detail.pending_approval.is_none());
    assert!(resolved_detail
        .subruns
        .iter()
        .all(|subrun| subrun.status == "completed"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_subrun_metadata_refresh_rehydrates_from_manifest_plan_without_detail_subruns() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-orchestrator",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Orchestrator Agent",
                Option::<String>::None,
                "Systems thinker",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Coordinate the team response.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leads team execution.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert orchestrator agent");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-project-delivery",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Project Delivery Agent",
                Option::<String>::None,
                "Structured and pragmatic",
                serde_json::to_string(&vec!["delivery"]).expect("tags"),
                "Keep project execution on track.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Supports cross-functional delivery.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert delivery agent");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                "team-workspace-core",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Workspace Core",
                Option::<String>::None,
                "Cross-functional design review board",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Debate options, then return a single aligned answer.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                canonical_test_agent_ref("agent-orchestrator"),
                canonical_test_member_refs(&["agent-orchestrator", "agent-project-delivery"]),
                "Core workspace decision board.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workspace core team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Leader finished.".into()),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Worker one finished.".into()),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Worker two finished.".into()),
            runtime::AssistantEvent::MessageStop,
        ],
    ]));
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
                "conv-team-refresh-state-first",
                octopus_core::DEFAULT_PROJECT_ID,
                "Team Refresh State First",
                "team:team-workspace-core",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(&session.summary.id, turn_input("Review the proposal", None))
        .await
        .expect("team run");
    assert!(!run.status.is_empty());

    let original_detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("original detail");
    let original_subrun_ids = original_detail
        .subruns
        .iter()
        .map(|subrun| subrun.run_id.clone())
        .collect::<Vec<_>>();
    assert!(!original_subrun_ids.is_empty());

    {
        let mut sessions = adapter.state.sessions.lock().expect("sessions mutex");
        let aggregate = sessions
            .get_mut(&session.summary.id)
            .expect("runtime aggregate");
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

        let session_policy = adapter
            .load_session_policy_snapshot(&aggregate.metadata.session_policy_snapshot_ref)
            .expect("session policy");
        let actor_manifest = adapter
            .load_actor_manifest_snapshot(&session_policy.manifest_snapshot_ref)
            .expect("actor manifest");
        let actor_manifest::CompiledActorManifest::Team(team_manifest) = actor_manifest else {
            panic!("expected team manifest");
        };

        team_runtime::ensure_subrun_state_metadata_for_session(
            &adapter,
            aggregate,
            &team_manifest,
            &session_policy,
            timestamp_now(),
        )
        .expect("refresh subrun metadata from manifest plan");

        let rebuilt_subrun_ids = aggregate
            .detail
            .subruns
            .iter()
            .map(|subrun| subrun.run_id.clone())
            .collect::<Vec<_>>();
        assert_eq!(rebuilt_subrun_ids, original_subrun_ids);
        assert_eq!(
            aggregate.metadata.subrun_states.len(),
            original_subrun_ids.len()
        );
        assert!(aggregate.detail.pending_mailbox.is_some());
        assert!(aggregate.detail.workflow.is_some());
        assert!(aggregate.detail.background_run.is_some());
        assert!(aggregate.detail.run.worker_dispatch.is_some());
    }

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_worker_subrun_explicit_cancel_releases_scheduler_queue() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "configuredModels": {
                "scheduler-model": {
                    "configuredModelId": "scheduler-model",
                    "name": "Scheduler Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": "env:ANTHROPIC_API_KEY",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
    );
    grant_owner_permissions(&infra, "user-owner");

    let output_path = root.join("team-subrun-explicit-cancel-output.txt");
    let output_path_string = output_path.display().to_string();

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Team Scheduler Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the queued workers.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for team subrun cancellation tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team scheduler leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                "agent-team-subrun-scheduler-worker-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Approval Worker",
                Option::<String>::None,
                "Writer",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Write the delegated file after approval.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker that blocks on capability approval.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
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
        .expect("upsert approval worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-worker-queued",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Second Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Finish after the first worker is cancelled.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Second queued worker for cancellation tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, permission_envelope_json, delegation_policy_json, approval_preference_json, leader_ref, member_refs, worker_concurrency_limit, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "team-subrun-scheduler-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Subrun Scheduler Approval Team",
                Option::<String>::None,
                "Queue aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Run workers through the runtime scheduler.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({
                    "mode": "leader-orchestrated",
                    "allowBackgroundRuns": true,
                    "allowParallelWorkers": true,
                    "maxWorkerCount": 2
                }))
                .expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "agent:agent-team-subrun-scheduler-leader",
                serde_json::to_string(&vec![
                    "agent:agent-team-subrun-scheduler-worker-approval",
                    "agent:agent-team-subrun-scheduler-worker-queued"
                ])
                .expect("member refs"),
                1_i64,
                "Team for explicit subrun cancellation tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Delegation plan ready.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 3,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Writing the queued worker file.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-team-subrun-explicit-cancel-write".into(),
                name: "bash".into(),
                input: serde_json::json!({
                    "command": format!(
                        "printf 'team subrun explicit cancel content\\n' > '{}'",
                        output_path_string
                    ),
                    "run_in_background": true
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 5,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Queued worker completed after explicit subrun cancellation released the slot."
                    .into(),
            ),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
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
                "conv-team-subrun-explicit-cancel",
                octopus_core::DEFAULT_PROJECT_ID,
                "Queued Subrun Explicit Cancel Session",
                "team:team-subrun-scheduler-approval",
                Some("scheduler-model"),
                octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input(
                "Queue the workers and cancel the first blocked subrun",
                None,
            ),
        )
        .await
        .expect("pending queued subrun approval");

    assert_eq!(pending_run.status, "waiting_approval");

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert_eq!(detail.subruns.len(), 2);
    let blocked_subrun = detail
        .subruns
        .iter()
        .find(|subrun| subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-approval")
        .expect("blocked subrun")
        .clone();
    assert_eq!(blocked_subrun.status, "waiting_approval");
    assert!(detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-queued"
            && subrun.status == "queued"
    }));
    assert_eq!(executor.request_count(), 2);
    let replay_after = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("initial events")
        .last()
        .map(|event| event.id.clone())
        .expect("initial event id");

    let cancelled = adapter
        .cancel_subrun(
            &session.summary.id,
            &blocked_subrun.run_id,
            CancelRuntimeSubrunInput {
                note: Some("skip the first worker".into()),
            },
        )
        .await
        .expect("cancel subrun");

    assert_eq!(cancelled.status, "failed");
    assert_eq!(executor.request_count(), 3);
    assert!(!output_path.exists());

    let cancelled_detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("cancelled detail");
    assert!(cancelled_detail.pending_approval.is_none());
    assert!(cancelled_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-approval"
            && subrun.status == "cancelled"
    }));
    assert!(cancelled_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-queued"
            && subrun.status == "completed"
    }));

    let cancelled_events = adapter
        .list_events(&session.summary.id, Some(&replay_after))
        .await
        .expect("cancelled events");
    let cancelled_subrun_event = cancelled_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("subrun.cancelled"))
        .expect("subrun cancelled event");
    assert_eq!(
        cancelled_subrun_event.actor_ref.as_deref(),
        Some("agent:agent-team-subrun-scheduler-worker-approval")
    );
    assert_eq!(cancelled_subrun_event.outcome.as_deref(), Some("cancelled"));
    assert!(cancelled_events.iter().any(|event| {
        event.kind.as_deref() == Some("subrun.completed")
            && event.actor_ref.as_deref() == Some("agent:agent-team-subrun-scheduler-worker-queued")
    }));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_worker_subrun_auth_resume_survives_restart_and_respects_scheduler_queue() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "configuredModels": {
                "scheduler-model": {
                    "configuredModelId": "scheduler-model",
                    "name": "Scheduler Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": "env:ANTHROPIC_API_KEY",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
    );
    grant_owner_permissions(&infra, "user-owner");

    let output_path = root.join("team-subrun-auth-output.txt");
    let output_path_string = output_path.display().to_string();

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-auth-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Team Scheduler Auth Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the queued workers.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for team subrun auth scheduler tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team scheduler auth leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                "agent-team-subrun-scheduler-worker-auth",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Auth Worker",
                Option::<String>::None,
                "Writer",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Write the delegated file after mediation resumes.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker that will be rewritten into an auth-blocked subrun.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
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
        .expect("upsert auth worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-worker-auth-queued",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Auth Second Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Finish after the auth-blocked worker resumes.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Second queued worker for auth scheduler tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued auth worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, permission_envelope_json, delegation_policy_json, approval_preference_json, leader_ref, member_refs, worker_concurrency_limit, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "team-subrun-scheduler-auth",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Subrun Scheduler Auth Team",
                Option::<String>::None,
                "Queue aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Run workers through the runtime scheduler.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({
                    "mode": "leader-orchestrated",
                    "allowBackgroundRuns": true,
                    "allowParallelWorkers": true,
                    "maxWorkerCount": 2
                }))
                .expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "agent:agent-team-subrun-scheduler-auth-leader",
                serde_json::to_string(&vec![
                    "agent:agent-team-subrun-scheduler-worker-auth",
                    "agent:agent-team-subrun-scheduler-worker-auth-queued"
                ])
                .expect("member refs"),
                1_i64,
                "Team for queued subrun auth restart tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued auth team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Delegation plan ready.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 3,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Writing the queued worker file.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-team-subrun-auth-write".into(),
                name: "bash".into(),
                input: serde_json::json!({
                    "command": format!(
                        "printf 'team subrun auth content\\n' > '{}'",
                        output_path_string
                    ),
                    "run_in_background": true
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 5,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Auth worker completed after restart.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 7,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Queued worker completed after auth slot release.".into(),
            ),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
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
                "conv-team-subrun-scheduler-auth",
                octopus_core::DEFAULT_PROJECT_ID,
                "Queued Subrun Auth Session",
                "team:team-subrun-scheduler-auth",
                Some("scheduler-model"),
                octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Queue the workers and pause the first on approval", None),
        )
        .await
        .expect("pending queued subrun approval");

    assert_eq!(pending_run.status, "waiting_approval");

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert_eq!(detail.subruns.len(), 2);
    let blocked_subrun = detail
        .subruns
        .iter()
        .find(|subrun| subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth")
        .expect("blocked subrun")
        .clone();
    assert_eq!(blocked_subrun.status, "waiting_approval");
    assert!(detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth-queued"
            && subrun.status == "queued"
    }));
    assert_eq!(executor.request_count(), 2);

    let blocked_state_path = infra
        .paths
        .runtime_state_dir
        .join("subruns")
        .join(format!("{}.json", blocked_subrun.run_id));
    let mut blocked_state: serde_json::Value =
        serde_json::from_slice(&fs::read(&blocked_state_path).expect("subrun state bytes"))
            .expect("subrun state json");
    let capability_state_ref = blocked_state["run"]["capabilityStateRef"]
        .as_str()
        .expect("capability state ref")
        .to_string();
    let capability_store = adapter
        .load_capability_store(Some(&capability_state_ref))
        .expect("capability store");
    capability_store.approve_tool("bash");
    adapter
        .persist_capability_store(&capability_state_ref, &capability_store)
        .expect("persist capability store");

    let auth_challenge_id = format!("auth-{}", blocked_subrun.run_id);
    let auth_challenge = json!({
        "approvalLayer": "provider",
        "capabilityId": blocked_state["run"]["checkpoint"]["capabilityId"],
        "checkpointRef": blocked_state["run"]["checkpoint"]["checkpointArtifactRef"],
        "conversationId": session.summary.conversation_id,
        "createdAt": blocked_subrun.updated_at + 9,
        "detail": "Worker requires provider authentication before replay.",
        "escalationReason": "provider authentication is required for the delegated tool",
        "id": auth_challenge_id,
        "dispatchKind": "capability-call",
        "providerKey": "delegated-provider",
        "concurrencyPolicy": "serialized",
        "input": {
            "command": format!(
                "printf 'team subrun auth content\\n' > '{}'",
                output_path_string
            ),
            "run_in_background": true
        },
        "requiredPermission": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
        "requiresApproval": false,
        "requiresAuth": true,
        "runId": blocked_subrun.run_id,
        "sessionId": session.summary.id,
        "status": "pending",
        "summary": "Complete provider authentication for the delegated worker.",
        "targetKind": "capability-call",
        "targetRef": "bash",
        "toolName": "bash"
    });
    let pending_mediation = json!({
        "approvalLayer": "provider",
        "authChallengeId": auth_challenge_id,
        "capabilityId": blocked_state["run"]["checkpoint"]["capabilityId"],
        "checkpointRef": blocked_state["run"]["checkpoint"]["checkpointArtifactRef"],
        "detail": "Worker requires provider authentication before replay.",
        "escalationReason": "provider authentication is required for the delegated tool",
        "mediationId": format!("mediation-{}", blocked_subrun.run_id),
        "mediationKind": "auth",
        "dispatchKind": "capability-call",
        "providerKey": "delegated-provider",
        "concurrencyPolicy": "serialized",
        "input": {
            "command": format!(
                "printf 'team subrun auth content\\n' > '{}'",
                output_path_string
            ),
            "run_in_background": true
        },
        "reason": "provider authentication required",
        "requiredPermission": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
        "requiresApproval": false,
        "requiresAuth": true,
        "state": "pending",
        "summary": "Complete provider authentication for the delegated worker.",
        "targetKind": "capability-call",
        "targetRef": "bash",
        "toolName": "bash"
    });

    blocked_state["run"]["status"] = json!("auth-required");
    blocked_state["run"]["currentStep"] = json!("awaiting_auth");
    blocked_state["run"]["updatedAt"] = json!(blocked_subrun.updated_at + 11);
    blocked_state["run"]["nextAction"] = json!("auth");
    blocked_state["run"]["approvalState"] = json!("auth-required");
    blocked_state["run"]["approvalTarget"] = serde_json::Value::Null;
    blocked_state["run"]["authTarget"] = auth_challenge.clone();
    blocked_state["run"]["pendingMediation"] = pending_mediation.clone();
    blocked_state["dispatch"]["workerInput"]["content"] = json!("");
    blocked_state["run"]["checkpoint"]["pendingApproval"] = serde_json::Value::Null;
    blocked_state["run"]["checkpoint"]["pendingAuthChallenge"] = auth_challenge;
    blocked_state["run"]["checkpoint"]["pendingMediation"] = pending_mediation;
    fs::write(
        &blocked_state_path,
        serde_json::to_vec_pretty(&blocked_state).expect("mutated subrun state bytes"),
    )
    .expect("overwrite subrun state");

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );
    let reloaded_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("reloaded detail");

    assert_eq!(reloaded_detail.subruns.len(), 2);
    assert!(reloaded_detail.pending_approval.is_none());
    assert_eq!(
        reloaded_detail
            .run
            .auth_target
            .as_ref()
            .map(|challenge| challenge.id.as_str()),
        Some(auth_challenge_id.as_str())
    );
    assert!(reloaded_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth"
            && subrun.status == "auth-required"
    }));
    assert!(reloaded_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth-queued"
            && subrun.status == "queued"
    }));
    let initial_events = reloaded
        .list_events(&session.summary.id, None)
        .await
        .expect("initial auth workflow events");
    assert!(initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("background.started")));
    let background_paused = initial_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("background.paused"))
        .expect("background paused event");
    assert_eq!(
        background_paused.workflow_run_id.as_deref(),
        reloaded_detail.run.workflow_run.as_deref()
    );
    assert_eq!(background_paused.outcome.as_deref(), Some("paused"));
    let replay_after = initial_events
        .last()
        .map(|event| event.id.clone())
        .expect("initial auth event id");

    let resolved = reloaded
        .resolve_auth_challenge(
            &session.summary.id,
            &auth_challenge_id,
            ResolveRuntimeAuthChallengeInput {
                resolution: "resolved".into(),
                note: Some("provider linked".into()),
            },
        )
        .await
        .expect("resolved queued subrun auth after restart");

    assert_eq!(resolved.status, "completed");
    assert_eq!(executor.request_count(), 4);
    for _ in 0..20 {
        if fs::read_to_string(&output_path)
            .map(|content| content == "team subrun auth content\n")
            .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        fs::read_to_string(&output_path).expect("written output"),
        "team subrun auth content\n"
    );

    let resolved_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("resolved detail");
    assert!(resolved_detail.run.auth_target.is_none());
    assert!(resolved_detail.pending_approval.is_none());
    assert!(resolved_detail
        .subruns
        .iter()
        .all(|subrun| subrun.status == "completed"));
    let resolved_events = reloaded
        .list_events(&session.summary.id, Some(&replay_after))
        .await
        .expect("resolved auth workflow events");
    assert!(resolved_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.step.completed")));
    assert!(resolved_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.completed")));
    let background_completed = resolved_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("background.completed"))
        .expect("background completed event");
    assert_eq!(
        background_completed.workflow_run_id.as_deref(),
        resolved.workflow_run.as_deref()
    );
    assert_eq!(background_completed.outcome.as_deref(), Some("completed"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_worker_subrun_auth_cancellation_releases_scheduler_queue_and_emits_cancelled_state() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "configuredModels": {
                "scheduler-model": {
                    "configuredModelId": "scheduler-model",
                    "name": "Scheduler Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": "env:ANTHROPIC_API_KEY",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
    );
    grant_owner_permissions(&infra, "user-owner");

    let output_path = root.join("team-subrun-auth-cancel-output.txt");
    let output_path_string = output_path.display().to_string();

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-auth-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Team Scheduler Auth Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the queued workers.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for team subrun auth scheduler cancellation tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team scheduler auth leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                "agent-team-subrun-scheduler-worker-auth",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Auth Worker",
                Option::<String>::None,
                "Writer",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Write the delegated file after mediation resumes.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker that will be rewritten into an auth-blocked subrun.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
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
        .expect("upsert auth worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-worker-auth-queued",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Auth Second Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Finish after the auth-blocked worker resumes or is cancelled.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Second queued worker for auth scheduler cancellation tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued auth worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, permission_envelope_json, delegation_policy_json, approval_preference_json, leader_ref, member_refs, worker_concurrency_limit, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "team-subrun-scheduler-auth",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Subrun Scheduler Auth Team",
                Option::<String>::None,
                "Queue aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Run workers through the runtime scheduler.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({
                    "mode": "leader-orchestrated",
                    "allowBackgroundRuns": true,
                    "allowParallelWorkers": true,
                    "maxWorkerCount": 2
                }))
                .expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "agent:agent-team-subrun-scheduler-auth-leader",
                serde_json::to_string(&vec![
                    "agent:agent-team-subrun-scheduler-worker-auth",
                    "agent:agent-team-subrun-scheduler-worker-auth-queued"
                ])
                .expect("member refs"),
                1_i64,
                "Team for queued subrun auth cancellation tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued auth team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Delegation plan ready.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 3,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Writing the queued worker file.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-team-subrun-auth-cancel-write".into(),
                name: "bash".into(),
                input: serde_json::json!({
                    "command": format!(
                        "printf 'team subrun auth cancel content\\n' > '{}'",
                        output_path_string
                    ),
                    "run_in_background": true
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 5,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Queued worker completed after auth cancellation released the slot.".into(),
            ),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
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
                "conv-team-subrun-scheduler-auth-cancel",
                octopus_core::DEFAULT_PROJECT_ID,
                "Queued Subrun Auth Cancellation Session",
                "team:team-subrun-scheduler-auth",
                Some("scheduler-model"),
                octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Queue the workers and cancel the blocked auth worker", None),
        )
        .await
        .expect("pending queued subrun auth cancellation");

    assert_eq!(pending_run.status, "waiting_approval");

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert_eq!(detail.subruns.len(), 2);
    let blocked_subrun = detail
        .subruns
        .iter()
        .find(|subrun| subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth")
        .expect("blocked subrun")
        .clone();
    assert_eq!(blocked_subrun.status, "waiting_approval");
    assert!(detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth-queued"
            && subrun.status == "queued"
    }));
    assert_eq!(executor.request_count(), 2);

    let blocked_state_path = infra
        .paths
        .runtime_state_dir
        .join("subruns")
        .join(format!("{}.json", blocked_subrun.run_id));
    let mut blocked_state: serde_json::Value =
        serde_json::from_slice(&fs::read(&blocked_state_path).expect("subrun state bytes"))
            .expect("subrun state json");
    let capability_state_ref = blocked_state["run"]["capabilityStateRef"]
        .as_str()
        .expect("capability state ref")
        .to_string();
    let capability_store = adapter
        .load_capability_store(Some(&capability_state_ref))
        .expect("capability store");
    capability_store.approve_tool("bash");
    adapter
        .persist_capability_store(&capability_state_ref, &capability_store)
        .expect("persist capability store");

    let auth_challenge_id = format!("auth-{}", blocked_subrun.run_id);
    let auth_challenge = json!({
        "approvalLayer": "provider",
        "capabilityId": blocked_state["run"]["checkpoint"]["capabilityId"],
        "checkpointRef": blocked_state["run"]["checkpoint"]["checkpointArtifactRef"],
        "conversationId": session.summary.conversation_id,
        "createdAt": blocked_subrun.updated_at + 9,
        "detail": "Worker requires provider authentication before replay.",
        "escalationReason": "provider authentication is required for the delegated tool",
        "id": auth_challenge_id,
        "dispatchKind": "capability-call",
        "providerKey": "delegated-provider",
        "concurrencyPolicy": "serialized",
        "input": {
            "command": format!(
                "printf 'team subrun auth cancel content\\n' > '{}'",
                output_path_string
            ),
            "run_in_background": true
        },
        "requiredPermission": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
        "requiresApproval": false,
        "requiresAuth": true,
        "runId": blocked_subrun.run_id,
        "sessionId": session.summary.id,
        "status": "pending",
        "summary": "Complete provider authentication for the delegated worker.",
        "targetKind": "capability-call",
        "targetRef": "bash",
        "toolName": "bash"
    });
    let pending_mediation = json!({
        "approvalLayer": "provider",
        "authChallengeId": auth_challenge_id,
        "capabilityId": blocked_state["run"]["checkpoint"]["capabilityId"],
        "checkpointRef": blocked_state["run"]["checkpoint"]["checkpointArtifactRef"],
        "detail": "Worker requires provider authentication before replay.",
        "escalationReason": "provider authentication is required for the delegated tool",
        "mediationId": format!("mediation-{}", blocked_subrun.run_id),
        "mediationKind": "auth",
        "dispatchKind": "capability-call",
        "providerKey": "delegated-provider",
        "concurrencyPolicy": "serialized",
        "input": {
            "command": format!(
                "printf 'team subrun auth cancel content\\n' > '{}'",
                output_path_string
            ),
            "run_in_background": true
        },
        "reason": "provider authentication required",
        "requiredPermission": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
        "requiresApproval": false,
        "requiresAuth": true,
        "state": "pending",
        "summary": "Complete provider authentication for the delegated worker.",
        "targetKind": "capability-call",
        "targetRef": "bash",
        "toolName": "bash"
    });

    blocked_state["run"]["status"] = json!("auth-required");
    blocked_state["run"]["currentStep"] = json!("awaiting_auth");
    blocked_state["run"]["updatedAt"] = json!(blocked_subrun.updated_at + 11);
    blocked_state["run"]["nextAction"] = json!("auth");
    blocked_state["run"]["approvalState"] = json!("auth-required");
    blocked_state["run"]["approvalTarget"] = serde_json::Value::Null;
    blocked_state["run"]["authTarget"] = auth_challenge.clone();
    blocked_state["run"]["pendingMediation"] = pending_mediation.clone();
    blocked_state["dispatch"]["workerInput"]["content"] = json!("");
    blocked_state["run"]["checkpoint"]["pendingApproval"] = serde_json::Value::Null;
    blocked_state["run"]["checkpoint"]["pendingAuthChallenge"] = auth_challenge;
    blocked_state["run"]["checkpoint"]["pendingMediation"] = pending_mediation;
    fs::write(
        &blocked_state_path,
        serde_json::to_vec_pretty(&blocked_state).expect("mutated subrun state bytes"),
    )
    .expect("overwrite subrun state");

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );
    let reloaded_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("reloaded detail");

    assert_eq!(reloaded_detail.subruns.len(), 2);
    assert!(reloaded_detail.pending_approval.is_none());
    assert_eq!(
        reloaded_detail
            .run
            .auth_target
            .as_ref()
            .map(|challenge| challenge.id.as_str()),
        Some(auth_challenge_id.as_str())
    );
    assert!(reloaded_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth"
            && subrun.status == "auth-required"
    }));
    assert!(reloaded_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth-queued"
            && subrun.status == "queued"
    }));
    let initial_events = reloaded
        .list_events(&session.summary.id, None)
        .await
        .expect("initial auth workflow events");
    let replay_after = initial_events
        .last()
        .map(|event| event.id.clone())
        .expect("initial auth event id");

    let cancelled = reloaded
        .resolve_auth_challenge(
            &session.summary.id,
            &auth_challenge_id,
            ResolveRuntimeAuthChallengeInput {
                resolution: "cancelled".into(),
                note: Some("provider login abandoned".into()),
            },
        )
        .await
        .expect("cancelled queued subrun auth after restart");

    assert_eq!(cancelled.status, "failed");
    assert_eq!(executor.request_count(), 3);
    assert!(!output_path.exists());

    let cancelled_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("cancelled detail");
    assert!(cancelled_detail.run.auth_target.is_none());
    assert!(cancelled_detail.pending_approval.is_none());
    assert!(cancelled_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth"
            && subrun.status == "cancelled"
    }));
    assert!(cancelled_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth-queued"
            && subrun.status == "completed"
    }));

    let cancelled_events = reloaded
        .list_events(&session.summary.id, Some(&replay_after))
        .await
        .expect("cancelled auth workflow events");
    let cancelled_subrun_event = cancelled_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("subrun.cancelled"))
        .expect("subrun cancelled event");
    assert_eq!(
        cancelled_subrun_event.actor_ref.as_deref(),
        Some("agent:agent-team-subrun-scheduler-worker-auth")
    );
    assert_eq!(cancelled_subrun_event.outcome.as_deref(), Some("cancelled"));
    assert!(cancelled_events.iter().any(|event| {
        event.kind.as_deref() == Some("subrun.completed")
            && event.actor_ref.as_deref()
                == Some("agent:agent-team-subrun-scheduler-worker-auth-queued")
    }));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_spawn_approval_chains_into_workflow_continuation_approval_when_required() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-spawn-workflow-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Spawn Workflow Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the team.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for chained workflow approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert chained leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-spawn-workflow-worker",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Spawn Workflow Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["delivery"]).expect("tags"),
                "Do the delegated work.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker for chained workflow approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert chained worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                "team-spawn-workflow-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Spawn Workflow Approval Team",
                Option::<String>::None,
                "Approval aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Delegate after approval, then continue workflow after approval.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "require-approval",
                    "workflowEscalation": "require-approval"
                }))
                .expect("approval preference"),
                canonical_test_agent_ref("agent-team-spawn-workflow-leader"),
                canonical_test_member_refs(&[
                    "agent-team-spawn-workflow-leader",
                    "agent-team-spawn-workflow-worker",
                ]),
                "Team for chained workflow approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert chained approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![vec![
        runtime::AssistantEvent::TextDelta("Delegation plan ready.".into()),
        runtime::AssistantEvent::MessageStop,
    ]]));
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
                "conv-team-spawn-workflow-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "Chained Team Approval",
                "team:team-spawn-workflow-approval",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(&session.summary.id, turn_input("Delegate the task", None))
        .await
        .expect("pending team spawn approval");

    assert_eq!(run.status, "waiting_approval");
    assert_eq!(
        run.pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("team-spawn")
    );
    let approval_id = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail")
        .pending_approval
        .as_ref()
        .map(|approval| approval.id.clone())
        .expect("approval id");

    let spawn_resolved = adapter
        .resolve_approval(
            &session.summary.id,
            &approval_id,
            ResolveRuntimeApprovalInput {
                decision: "approve".into(),
            },
        )
        .await
        .expect("resolved team spawn approval");

    assert_eq!(spawn_resolved.status, "waiting_approval");
    assert_eq!(spawn_resolved.current_step, "awaiting_approval");
    assert_eq!(
        spawn_resolved
            .pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("workflow-continuation")
    );
    assert!(spawn_resolved.worker_dispatch.is_some());
    assert!(spawn_resolved.workflow_run.is_some());
    assert!(spawn_resolved.background_state.is_some());
    assert_eq!(executor.request_count(), 1);

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail after spawn approval");
    assert!(!detail.subruns.is_empty());
    assert!(detail.workflow.is_some());
    assert!(detail.background_run.is_some());
    assert_eq!(
        detail
            .pending_approval
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("workflow-continuation")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_spawn_policy_deny_suppresses_subrun_projection_on_main_runtime_path() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-policy-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Policy Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the policy-constrained team.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for team spawn deny policy tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team policy leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-policy-worker",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Policy Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["delivery"]).expect("tags"),
                "Do the delegated work.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker for team spawn deny policy tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team policy worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, delegation_policy_json, approval_preference_json, leader_ref, member_refs, worker_concurrency_limit, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![
                "team-spawn-deny",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Spawn Deny Team",
                Option::<String>::None,
                "Policy constrained team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Do not delegate when policy forbids it.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "mode": "disabled",
                    "allowBackgroundRuns": false,
                    "allowParallelWorkers": false,
                    "maxWorkerCount": 0
                }))
                .expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "auto",
                    "workflowEscalation": "require-approval"
                }))
                .expect("approval preference"),
                canonical_test_agent_ref("agent-team-policy-leader"),
                canonical_test_member_refs(&[
                    "agent-team-policy-leader",
                    "agent-team-policy-worker",
                ]),
                2_i64,
                "Team for team spawn deny policy tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team spawn deny team");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-team-spawn-deny",
                octopus_core::DEFAULT_PROJECT_ID,
                "Team Spawn Deny",
                "team:team-spawn-deny",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(&session.summary.id, turn_input("Delegate the task", None))
        .await
        .expect("team run");

    assert_eq!(run.status, "completed");
    assert_eq!(
        run.worker_dispatch
            .as_ref()
            .map(|dispatch| dispatch.total_subruns),
        Some(0)
    );

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert_eq!(detail.subrun_count, 0);
    assert!(detail.subruns.is_empty());
    assert!(detail.handoffs.is_empty());

    let subrun_state_dir = infra.paths.runtime_state_dir.join("subruns");
    let subrun_state_count = if subrun_state_dir.exists() {
        fs::read_dir(&subrun_state_dir)
            .expect("subrun state dir")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(run.id.as_str())
            })
            .count()
    } else {
        0
    };
    assert_eq!(subrun_state_count, 0);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn runtime_session_public_contract_and_projection_fields_match_phase_two_shape() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(16),
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-phase-two-shape",
                octopus_core::DEFAULT_PROJECT_ID,
                "Phase 2 Contract Shape",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    assert_eq!(session.selected_actor_ref, agent_actor_ref);
    assert_eq!(
        session.manifest_revision,
        octopus_core::ASSET_MANIFEST_REVISION_V2
    );
    assert_eq!(session.active_run_id, session.run.id);
    assert_eq!(session.subrun_count, 0);
    assert_eq!(
        session.session_policy.selected_actor_ref,
        session.selected_actor_ref
    );
    assert_eq!(
        session.session_policy.selected_configured_model_id,
        "quota-model"
    );
    assert_eq!(
        session.session_policy.execution_permission_mode,
        octopus_core::RUNTIME_PERMISSION_READ_ONLY
    );
    assert_eq!(
        session.session_policy.manifest_revision,
        octopus_core::ASSET_MANIFEST_REVISION_V2
    );
    assert!(session.memory_summary.summary.contains("durable"));
    assert_eq!(
        session.capability_summary.visible_tools,
        Vec::<String>::new()
    );
    assert_eq!(session.run.trace_context.session_id, session.summary.id);
    assert!(!session.run.trace_context.trace_id.is_empty());
    assert!(!session.run.trace_context.turn_id.is_empty());
    assert_eq!(session.run.checkpoint.current_iteration_index, 0);

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Finish the phase two contract", None),
        )
        .await
        .expect("run");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let session_projection: (String, String, String, i64, String, String) = connection
        .query_row(
            "SELECT selected_actor_ref, manifest_revision, active_run_id, subrun_count, manifest_snapshot_ref, session_policy_snapshot_ref
             FROM runtime_session_projections
             WHERE id = ?1",
            [&session.summary.id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("session projection");
    assert_eq!(session_projection.0, session.selected_actor_ref);
    assert_eq!(
        session_projection.1,
        octopus_core::ASSET_MANIFEST_REVISION_V2
    );
    assert_eq!(session_projection.2, run.id);
    assert_eq!(session_projection.3, 0);
    assert!(!session_projection.4.is_empty());
    assert!(!session_projection.5.is_empty());

    let run_projection: (String, Option<String>, String, Option<String>, String, String, String) =
        connection
            .query_row(
                "SELECT run_kind, parent_run_id, actor_ref, delegated_by_tool_call_id, approval_state, trace_id, turn_id
                 FROM runtime_run_projections
                 WHERE id = ?1",
                [&run.id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .expect("run projection");
    assert_eq!(run_projection.0, "primary");
    assert_eq!(run_projection.1, None);
    assert_eq!(run_projection.2, session.selected_actor_ref);
    assert_eq!(run_projection.3, None);
    assert_eq!(run_projection.4, "not-required");
    assert!(!run_projection.5.is_empty());
    assert!(!run_projection.6.is_empty());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn runtime_events_only_emit_declared_runtime_event_kinds() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(8),
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-phase-two-events",
                octopus_core::DEFAULT_PROJECT_ID,
                "Phase 2 Events",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");
    adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Emit valid phase two events", None),
        )
        .await
        .expect("run");

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    let allowed = [
        "planner.started",
        "planner.completed",
        "model.started",
        "model.streaming",
        "model.completed",
        "model.failed",
        "tool.requested",
        "tool.started",
        "tool.completed",
        "tool.failed",
        "skill.requested",
        "skill.started",
        "skill.completed",
        "skill.failed",
        "mcp.requested",
        "mcp.started",
        "mcp.completed",
        "mcp.failed",
        "approval.requested",
        "approval.resolved",
        "approval.cancelled",
        "auth.challenge_requested",
        "auth.resolved",
        "auth.failed",
        "policy.exposure_denied",
        "policy.surface_deferred",
        "policy.session_compiled",
        "trace.emitted",
        "subrun.spawned",
        "subrun.cancelled",
        "subrun.completed",
        "subrun.failed",
        "workflow.started",
        "workflow.step.started",
        "workflow.step.completed",
        "workflow.completed",
        "workflow.failed",
        "background.started",
        "background.paused",
        "background.completed",
        "background.failed",
        "runtime.run.updated",
        "runtime.message.created",
        "runtime.trace.emitted",
        "runtime.approval.requested",
        "runtime.approval.resolved",
        "runtime.session.updated",
        "runtime.error",
        "memory.selected",
        "memory.proposed",
        "memory.approved",
        "memory.rejected",
        "memory.revalidated",
    ];
    for event in &events {
        let kind = event.kind.as_deref().unwrap_or(event.event_type.as_str());
        assert!(
            allowed.contains(&kind),
            "unexpected runtime event kind: {kind}"
        );
    }

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_selects_runtime_memory_and_emits_memory_events() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the first explicit deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("First deliverable".into()),
                preview_kind: "markdown".into(),
                file_name: Some("first-deliverable.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some("# First deliverable\n\nProduce the first draft".into()),
                data_base64: None,
            }],
        }),
    );
    adapter
        .persist_runtime_memory_record(
            &memory_runtime::PersistedRuntimeMemoryRecord {
                memory_id: "mem-user-preference".into(),
                project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
                owner_ref: Some("user:user-owner".into()),
                source_run_id: Some("seed-run".into()),
                kind: "user".into(),
                scope: "user-private".into(),
                title: "user memory".into(),
                summary: "Remember the user's approval preference.".into(),
                freshness_state: "fresh".into(),
                last_validated_at: Some(1),
                proposal_state: "approved".into(),
                storage_path: None,
                content_hash: None,
                updated_at: 1,
            },
            &json!({ "summary": "Remember the user's approval preference." }),
        )
        .expect("persist runtime memory");

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-events",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Events",
                &agent_actor_ref,
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
            SubmitRuntimeTurnInput {
                content: "Remember this explicit feedback for later turns.".into(),
                permission_mode: None,
                recall_mode: Some("default".into()),
                ignored_memory_ids: Vec::new(),
                memory_intent: Some("feedback".into()),
            },
        )
        .await
        .expect("run");

    assert_eq!(run.selected_memory.len(), 1);
    assert_eq!(run.selected_memory[0].memory_id, "mem-user-preference");
    assert_eq!(
        run.freshness_summary
            .as_ref()
            .map(|value| value.fresh_count),
        Some(1)
    );
    assert_eq!(
        run.pending_memory_proposal
            .as_ref()
            .map(|proposal| proposal.proposal_state.as_str()),
        Some("pending")
    );

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("detail");
    assert_eq!(detail.memory_selection_summary.selected_count, 1);
    assert_eq!(detail.pending_memory_proposal_count, 1);

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    assert!(events
        .iter()
        .any(|event| event.event_type == "memory.selected"));
    assert!(events
        .iter()
        .any(|event| event.event_type == "memory.proposed"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_filters_runtime_memory_by_actor_scope_kind_and_owner() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the first explicit deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("First deliverable".into()),
                preview_kind: "markdown".into(),
                file_name: Some("first-deliverable.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some("# First deliverable\n\nProduce the first draft".into()),
                data_base64: None,
            }],
        }),
    );
    for record in [
        memory_runtime::PersistedRuntimeMemoryRecord {
            memory_id: "mem-owned-agent".into(),
            project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
            owner_ref: Some(agent_actor_ref.clone()),
            source_run_id: Some("seed-run".into()),
            kind: "reference".into(),
            scope: "agent-private".into(),
            title: "Owned agent memory".into(),
            summary: "Provide concise implementation summaries with direct next steps.".into(),
            freshness_state: "fresh".into(),
            last_validated_at: Some(5),
            proposal_state: "approved".into(),
            storage_path: None,
            content_hash: None,
            updated_at: 5,
        },
        memory_runtime::PersistedRuntimeMemoryRecord {
            memory_id: "mem-owned-user".into(),
            project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
            owner_ref: Some("user:user-owner".into()),
            source_run_id: Some("seed-run".into()),
            kind: "user".into(),
            scope: "user-private".into(),
            title: "User preference".into(),
            summary: "The user prefers concise implementation summaries.".into(),
            freshness_state: "fresh".into(),
            last_validated_at: Some(4),
            proposal_state: "approved".into(),
            storage_path: None,
            content_hash: None,
            updated_at: 4,
        },
        memory_runtime::PersistedRuntimeMemoryRecord {
            memory_id: "mem-other-agent".into(),
            project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
            owner_ref: Some("agent:agent-other".into()),
            source_run_id: Some("seed-run".into()),
            kind: "reference".into(),
            scope: "agent-private".into(),
            title: "Foreign agent memory".into(),
            summary: "Do not expose this memory to another actor.".into(),
            freshness_state: "fresh".into(),
            last_validated_at: Some(6),
            proposal_state: "approved".into(),
            storage_path: None,
            content_hash: None,
            updated_at: 6,
        },
        memory_runtime::PersistedRuntimeMemoryRecord {
            memory_id: "mem-unknown-kind".into(),
            project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
            owner_ref: Some("user:user-owner".into()),
            source_run_id: Some("seed-run".into()),
            kind: "scratchpad".into(),
            scope: "user-private".into(),
            title: "Unsupported kind".into(),
            summary: "Unsupported runtime memory kind should not be selected.".into(),
            freshness_state: "fresh".into(),
            last_validated_at: Some(7),
            proposal_state: "approved".into(),
            storage_path: None,
            content_hash: None,
            updated_at: 7,
        },
    ] {
        adapter
            .persist_runtime_memory_record(&record, &json!({ "summary": record.summary }))
            .expect("persist runtime memory");
    }

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-selector-gating",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Selector Gating",
                &agent_actor_ref,
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
            SubmitRuntimeTurnInput {
                content: "Give concise implementation summaries with the next steps.".into(),
                permission_mode: None,
                recall_mode: Some("default".into()),
                ignored_memory_ids: Vec::new(),
                memory_intent: None,
            },
        )
        .await
        .expect("run");
    let selected_ids = run
        .selected_memory
        .iter()
        .map(|item| item.memory_id.as_str())
        .collect::<Vec<_>>();
    assert!(selected_ids.contains(&"mem-owned-agent"));
    assert!(selected_ids.contains(&"mem-owned-user"));
    assert!(!selected_ids.contains(&"mem-other-agent"));
    assert!(!selected_ids.contains(&"mem-unknown-kind"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_prefers_project_memory_from_subrun_lineage_over_unrelated_branch_memory() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let team_actor_ref = builtin_team_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the first explicit deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("First deliverable".into()),
                preview_kind: "markdown".into(),
                file_name: Some("first-deliverable.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some("# First deliverable\n\nProduce the first draft".into()),
                data_base64: None,
            }],
        }),
    );
    for record in [
        memory_runtime::PersistedRuntimeMemoryRecord {
            memory_id: "mem-lineage-related".into(),
            project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
            owner_ref: Some(format!("project:{}", octopus_core::DEFAULT_PROJECT_ID)),
            source_run_id: Some("run-lineage-subrun".into()),
            kind: "project".into(),
            scope: "project-shared".into(),
            title: "Workflow checklist".into(),
            summary: "Approval reviews need the finance tag on every request.".into(),
            freshness_state: "fresh".into(),
            last_validated_at: Some(2),
            proposal_state: "approved".into(),
            storage_path: None,
            content_hash: None,
            updated_at: 2,
        },
        memory_runtime::PersistedRuntimeMemoryRecord {
            memory_id: "mem-lineage-unrelated".into(),
            project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
            owner_ref: Some(format!("project:{}", octopus_core::DEFAULT_PROJECT_ID)),
            source_run_id: Some("run-unrelated".into()),
            kind: "project".into(),
            scope: "project-shared".into(),
            title: "Workflow checklist".into(),
            summary: "Approval reviews need the finance tag on every request.".into(),
            freshness_state: "fresh".into(),
            last_validated_at: Some(20),
            proposal_state: "approved".into(),
            storage_path: None,
            content_hash: None,
            updated_at: 20,
        },
    ] {
        adapter
            .persist_runtime_memory_record(&record, &json!({ "summary": record.summary }))
            .expect("persist runtime memory");
    }

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-lineage",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Lineage",
                &team_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    {
        let mut sessions = adapter
            .state
            .sessions
            .lock()
            .expect("runtime sessions mutex");
        let aggregate = sessions
            .get_mut(&session.summary.id)
            .expect("runtime aggregate");
        let run_id = aggregate.detail.run.id.clone();
        aggregate.detail.run.parent_run_id = Some("run-lineage-parent".into());
        aggregate.detail.run.workflow_run = Some("workflow-lineage".into());
        aggregate.detail.run.workflow_run_detail = Some(RuntimeWorkflowRunDetail {
            workflow_run_id: "workflow-lineage".into(),
            status: "running".into(),
            current_step_id: Some("run-lineage-subrun".into()),
            current_step_label: Some("Worker".into()),
            total_steps: 2,
            completed_steps: 1,
            background_capable: false,
            steps: vec![
                RuntimeWorkflowStepSummary {
                    step_id: run_id.clone(),
                    node_kind: "leader".into(),
                    label: "Leader plan".into(),
                    actor_ref: aggregate.detail.run.actor_ref.clone(),
                    run_id: Some(run_id.clone()),
                    parent_run_id: Some("run-lineage-parent".into()),
                    delegated_by_tool_call_id: None,
                    mailbox_ref: None,
                    handoff_ref: None,
                    status: "completed".into(),
                    started_at: aggregate.detail.run.started_at,
                    updated_at: aggregate.detail.run.updated_at,
                },
                RuntimeWorkflowStepSummary {
                    step_id: "run-lineage-subrun".into(),
                    node_kind: "worker".into(),
                    label: "Worker".into(),
                    actor_ref: "agent:worker-runtime".into(),
                    run_id: Some("run-lineage-subrun".into()),
                    parent_run_id: Some(run_id.clone()),
                    delegated_by_tool_call_id: Some("tool-lineage".into()),
                    mailbox_ref: Some("mailbox-lineage".into()),
                    handoff_ref: Some("handoff-lineage".into()),
                    status: "completed".into(),
                    started_at: aggregate.detail.run.started_at,
                    updated_at: aggregate.detail.run.updated_at,
                },
            ],
            blocking: None,
        });
        aggregate.detail.subruns = vec![RuntimeSubrunSummary {
            run_id: "run-lineage-subrun".into(),
            parent_run_id: Some(run_id),
            actor_ref: "agent:worker-runtime".into(),
            label: "Worker".into(),
            status: "completed".into(),
            run_kind: "subrun".into(),
            delegated_by_tool_call_id: Some("tool-lineage".into()),
            workflow_run_id: Some("workflow-lineage".into()),
            mailbox_ref: Some("mailbox-lineage".into()),
            handoff_ref: Some("handoff-lineage".into()),
            started_at: aggregate.detail.run.started_at,
            updated_at: aggregate.detail.run.updated_at,
        }];
    }

    let run = adapter
        .submit_turn(
            &session.summary.id,
            SubmitRuntimeTurnInput {
                content: "Approval reviews need the finance tag on every request.".into(),
                permission_mode: None,
                recall_mode: Some("default".into()),
                ignored_memory_ids: Vec::new(),
                memory_intent: None,
            },
        )
        .await
        .expect("run");

    assert_eq!(
        run.selected_memory
            .first()
            .map(|item| item.memory_id.as_str()),
        Some("mem-lineage-related")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_rejects_memory_pollution_candidates() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the first explicit deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("First deliverable".into()),
                preview_kind: "markdown".into(),
                file_name: Some("first-deliverable.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some("# First deliverable\n\nProduce the first draft".into()),
                data_base64: None,
            }],
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-pollution",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Pollution",
                &agent_actor_ref,
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
            SubmitRuntimeTurnInput {
                content: "For this task, keep the TODO list open for now.".into(),
                permission_mode: None,
                recall_mode: Some("default".into()),
                ignored_memory_ids: Vec::new(),
                memory_intent: Some("project".into()),
            },
        )
        .await
        .expect("run");

    assert!(run.pending_memory_proposal.is_none());
    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    assert!(!events
        .iter()
        .any(|event| event.event_type == "memory.proposed"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn resolving_memory_proposal_persists_runtime_memory_record_and_event() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the first explicit deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("First deliverable".into()),
                preview_kind: "markdown".into(),
                file_name: Some("first-deliverable.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some("# First deliverable\n\nProduce the first draft".into()),
                data_base64: None,
            }],
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-resolution",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Resolution",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let submitted = adapter
        .submit_turn(
            &session.summary.id,
            SubmitRuntimeTurnInput {
                content: "Please remember that approval reviews need the finance tag.".into(),
                permission_mode: None,
                recall_mode: Some("default".into()),
                ignored_memory_ids: Vec::new(),
                memory_intent: Some("feedback".into()),
            },
        )
        .await
        .expect("submitted");
    let proposal_id = submitted
        .pending_memory_proposal
        .as_ref()
        .map(|proposal| proposal.proposal_id.clone())
        .expect("pending proposal");

    let resolved = adapter
        .resolve_memory_proposal(
            &session.summary.id,
            &proposal_id,
            ResolveRuntimeMemoryProposalInput {
                decision: "approve".into(),
                note: Some("validated".into()),
            },
        )
        .await
        .expect("resolved");
    assert!(resolved.pending_memory_proposal.is_none());

    let records = adapter
        .load_runtime_memory_records(octopus_core::DEFAULT_PROJECT_ID)
        .expect("memory records");
    assert!(records.iter().any(|record| {
        record.summary == "Please remember that approval reviews need the finance tag."
            && record.proposal_state == "approved"
            && record.freshness_state == "fresh"
    }));
    assert!(
        records
            .iter()
            .find(|record| record.summary
                == "Please remember that approval reviews need the finance tag.")
            .map(|record| adapter.runtime_memory_body_path(&record.memory_id))
            .is_some_and(|path| path.exists()),
        "memory body should be persisted under data/knowledge"
    );

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    assert!(events
        .iter()
        .any(|event| event.event_type == "memory.approved"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn revalidating_existing_memory_refreshes_existing_record_in_place() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(12),
        }),
    );
    adapter
        .persist_runtime_memory_record(
            &memory_runtime::PersistedRuntimeMemoryRecord {
                memory_id: "mem-stale-feedback".into(),
                project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
                owner_ref: Some("user:runtime".into()),
                source_run_id: Some("seed-run".into()),
                kind: "feedback".into(),
                scope: "user-private".into(),
                title: "feedback memory".into(),
                summary: "Approval reviews need the finance tag on every request.".into(),
                freshness_state: "stale".into(),
                last_validated_at: Some(1),
                proposal_state: "approved".into(),
                storage_path: None,
                content_hash: None,
                updated_at: 1,
            },
            &json!({
                "kind": "feedback",
                "normalizedContent": "Approval reviews need the finance tag on every request.",
                "summary": "Approval reviews need the finance tag on every request."
            }),
        )
        .expect("seed stale memory");

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-revalidation",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Revalidation",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let submitted = adapter
        .submit_turn(
            &session.summary.id,
            SubmitRuntimeTurnInput {
                content: "Approval reviews need the finance tag on every request.".into(),
                permission_mode: None,
                recall_mode: Some("skip".into()),
                ignored_memory_ids: Vec::new(),
                memory_intent: Some("feedback".into()),
            },
        )
        .await
        .expect("submitted");
    let proposal = submitted
        .pending_memory_proposal
        .as_ref()
        .expect("pending proposal");
    assert_eq!(proposal.memory_id, "mem-stale-feedback");

    let resolved = adapter
        .resolve_memory_proposal(
            &session.summary.id,
            &proposal.proposal_id,
            ResolveRuntimeMemoryProposalInput {
                decision: "revalidate".into(),
                note: Some("freshened".into()),
            },
        )
        .await
        .expect("resolved");
    assert!(resolved.pending_memory_proposal.is_none());

    let records = adapter
        .load_runtime_memory_records(octopus_core::DEFAULT_PROJECT_ID)
        .expect("memory records");
    let record = records
        .iter()
        .find(|record| record.memory_id == "mem-stale-feedback")
        .expect("revalidated memory record");
    assert_eq!(record.freshness_state, "revalidated");
    assert_eq!(record.proposal_state, "revalidated");

    let body: serde_json::Value = serde_json::from_slice(
        &fs::read(adapter.runtime_memory_body_path("mem-stale-feedback"))
            .expect("memory body bytes"),
    )
    .expect("memory body json");
    assert_eq!(
        body.get("normalizedContent")
            .and_then(serde_json::Value::as_str),
        Some("Approval reviews need the finance tag on every request.")
    );
    assert_eq!(
        body.get("review")
            .and_then(|value| value.get("decision"))
            .and_then(serde_json::Value::as_str),
        Some("revalidate")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn memory_proposal_mediation_targets_specific_proposal_not_durable_memory_id() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(12),
        }),
    );
    adapter
        .persist_runtime_memory_record(
            &memory_runtime::PersistedRuntimeMemoryRecord {
                memory_id: "mem-stale-feedback".into(),
                project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
                owner_ref: Some("user:runtime".into()),
                source_run_id: Some("seed-run".into()),
                kind: "feedback".into(),
                scope: "user-private".into(),
                title: "feedback memory".into(),
                summary: "Approval reviews need the finance tag on every request.".into(),
                freshness_state: "stale".into(),
                last_validated_at: Some(1),
                proposal_state: "approved".into(),
                storage_path: None,
                content_hash: None,
                updated_at: 1,
            },
            &json!({
                "kind": "feedback",
                "normalizedContent": "Approval reviews need the finance tag on every request.",
                "summary": "Approval reviews need the finance tag on every request."
            }),
        )
        .expect("seed stale memory");

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-mediation-target",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Mediation Target",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let submitted = adapter
        .submit_turn(
            &session.summary.id,
            SubmitRuntimeTurnInput {
                content: "Approval reviews need the finance tag on every request.".into(),
                permission_mode: None,
                recall_mode: Some("skip".into()),
                ignored_memory_ids: Vec::new(),
                memory_intent: Some("feedback".into()),
            },
        )
        .await
        .expect("submitted");
    let proposal = submitted
        .pending_memory_proposal
        .as_ref()
        .expect("pending proposal");
    assert_eq!(proposal.memory_id, "mem-stale-feedback");
    assert_ne!(proposal.proposal_id, proposal.memory_id);
    assert_eq!(
        submitted
            .pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("memory-write")
    );
    assert_eq!(
        submitted
            .pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_ref.as_str()),
        Some(proposal.proposal_id.as_str())
    );

    let resolved = adapter
        .resolve_memory_proposal(
            &session.summary.id,
            &proposal.proposal_id,
            ResolveRuntimeMemoryProposalInput {
                decision: "revalidate".into(),
                note: Some("freshened".into()),
            },
        )
        .await
        .expect("resolved");
    assert_eq!(
        resolved
            .last_mediation_outcome
            .as_ref()
            .map(|outcome| outcome.target_kind.as_str()),
        Some("memory-write")
    );
    assert_eq!(
        resolved
            .last_mediation_outcome
            .as_ref()
            .map(|outcome| outcome.target_ref.as_str()),
        Some(proposal.proposal_id.as_str())
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn create_session_populates_real_capability_plan_and_state_snapshot() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-capability-plan",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Capability Plan Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Use the runtime capability planner.",
                serde_json::to_string(&vec!["bash", "WebFetch"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for capability plan tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert capability agent");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-capability-plan",
                octopus_core::DEFAULT_PROJECT_ID,
                "Capability Plan Session",
                "agent:agent-capability-plan",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    assert_eq!(
        session.capability_summary.visible_tools,
        vec!["bash".to_string()]
    );
    assert_eq!(
        session.capability_summary.deferred_tools,
        vec!["WebFetch".to_string()]
    );
    assert!(session.provider_state_summary.is_empty());
    assert!(session.pending_mediation.is_none());
    assert!(session.last_execution_outcome.is_none());
    assert!(session.capability_state_ref.is_some());
    assert_eq!(
        session.run.capability_plan_summary,
        session.capability_summary
    );
    assert_eq!(
        session.run.checkpoint.capability_plan_summary,
        session.capability_summary
    );
    assert_eq!(session.run.checkpoint.current_iteration_index, 0);
    assert!(session.run.checkpoint.capability_state_ref.is_some());

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let persisted: (String, String, i64, i64, i64, i64) = connection
        .query_row(
            "SELECT capability_plan_summary_json, capability_state_ref, granted_tool_count, injected_skill_message_count, deferred_capability_count, hidden_capability_count
             FROM runtime_session_projections
             WHERE id = ?1",
            [&session.summary.id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("session capability projection");
    let summary: RuntimeCapabilityPlanSummary =
        serde_json::from_str(&persisted.0).expect("capability plan summary json");
    assert_eq!(summary.visible_tools, vec!["bash".to_string()]);
    assert_eq!(summary.deferred_tools, vec!["WebFetch".to_string()]);
    assert_eq!(
        persisted.1,
        session.capability_state_ref.clone().expect("state ref")
    );
    assert_eq!(persisted.2, 0);
    assert_eq!(persisted.3, 0);
    assert_eq!(persisted.4, 1);
    assert!(persisted.5 >= 0);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn create_session_includes_selected_plugin_tools_in_capability_plan() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_external_plugin(&root, "sample-plugin", "sample-plugin", "plugin_echo");
    write_workspace_config_with_plugins(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
        json!({
            "sample-plugin@external": true
        }),
        &["./external-plugins"],
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, capability_policy_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "agent-plugin-capability-plan",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Plugin Capability Plan Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Use plugin tools through the runtime capability planner.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for plugin capability plan tests.",
                serde_json::to_string(&json!({
                    "pluginCapabilityRefs": ["plugin_echo"]
                }))
                .expect("capability policy"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert plugin capability agent");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-plugin-capability-plan",
                octopus_core::DEFAULT_PROJECT_ID,
                "Plugin Capability Plan Session",
                "agent:agent-plugin-capability-plan",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    assert!(
        session
            .capability_summary
            .deferred_tools
            .contains(&"plugin_echo".to_string()),
        "selected plugin tool should be planned as a deferred runtime capability"
    );
    assert!(
        session.provider_state_summary.iter().any(|provider| {
            provider.provider_key == "sample-plugin@external" && provider.state == "ready"
        }),
        "selected plugin provider should surface as ready"
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_replans_and_executes_selected_plugin_tools_through_capability_runtime() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_external_plugin(&root, "sample-plugin", "sample-plugin", "plugin_echo");
    write_workspace_config_with_plugins(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
        json!({
            "sample-plugin@external": true
        }),
        &["./external-plugins"],
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, capability_policy_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "agent-plugin-runtime-loop",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Plugin Runtime Loop Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Discover and use plugin tools through the runtime capability planner.",
                serde_json::to_string(&vec!["ToolSearch"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for plugin runtime loop tests.",
                serde_json::to_string(&json!({
                    "pluginCapabilityRefs": ["plugin_echo"]
                }))
                .expect("capability policy"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert plugin runtime loop agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Selecting the plugin tool.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-select-plugin-echo".into(),
                name: "ToolSearch".into(),
                input: serde_json::json!({
                    "query": "select:plugin_echo",
                    "max_results": 5
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 5,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Running the selected plugin tool.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-plugin-echo".into(),
                name: "plugin_echo".into(),
                input: serde_json::json!({
                    "message": "hello from plugin"
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 6,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Plugin tool completed.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 7,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
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
                "conv-plugin-runtime-loop",
                octopus_core::DEFAULT_PROJECT_ID,
                "Plugin Runtime Loop Session",
                "agent:agent-plugin-runtime-loop",
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
            turn_input("Discover and run the plugin tool", None),
        )
        .await
        .expect("run");

    assert_eq!(run.status, "completed");
    assert_eq!(run.current_step, "completed");
    assert_eq!(executor.request_count(), 3);

    let requests = executor.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(
        requests[0]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["ToolSearch"]
    );
    assert!(
        requests[1]
            .tools
            .iter()
            .any(|tool| tool.name == "plugin_echo"),
        "selected plugin tool should be replanned into the next model request"
    );
    assert!(
        requests[2]
            .messages
            .iter()
            .any(|message| matches!(message.role, runtime::MessageRole::Tool)),
        "final model request should include the plugin tool result"
    );

    let output_artifact_ref = run
        .artifact_refs
        .first()
        .cloned()
        .expect("runtime output artifact ref");
    let output_artifact: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            root.join("data")
                .join("artifacts")
                .join("runtime")
                .join(format!("{output_artifact_ref}.json")),
        )
        .expect("runtime output artifact"),
    )
    .expect("runtime output artifact json");
    let serialized_messages = output_artifact
        .get("serializedSession")
        .and_then(|value| value.get("session"))
        .and_then(|value| value.get("messages"))
        .and_then(serde_json::Value::as_array)
        .expect("serialized runtime messages");
    let plugin_tool_result = serialized_messages
        .iter()
        .flat_map(|message| {
            message
                .get("blocks")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
        })
        .find(|block| {
            block.get("type").and_then(serde_json::Value::as_str) == Some("tool_result")
                && block.get("toolName").and_then(serde_json::Value::as_str) == Some("plugin_echo")
        })
        .expect("plugin tool result");
    assert_eq!(
        plugin_tool_result
            .get("isError")
            .and_then(serde_json::Value::as_bool),
        Some(false),
        "plugin tool should execute successfully through the runtime capability bridge"
    );

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    let event_kinds = events
        .iter()
        .map(|event| event.kind.as_deref().unwrap_or(event.event_type.as_str()))
        .collect::<Vec<_>>();
    assert!(event_kinds.contains(&"tool.completed"));
    assert!(
        !event_kinds.contains(&"tool.failed"),
        "plugin capability execution should not fall back to a failed tool event"
    );
    let planner_completed = events
        .iter()
        .filter(|event| event.kind.as_deref() == Some("planner.completed"))
        .collect::<Vec<_>>();
    assert!(
        planner_completed.len() >= 2,
        "runtime loop should emit planner completion events for real replans"
    );
    assert!(planner_completed.iter().any(|event| {
        event
            .capability_plan_summary
            .as_ref()
            .is_some_and(|summary| summary.visible_tools.contains(&"plugin_echo".to_string()))
    }));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn non_coding_research_docs_agent_runs_through_same_capability_trunk() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_external_plugin(&root, "sample-plugin", "sample-plugin", "plugin_echo");
    write_workspace_config_with_plugins(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
        json!({
            "sample-plugin@external": true
        }),
        &["./external-plugins"],
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, task_domains, description, capability_policy_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                "agent-research-docs-runtime-loop",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Research Docs Agent",
                Option::<String>::None,
                "Evidence-driven researcher",
                serde_json::to_string(&vec!["research", "docs"]).expect("tags"),
                "Discover research helpers and summarize documentation sources.",
                serde_json::to_string(&vec!["ToolSearch"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&vec!["research", "docs"]).expect("task domains"),
                "Agent for non-coding research/docs runtime acceptance tests.",
                serde_json::to_string(&json!({
                    "pluginCapabilityRefs": ["plugin_echo"]
                }))
                .expect("capability policy"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert research docs agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Selecting the research helper.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-select-plugin-echo".into(),
                name: "ToolSearch".into(),
                input: serde_json::json!({
                    "query": "select:plugin_echo",
                    "max_results": 5
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 5,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Running the research helper.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-plugin-echo".into(),
                name: "plugin_echo".into(),
                input: serde_json::json!({
                    "message": "Summarize the docs findings"
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 6,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Research summary prepared.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 7,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
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
                "conv-research-docs-runtime-loop",
                octopus_core::DEFAULT_PROJECT_ID,
                "Research Docs Runtime Loop Session",
                "agent:agent-research-docs-runtime-loop",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    assert!(
        session
            .capability_summary
            .deferred_tools
            .contains(&"plugin_echo".to_string()),
        "research/docs actor should receive the same deferred capability surface"
    );

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Research the docs and summarize the findings", None),
        )
        .await
        .expect("run");

    assert_eq!(run.status, "completed");
    assert_eq!(run.current_step, "completed");
    assert_eq!(executor.request_count(), 3);

    let requests = executor.requests();
    assert_eq!(requests.len(), 3);
    let first_prompt = requests[0].system_prompt.join("\n\n");
    assert!(
        first_prompt.contains("Task domains: research, docs."),
        "system prompt should preserve the non-coding research/docs actor domain"
    );
    assert_eq!(
        requests[0]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["ToolSearch"]
    );
    assert!(
        requests[1]
            .tools
            .iter()
            .any(|tool| tool.name == "plugin_echo"),
        "selected capability should be replanned into the next model request"
    );
    assert!(
        requests[2]
            .messages
            .iter()
            .any(|message| matches!(message.role, runtime::MessageRole::Tool)),
        "final request should include the research helper result"
    );

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    let event_kinds = events
        .iter()
        .map(|event| event.kind.as_deref().unwrap_or(event.event_type.as_str()))
        .collect::<Vec<_>>();
    assert!(event_kinds.contains(&"tool.completed"));
    assert!(
        !event_kinds.contains(&"tool.failed"),
        "non-coding research/docs actor should stay on the same successful capability trunk"
    );
    let planner_completed = events
        .iter()
        .filter(|event| event.kind.as_deref() == Some("planner.completed"))
        .collect::<Vec<_>>();
    assert!(
        planner_completed.len() >= 2,
        "research/docs runtime loop should emit real replans"
    );
    assert!(planner_completed.iter().any(|event| {
        event
            .capability_plan_summary
            .as_ref()
            .is_some_and(|summary| summary.visible_tools.contains(&"plugin_echo".to_string()))
    }));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_requiring_approval_persists_real_mediation_and_outcome() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "agent-capability-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Capability Approval Agent",
                Option::<String>::None,
                "Approver",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Use the runtime capability planner.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for capability approval tests.",
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
        .expect("upsert approval agent");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-capability-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "Capability Approval Session",
                "agent:agent-capability-approval",
                Some("quota-model"),
                "workspace-write",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Do the write action", Some("workspace-write")),
        )
        .await
        .expect("run");
    let expected_target_ref = format!("model-execution:{}:quota-model", run.id);

    let pending = run.pending_mediation.clone().expect("pending mediation");
    assert_eq!(
        pending.tool_name.as_deref(),
        Some("Capability Approval Agent")
    );
    assert_eq!(pending.mediation_kind, "approval");
    assert_eq!(pending.target_kind, "model-execution");
    assert_eq!(pending.target_ref, expected_target_ref);
    assert_eq!(pending.dispatch_kind.as_deref(), Some("model_execution"));
    assert_eq!(pending.concurrency_policy.as_deref(), Some("serialized"));
    assert_eq!(
        pending
            .input
            .as_ref()
            .and_then(|value| value.get("content")),
        Some(&json!("Do the write action"))
    );
    assert_eq!(
        pending
            .input
            .as_ref()
            .and_then(|value| value.get("requestedPermissionMode")),
        Some(&json!("workspace-write"))
    );
    assert!(run.capability_state_ref.is_some());
    assert_eq!(run.checkpoint.current_iteration_index, 0);
    assert_eq!(
        run.checkpoint.tool_name.as_deref(),
        Some("Capability Approval Agent")
    );
    assert_eq!(
        run.checkpoint.dispatch_kind.as_deref(),
        Some("model_execution")
    );
    assert_eq!(
        run.checkpoint.concurrency_policy.as_deref(),
        Some("serialized")
    );
    assert_eq!(
        run.checkpoint
            .input
            .as_ref()
            .and_then(|value| value.get("content")),
        Some(&json!("Do the write action"))
    );
    assert_eq!(
        run.approval_target
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("model-execution")
    );
    assert_eq!(
        run.approval_target
            .as_ref()
            .and_then(|approval| approval.target_ref.as_deref()),
        Some(expected_target_ref.as_str())
    );
    assert_eq!(
        run.approval_target
            .as_ref()
            .and_then(|approval| approval.dispatch_kind.as_deref()),
        Some("model_execution")
    );
    assert_eq!(
        run.approval_target
            .as_ref()
            .and_then(|approval| approval.concurrency_policy.as_deref()),
        Some("serialized")
    );
    assert_eq!(
        run.approval_target
            .as_ref()
            .and_then(|approval| approval.input.as_ref())
            .and_then(|value| value.get("content")),
        Some(&json!("Do the write action"))
    );
    assert!(run
        .checkpoint
        .checkpoint_artifact_ref
        .as_deref()
        .is_some_and(|value| value.contains("runtime/checkpoints/mediation/")));
    let outcome = run
        .last_execution_outcome
        .clone()
        .expect("last execution outcome");
    assert_eq!(
        outcome.tool_name.as_deref(),
        Some("Capability Approval Agent")
    );
    assert_eq!(outcome.outcome, "require_approval");
    assert!(outcome.requires_approval);
    assert_eq!(
        run.checkpoint
            .pending_mediation
            .as_ref()
            .and_then(|value| value.tool_name.as_deref()),
        Some("Capability Approval Agent")
    );
    assert_eq!(
        run.checkpoint
            .pending_mediation
            .as_ref()
            .and_then(|value| value.dispatch_kind.as_deref()),
        Some("model_execution")
    );
    assert_eq!(
        run.checkpoint
            .pending_mediation
            .as_ref()
            .and_then(|value| value.concurrency_policy.as_deref()),
        Some("serialized")
    );
    assert_eq!(
        run.checkpoint
            .pending_mediation
            .as_ref()
            .and_then(|value| value.input.as_ref())
            .and_then(|value| value.get("content")),
        Some(&json!("Do the write action"))
    );
    assert_eq!(
        run.checkpoint.target_kind.as_deref(),
        Some("model-execution")
    );
    assert_eq!(
        run.checkpoint.target_ref.as_deref(),
        Some(expected_target_ref.as_str())
    );
    assert_eq!(
        run.checkpoint
            .last_execution_outcome
            .as_ref()
            .map(|value| value.outcome.as_str()),
        Some("require_approval")
    );
    assert_eq!(
        run.checkpoint
            .last_execution_outcome
            .as_ref()
            .and_then(|value| value.dispatch_kind.as_deref()),
        Some("model_execution")
    );
    assert_eq!(
        run.checkpoint
            .last_execution_outcome
            .as_ref()
            .and_then(|value| value.concurrency_policy.as_deref()),
        Some("serialized")
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let persisted: (String, String, String, String, i64) = connection
        .query_row(
            "SELECT pending_mediation_json, capability_state_ref, last_execution_outcome_json, capability_plan_summary_json, deferred_capability_count
             FROM runtime_run_projections
             WHERE id = ?1",
            [&run.id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("run capability projection");
    let persisted_pending: RuntimePendingMediationSummary =
        serde_json::from_str(&persisted.0).expect("pending mediation json");
    assert_eq!(
        persisted_pending.tool_name.as_deref(),
        Some("Capability Approval Agent")
    );
    assert_eq!(persisted_pending.target_kind, "model-execution");
    assert_eq!(persisted_pending.target_ref, expected_target_ref);
    assert_eq!(
        persisted_pending.dispatch_kind.as_deref(),
        Some("model_execution")
    );
    assert_eq!(
        persisted_pending.concurrency_policy.as_deref(),
        Some("serialized")
    );
    assert_eq!(
        persisted_pending
            .input
            .as_ref()
            .and_then(|value| value.get("content")),
        Some(&json!("Do the write action"))
    );
    assert_eq!(
        persisted.1,
        run.capability_state_ref.clone().expect("state ref")
    );
    let persisted_outcome: RuntimeCapabilityExecutionOutcome =
        serde_json::from_str(&persisted.2).expect("outcome json");
    assert_eq!(persisted_outcome.outcome, "require_approval");
    assert_eq!(
        persisted_outcome.dispatch_kind.as_deref(),
        Some("model_execution")
    );
    assert_eq!(
        persisted_outcome.concurrency_policy.as_deref(),
        Some("serialized")
    );
    let persisted_plan: RuntimeCapabilityPlanSummary =
        serde_json::from_str(&persisted.3).expect("plan json");
    assert_eq!(persisted_plan.visible_tools, vec!["bash".to_string()]);
    assert_eq!(persisted.4, 0);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_with_workspace_write_does_not_require_blanket_model_execution_approval() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-workspace-write-submit",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Workspace Write Agent",
                Option::<String>::None,
                "Operator",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Use the runtime capability planner.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for workspace-write submit tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workspace-write agent");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-workspace-write-submit",
                octopus_core::DEFAULT_PROJECT_ID,
                "Workspace Write Session",
                "agent:agent-workspace-write-submit",
                Some("quota-model"),
                octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Do the workspace write action", Some("workspace-write")),
        )
        .await
        .expect("run");

    assert_eq!(run.status, "completed");
    assert_eq!(run.current_step, "completed");
    assert!(run.pending_mediation.is_none());
    assert!(run.approval_target.is_none());
    assert_eq!(run.approval_state, "not-required");

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_uses_compiled_model_execution_policy_for_tool_execution_approval() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, memory_policy_json, delegation_policy_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "agent-tool-execution-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Tool Execution Approval Agent",
                Option::<String>::None,
                "Approver",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Require approval before model execution starts.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for model-execution policy tests.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({})).expect("permission envelope"),
                serde_json::to_string(&json!({})).expect("memory policy"),
                serde_json::to_string(&json!({})).expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "require-approval",
                    "workflowEscalation": "require-approval"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert tool execution approval agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![vec![
        runtime::AssistantEvent::TextDelta("This should never execute.".into()),
        runtime::AssistantEvent::MessageStop,
    ]]));
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
                "conv-tool-execution-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "Tool Execution Approval Session",
                "agent:agent-tool-execution-approval",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let session_policy = adapter
        .load_session_policy_snapshot(&format!("{}-policy", session.summary.id))
        .expect("session policy snapshot");
    let execution_policy = session_policy
        .target_decisions
        .get("model-execution:quota-model")
        .expect("model execution policy decision");
    assert_eq!(execution_policy.target_kind, "model-execution");
    assert_eq!(execution_policy.action, "requireApproval");
    assert!(execution_policy.deferred);
    assert!(execution_policy.requires_approval);
    assert_eq!(
        execution_policy.required_permission.as_deref(),
        Some("read-only")
    );

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Do not start model execution yet", None),
        )
        .await
        .expect("pending run");

    assert_eq!(run.status, "waiting_approval");
    assert_eq!(run.current_step, "awaiting_approval");
    assert_eq!(executor.request_count(), 0);
    assert_eq!(
        run.pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("model-execution")
    );
    assert_eq!(
        run.approval_target
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("model-execution")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn resolving_a_consumed_runtime_approval_returns_conflict() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, memory_policy_json, delegation_policy_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "agent-consumed-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Consumed Approval Agent",
                Option::<String>::None,
                "Approver",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Require approval before model execution starts.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for consumed approval tests.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({})).expect("permission envelope"),
                serde_json::to_string(&json!({})).expect("memory policy"),
                serde_json::to_string(&json!({})).expect("delegation policy"),
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
        .expect("upsert consumed approval agent");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-consumed-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "Consumed Approval Session",
                "agent:agent-consumed-approval",
                Some("quota-model"),
                octopus_core::RUNTIME_PERMISSION_READ_ONLY,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Pause for approval first", None),
        )
        .await
        .expect("pending run");
    assert_eq!(pending_run.status, "waiting_approval");

    let approval_id = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail")
        .pending_approval
        .map(|approval| approval.id)
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

    let error = adapter
        .resolve_approval(
            &session.summary.id,
            &approval_id,
            ResolveRuntimeApprovalInput {
                decision: "approve".into(),
            },
        )
        .await
        .expect_err("consumed approval should conflict");
    assert!(matches!(error, AppError::Conflict(_)));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn partial_approval_preference_json_merges_with_defaults_for_policy_compilation() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, memory_policy_json, delegation_policy_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "agent-partial-approval-policy",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Partial Approval Policy Agent",
                Option::<String>::None,
                "Approver",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Require approval before model execution starts.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for partial approval preference tests.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({})).expect("permission envelope"),
                serde_json::to_string(&json!({})).expect("memory policy"),
                serde_json::to_string(&json!({})).expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert partial approval policy agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![vec![
        runtime::AssistantEvent::TextDelta("This should never execute.".into()),
        runtime::AssistantEvent::MessageStop,
    ]]));
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
                "conv-partial-approval-policy",
                octopus_core::DEFAULT_PROJECT_ID,
                "Partial Approval Policy Session",
                "agent:agent-partial-approval-policy",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let session_policy = adapter
        .load_session_policy_snapshot(&format!("{}-policy", session.summary.id))
        .expect("session policy snapshot");
    let execution_policy = session_policy
        .target_decisions
        .get("model-execution:quota-model")
        .expect("model execution policy decision");
    assert_eq!(execution_policy.target_kind, "model-execution");
    assert_eq!(execution_policy.action, "requireApproval");
    assert!(execution_policy.deferred);
    assert!(execution_policy.requires_approval);
    assert_eq!(
        execution_policy.required_permission.as_deref(),
        Some("read-only")
    );

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Do not start model execution yet", None),
        )
        .await
        .expect("pending run");

    assert_eq!(run.status, "waiting_approval");
    assert_eq!(run.current_step, "awaiting_approval");
    assert_eq!(executor.request_count(), 0);
    assert_eq!(
        run.pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("model-execution")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn policy_compiler_uses_runtime_config_enablement_for_model_and_mcp_targets() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "configuredModels": {
                "quota-model": {
                    "configuredModelId": "quota-model",
                    "name": "Quota Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": "env:ANTHROPIC_API_KEY",
                    "enabled": false,
                    "source": "workspace"
                }
            }
        }),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, memory_policy_json, delegation_policy_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "agent-runtime-config-enablement",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Runtime Config Enablement Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["policy"]).expect("tags"),
                "Respect the frozen runtime policy.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&vec!["missing-mcp"]).expect("mcp server names"),
                "Agent for runtime config enablement policy tests.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({})).expect("permission envelope"),
                serde_json::to_string(&json!({})).expect("memory policy"),
                serde_json::to_string(&json!({})).expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "require-approval",
                    "workflowEscalation": "require-approval"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert enablement agent");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-runtime-config-enablement",
                octopus_core::DEFAULT_PROJECT_ID,
                "Runtime Config Enablement Session",
                "agent:agent-runtime-config-enablement",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let session_policy = adapter
        .load_session_policy_snapshot(&format!("{}-policy", session.summary.id))
        .expect("session policy snapshot");

    let execution_policy = session_policy
        .target_decisions
        .get("model-execution:quota-model")
        .expect("model execution policy decision");
    assert_eq!(execution_policy.action, "deny");
    assert!(execution_policy.hidden);
    assert_eq!(
        execution_policy.reason.as_deref(),
        Some("configured model is disabled or unavailable in the frozen runtime config")
    );

    let provider_auth_policy = session_policy
        .target_decisions
        .get("provider-auth:missing-mcp")
        .expect("provider auth policy decision");
    assert_eq!(provider_auth_policy.action, "deny");
    assert!(provider_auth_policy.hidden);
    assert_eq!(
        provider_auth_policy.reason.as_deref(),
        Some("provider or MCP server is not configured in the frozen runtime config")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn policy_compiler_uses_workspace_authorization_for_capability_buckets() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-workspace-authz",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Workspace Authz Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["policy"]).expect("tags"),
                "Respect workspace authorization.",
                serde_json::to_string(&vec!["shell"]).expect("builtin tool keys"),
                serde_json::to_string(&vec!["repo_search"]).expect("skill ids"),
                serde_json::to_string(&vec!["repo-mcp"]).expect("mcp server names"),
                "Agent for workspace authorization policy tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert authz agent");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-workspace-authz",
                octopus_core::DEFAULT_PROJECT_ID,
                "Workspace Authz Session",
                "agent:agent-workspace-authz",
                Some("quota-model"),
                "readonly",
            ),
            "user-without-role",
        )
        .await
        .expect("session");

    let session_policy = adapter
        .load_session_policy_snapshot(&format!("{}-policy", session.summary.id))
        .expect("session policy snapshot");

    assert_eq!(session_policy.capability_decisions.builtin.action, "deny");
    assert!(session_policy.capability_decisions.builtin.hidden);
    assert_eq!(session_policy.capability_decisions.skill.action, "deny");
    assert!(session_policy.capability_decisions.skill.hidden);
    assert_eq!(session_policy.capability_decisions.mcp.action, "deny");
    assert!(session_policy.capability_decisions.mcp.hidden);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_with_configured_mcp_server_stays_async_safe() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config_with_http_mcp(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
        "remote",
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-mcp-runtime",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "MCP Runtime Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Exercise capability planning with MCP config.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&vec!["remote"]).expect("mcp server names"),
                "Agent for MCP runtime projection tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert mcp runtime agent");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-mcp-runtime",
                octopus_core::DEFAULT_PROJECT_ID,
                "MCP Runtime Session",
                "agent:agent-mcp-runtime",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");
    assert!(session
        .provider_state_summary
        .iter()
        .any(|provider| provider.provider_key == "remote"));

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Inspect runtime state", None),
        )
        .await
        .expect("run");
    assert!(run
        .provider_state_summary
        .iter()
        .any(|provider| provider.provider_key == "remote"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn resolve_approval_with_configured_mcp_server_stays_async_safe() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config_with_http_mcp(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
        "remote",
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "agent-mcp-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "MCP Approval Agent",
                Option::<String>::None,
                "Approver",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Exercise approval resume with MCP config.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&vec!["remote"]).expect("mcp server names"),
                "Agent for MCP approval projection tests.",
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
        .expect("upsert mcp approval agent");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-mcp-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "MCP Approval Session",
                "agent:agent-mcp-approval",
                Some("quota-model"),
                "workspace-write",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Run the approval gated action", Some("workspace-write")),
        )
        .await
        .expect("pending approval run");

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    let approval_id = detail
        .pending_approval
        .as_ref()
        .map(|approval| approval.id.clone())
        .expect("pending approval id");

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
    assert_eq!(resolved.approval_state, "approved");
    assert_eq!(resolved.checkpoint.current_iteration_index, 1);
    assert!(resolved
        .provider_state_summary
        .iter()
        .any(|provider| provider.provider_key == "remote"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_does_not_create_deliverable_for_ordinary_assistant_replies() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(12),
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-deliverable-persistence",
                octopus_core::DEFAULT_PROJECT_ID,
                "Deliverable Persistence",
                &agent_actor_ref,
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
            turn_input("Draft the deliverable body", None),
        )
        .await
        .expect("run");

    assert!(run.deliverable_refs.is_empty());
    let runtime_artifact_id = run
        .artifact_refs
        .first()
        .cloned()
        .expect("runtime artifact id");
    assert!(adapter
        .get_deliverable_detail(&runtime_artifact_id)
        .expect("deliverable detail query")
        .is_none());

    let session_detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert!(session_detail
        .messages
        .iter()
        .filter(|message| message.sender_type == "assistant")
        .all(|message| message.deliverable_refs.as_ref().is_none_or(Vec::is_empty)));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_persists_explicit_deliverable_detail_and_versions_across_reload() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the release notes deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("Release Notes Draft".into()),
                preview_kind: "markdown".into(),
                file_name: Some("release-notes.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some(
                    "# Release Notes Draft\n\n- Runtime deliverables now require explicit output."
                        .into(),
                ),
                data_base64: None,
            }],
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-deliverable-persistence",
                octopus_core::DEFAULT_PROJECT_ID,
                "Deliverable Persistence",
                &agent_actor_ref,
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
            turn_input("Draft the deliverable body", None),
        )
        .await
        .expect("run");

    let deliverable_ref = run
        .deliverable_refs
        .first()
        .cloned()
        .expect("generated deliverable ref");
    let artifact_id = deliverable_ref.artifact_id.clone();
    let detail = adapter
        .get_deliverable_detail(&artifact_id)
        .expect("deliverable detail query")
        .expect("deliverable detail");
    assert_eq!(detail.id, artifact_id);
    assert_eq!(detail.project_id, octopus_core::DEFAULT_PROJECT_ID);
    assert_eq!(detail.conversation_id, session.summary.conversation_id);
    assert_eq!(detail.session_id, session.summary.id);
    assert_eq!(detail.run_id, run.id);
    assert_eq!(detail.latest_version, 1);
    assert_eq!(detail.latest_version_ref.version, 1);
    assert_eq!(detail.latest_version_ref.artifact_id, detail.id);
    assert_eq!(detail.latest_version_ref.title, "Release Notes Draft");
    assert_eq!(detail.promotion_state, "not-promoted");

    let versions = adapter
        .list_deliverable_versions(&artifact_id)
        .expect("deliverable versions query");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].artifact_id, artifact_id);
    assert_eq!(versions[0].version, 1);
    assert_eq!(
        versions[0].session_id.as_deref(),
        Some(session.summary.id.as_str())
    );
    assert_eq!(versions[0].run_id.as_deref(), Some(run.id.as_str()));

    let content = adapter
        .get_deliverable_version_content(&artifact_id, 1)
        .expect("deliverable content query")
        .expect("deliverable content");
    assert_eq!(content.artifact_id, artifact_id);
    assert_eq!(content.version, 1);
    assert!(content.editable);
    assert_eq!(content.preview_kind, "markdown");
    assert_eq!(content.file_name.as_deref(), Some("release-notes.md"));
    assert!(content
        .text_content
        .as_deref()
        .is_some_and(|value| value.contains("Runtime deliverables now require explicit output.")));

    let session_detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert!(session_detail
        .messages
        .iter()
        .rev()
        .find(|message| message.sender_type == "assistant")
        .and_then(|message| message.deliverable_refs.clone())
        .is_some_and(|refs| refs
            .iter()
            .any(|reference| reference.artifact_id == artifact_id)));

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the release notes deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("Release Notes Draft".into()),
                preview_kind: "markdown".into(),
                file_name: Some("release-notes.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some(
                    "# Release Notes Draft\n\n- Runtime deliverables now require explicit output."
                        .into(),
                ),
                data_base64: None,
            }],
        }),
    );
    let reloaded_detail = reloaded
        .get_deliverable_detail(&artifact_id)
        .expect("reloaded detail query")
        .expect("reloaded detail");
    assert_eq!(reloaded_detail.latest_version, 1);
    assert_eq!(reloaded_detail.latest_version_ref.version, 1);
    let reloaded_versions = reloaded
        .list_deliverable_versions(&artifact_id)
        .expect("reloaded versions query");
    assert_eq!(reloaded_versions.len(), 1);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn creating_new_deliverable_version_preserves_previous_versions() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the first explicit deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("First deliverable".into()),
                preview_kind: "markdown".into(),
                file_name: Some("first-deliverable.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some("# First deliverable\n\nProduce the first draft".into()),
                data_base64: None,
            }],
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-deliverable-versioning",
                octopus_core::DEFAULT_PROJECT_ID,
                "Deliverable Versioning",
                &agent_actor_ref,
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
            turn_input("Produce the first draft", None),
        )
        .await
        .expect("run");

    let artifact_id = run
        .deliverable_refs
        .first()
        .map(|reference| reference.artifact_id.clone())
        .expect("generated deliverable artifact id");
    let updated = adapter
        .create_deliverable_version(
            &artifact_id,
            CreateDeliverableVersionInput {
                title: Some("Edited deliverable".into()),
                preview_kind: "markdown".into(),
                text_content: Some("Edited deliverable body".into()),
                data_base64: None,
                content_type: Some("text/markdown".into()),
                source_message_id: Some("msg-edited-version".into()),
                parent_version: Some(1),
            },
        )
        .await
        .expect("create deliverable version");

    assert_eq!(updated.id, artifact_id);
    assert_eq!(updated.latest_version, 2);
    assert_eq!(updated.latest_version_ref.version, 2);
    assert_eq!(updated.title, "Edited deliverable");

    let versions = adapter
        .list_deliverable_versions(&artifact_id)
        .expect("versions query");
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version, 2);
    assert_eq!(versions[0].parent_version, Some(1));
    assert_eq!(versions[1].version, 1);

    let version_one = adapter
        .get_deliverable_version_content(&artifact_id, 1)
        .expect("version one content query")
        .expect("version one content");
    assert!(version_one
        .text_content
        .as_deref()
        .is_some_and(|value| value.contains("Produce the first draft")));
    let version_two = adapter
        .get_deliverable_version_content(&artifact_id, 2)
        .expect("version two content query")
        .expect("version two content");
    assert_eq!(
        version_two.text_content.as_deref(),
        Some("Edited deliverable body")
    );

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(12),
        }),
    );
    let reloaded_detail = reloaded
        .get_deliverable_detail(&artifact_id)
        .expect("reloaded detail query")
        .expect("reloaded detail");
    assert_eq!(reloaded_detail.latest_version, 2);
    let reloaded_versions = reloaded
        .list_deliverable_versions(&artifact_id)
        .expect("reloaded versions query");
    assert_eq!(reloaded_versions.len(), 2);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn promoting_deliverable_creates_knowledge_record_and_preserves_lineage() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the reusable guidance deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("Reusable guidance".into()),
                preview_kind: "markdown".into(),
                file_name: Some("reusable-guidance.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some(
                    "# Reusable guidance\n\nKeep the finance review tag on approvals.".into(),
                ),
                data_base64: None,
            }],
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-deliverable-promotion",
                octopus_core::DEFAULT_PROJECT_ID,
                "Deliverable Promotion",
                &agent_actor_ref,
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
            turn_input("Capture the reusable guidance", None),
        )
        .await
        .expect("run");
    let artifact_id = run
        .deliverable_refs
        .first()
        .map(|reference| reference.artifact_id.clone())
        .expect("generated deliverable artifact id");

    let promoted = adapter
        .promote_deliverable(
            &artifact_id,
            PromoteDeliverableInput {
                title: Some("Reusable guidance".into()),
                summary: Some("Keep the finance review tag on approvals.".into()),
                kind: Some("shared".into()),
            },
        )
        .await
        .expect("promote deliverable");
    assert_eq!(promoted.id, artifact_id);
    assert_eq!(promoted.promotion_state, "promoted");
    let promotion_knowledge_id = promoted
        .promotion_knowledge_id
        .clone()
        .expect("promotion knowledge id");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let knowledge_row: (String, String, String, String, String) = connection
        .query_row(
            "SELECT title, summary, kind, source_type, source_ref
             FROM knowledge_records
             WHERE id = ?1",
            [promotion_knowledge_id.clone()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("promoted knowledge row");
    assert_eq!(knowledge_row.0, "Reusable guidance");
    assert_eq!(knowledge_row.1, "Keep the finance review tag on approvals.");
    assert_eq!(knowledge_row.2, "shared");
    assert_eq!(knowledge_row.3, "artifact");
    assert_eq!(knowledge_row.4, artifact_id);

    let versions = adapter
        .list_deliverable_versions(&artifact_id)
        .expect("deliverable versions query");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].run_id.as_deref(), Some(run.id.as_str()));
    assert_eq!(
        versions[0].session_id.as_deref(),
        Some(session.summary.id.as_str())
    );

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    assert!(events.iter().any(|event| {
        event.kind.as_deref() == Some("deliverable.promoted")
            && event.target_kind.as_deref() == Some("deliverable")
            && event.target_ref.as_deref() == Some(artifact_id.as_str())
    }));

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(12),
        }),
    );
    let reloaded_detail = reloaded
        .get_deliverable_detail(&artifact_id)
        .expect("reloaded detail query")
        .expect("reloaded detail");
    assert_eq!(reloaded_detail.promotion_state, "promoted");
    assert_eq!(
        reloaded_detail.promotion_knowledge_id.as_deref(),
        Some(promotion_knowledge_id.as_str())
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn startup_leaves_unsupported_runtime_projection_rows_intact() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let detail_json = json!({
        "summary": {
            "id": "rt-legacy-selected-actor",
            "conversationId": "conv-legacy-selected-actor",
            "projectId": "proj-redesign",
            "title": "Legacy Session",
            "sessionKind": "project",
            "status": "draft",
            "updatedAt": 1,
            "lastMessagePreview": null,
            "configSnapshotId": "cfgsnap-legacy",
            "effectiveConfigHash": "hash-legacy",
            "startedFromScopeSet": ["workspace", "project"],
            "sessionPolicy": {
                "selectedConfiguredModelId": "quota-model",
                "executionPermissionMode": "workspace-write",
                "configSnapshotId": "cfgsnap-legacy",
                "manifestRevision": "asset-manifest/v2",
                "capabilityPolicy": {},
                "memoryPolicy": {},
                "delegationPolicy": {},
                "approvalPreference": {}
            },
            "activeRunId": "run-legacy-selected-actor",
            "subrunCount": 0,
            "memorySummary": {
                "summary": "",
                "durableMemoryCount": 0,
                "selectedMemoryIds": []
            },
            "capabilitySummary": {
                "visibleTools": [],
                "discoverableSkills": []
            }
        },
        "sessionPolicy": {
            "selectedConfiguredModelId": "quota-model",
            "executionPermissionMode": "workspace-write",
            "configSnapshotId": "cfgsnap-legacy",
            "manifestRevision": "asset-manifest/v2",
            "capabilityPolicy": {},
            "memoryPolicy": {},
            "delegationPolicy": {},
            "approvalPreference": {}
        },
        "activeRunId": "run-legacy-selected-actor",
        "subrunCount": 0,
        "memorySummary": {
            "summary": "",
            "durableMemoryCount": 0,
            "selectedMemoryIds": []
        },
        "capabilitySummary": {
            "visibleTools": [],
            "discoverableSkills": []
        },
        "run": {
            "id": "run-legacy-selected-actor",
            "sessionId": "rt-legacy-selected-actor",
            "conversationId": "conv-legacy-selected-actor",
            "status": "draft",
            "currentStep": "ready",
            "startedAt": 1,
            "updatedAt": 1,
            "configuredModelId": null,
            "configuredModelName": null,
            "modelId": null,
            "consumedTokens": null,
            "nextAction": "submit_turn",
            "configSnapshotId": "cfgsnap-legacy",
            "effectiveConfigHash": "hash-legacy",
            "startedFromScopeSet": ["workspace", "project"],
            "checkpoint": {
                "serializedSession": {},
                "currentIterationIndex": 0,
                "usageSummary": {
                    "inputTokens": 0,
                    "outputTokens": 0,
                    "totalTokens": 0
                },
                "pendingApproval": null,
                "compactionMetadata": {}
            },
            "resolvedTarget": null,
            "requestedActorKind": null,
            "requestedActorId": null,
            "resolvedActorKind": null,
            "resolvedActorId": null,
            "resolvedActorLabel": null
        },
        "messages": [],
        "trace": [],
        "pendingApproval": null
    });

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT INTO runtime_session_projections (
                id, conversation_id, project_id, title, status, updated_at,
                config_snapshot_id, effective_config_hash, started_from_scope_set, detail_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                "rt-legacy-selected-actor",
                "conv-legacy-selected-actor",
                "proj-redesign",
                "Legacy Session",
                "draft",
                1_i64,
                "cfgsnap-legacy",
                "hash-legacy",
                serde_json::to_string(&vec!["workspace", "project"]).expect("scope set"),
                serde_json::to_string(&detail_json).expect("detail json"),
            ],
        )
        .expect("insert legacy session projection");

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let sessions = adapter.list_sessions().await.expect("sessions");
    assert!(sessions.is_empty());

    let remaining: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM runtime_session_projections WHERE id = ?1",
            ["rt-legacy-selected-actor"],
            |row| row.get(0),
        )
        .expect("legacy projection count");
    assert_eq!(remaining, 1);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn quota_enabled_models_require_provider_token_usage_metadata() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(64),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver { total_tokens: None }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-missing-usage",
                "",
                "Missing Usage",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");
    let error = adapter
        .submit_turn(&session.summary.id, turn_input("This should fail", None))
        .await
        .expect_err("missing token usage should fail");
    assert!(error
        .to_string()
        .contains("requires provider token usage for quota enforcement"));

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let usage_row: Option<i64> = connection
            .query_row(
                "SELECT used_tokens FROM configured_model_usage_projections WHERE configured_model_id = ?1",
                ["quota-model"],
                |row| row.get(0),
            )
            .optional()
            .expect("usage row");
    assert_eq!(usage_row, None);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn compile_actor_manifest_preserves_personal_pet_metadata() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT INTO agents (
                id, workspace_id, project_id, scope, owner_user_id, asset_role, name, avatar_path,
                personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names,
                description, status, updated_at
            ) VALUES (
                ?1, ?2, NULL, ?3, ?4, ?5, ?6, NULL,
                ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15
            )",
            params![
                "pet-user-owner",
                octopus_core::DEFAULT_WORKSPACE_ID,
                "personal",
                "user-owner",
                "pet",
                "Owner Pet",
                "Personal companion",
                "[]",
                "Stay close to the owner.",
                "[]",
                "[]",
                "[]",
                "Personal pet actor",
                "active",
                1_i64,
            ],
        )
        .expect("insert pet agent");

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(32),
        }),
    );

    let manifest = adapter
        .compile_actor_manifest("agent:pet-user-owner")
        .expect("compile pet actor manifest");
    let actor_manifest::CompiledActorManifest::Agent(agent_manifest) = manifest else {
        panic!("expected agent manifest");
    };

    assert_eq!(agent_manifest.record.scope, "personal");
    assert_eq!(
        agent_manifest.record.owner_user_id.as_deref(),
        Some("user-owner")
    );
    assert_eq!(agent_manifest.record.asset_role, "pet");

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn compile_actor_manifest_rejects_legacy_team_rows_without_actor_refs() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (
                id, workspace_id, project_id, scope, name, avatar_path, personality, tags,
                prompt, builtin_tool_keys, skill_ids, mcp_server_names,
                leader_ref, member_refs,
                description, status, updated_at
            ) VALUES (
                ?1, ?2, NULL, ?3, ?4, NULL, ?5, ?6,
                ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15
            )",
            params![
                "team-legacy-member-ids-only",
                octopus_core::DEFAULT_WORKSPACE_ID,
                "workspace",
                "Legacy Team",
                "Compatibility only",
                "[]",
                "Rely on legacy member ids.",
                "[]",
                "[]",
                "[]",
                "",
                serde_json::to_string(&Vec::<String>::new()).expect("member refs"),
                "Legacy compatibility row.",
                "active",
                1_i64,
            ],
        )
        .expect("insert legacy-only team row");

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(32),
        }),
    );

    let error = adapter
        .compile_actor_manifest("team:team-legacy-member-ids-only")
        .expect_err("legacy-only team rows must fail closed");

    assert!(matches!(error, AppError::InvalidInput(_)));
    assert!(error.to_string().contains("leader_ref"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn builtin_agent_template_refs_create_runtime_sessions() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(64),
    );

    let builtin_agent = infra
        .workspace
        .list_agents()
        .await
        .expect("list agents")
        .into_iter()
        .find(|agent| {
            agent
                .integration_source
                .as_ref()
                .is_some_and(|source| source.kind == "builtin-template")
        })
        .expect("builtin agent");
    let actor_ref = format!("agent:{}", builtin_agent.id);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(32),
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-builtin-agent-template",
                octopus_core::DEFAULT_PROJECT_ID,
                "Builtin Agent Template Session",
                &actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("builtin template actor should create session");

    assert_eq!(session.summary.selected_actor_ref, actor_ref);
    assert_eq!(session.run.actor_ref, format!("agent:{}", builtin_agent.id));
    assert!(session
        .run
        .resolved_actor_label
        .as_deref()
        .is_some_and(|label| label.contains(&builtin_agent.name)));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn builtin_team_template_refs_execute_through_runtime_subruns() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let builtin_team = infra
        .workspace
        .list_teams()
        .await
        .expect("list teams")
        .into_iter()
        .find(|team| {
            team.integration_source
                .as_ref()
                .is_some_and(|source| source.kind == "builtin-template")
                && !team.member_refs.is_empty()
        })
        .expect("builtin team with members");
    let actor_ref = format!("team:{}", builtin_team.id);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-builtin-team-template",
                octopus_core::DEFAULT_PROJECT_ID,
                "Builtin Team Template Session",
                &actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("builtin team template should create session");

    let run = adapter
        .submit_turn(&session.summary.id, turn_input("Review the proposal", None))
        .await
        .expect("builtin team template should create a resolvable team runtime run");

    assert_eq!(run.actor_ref, actor_ref);
    assert_eq!(run.status, "waiting_approval");
    assert_eq!(
        run.pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("team-spawn")
    );

    let approval_id = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail")
        .pending_approval
        .as_ref()
        .map(|approval| approval.id.clone())
        .expect("team spawn approval id");

    let spawn_resolved = adapter
        .resolve_approval(
            &session.summary.id,
            &approval_id,
            ResolveRuntimeApprovalInput {
                decision: "approve".into(),
            },
        )
        .await
        .expect("builtin team spawn approval should resume runtime");

    assert_eq!(spawn_resolved.actor_ref, actor_ref);
    assert!(spawn_resolved.worker_dispatch.is_some());
    assert!(spawn_resolved.workflow_run.is_some());
    assert_eq!(
        spawn_resolved
            .pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("workflow-continuation")
    );

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail after spawn approval");
    assert!(!detail.subruns.is_empty());
    assert!(detail.workflow.is_some());
    assert!(detail
        .subruns
        .iter()
        .all(|subrun| builtin_team.member_refs.contains(&subrun.actor_ref)));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
