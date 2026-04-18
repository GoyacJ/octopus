use super::*;

use std::{fs, path::Path};

use async_trait::async_trait;
pub(crate) use octopus_infra::build_infra_bundle;
pub(crate) use octopus_platform::WorkspaceService;
use rusqlite::params;
use serde_json::json;

pub(crate) const TEST_ANTHROPIC_API_KEY_ENV: &str = "OCTOPUS_TEST_ANTHROPIC_API_KEY";
pub(crate) const TEST_ANTHROPIC_CREDENTIAL_REF: &str = "env:OCTOPUS_TEST_ANTHROPIC_API_KEY";

pub(crate) fn test_root() -> std::path::PathBuf {
    ensure_test_provider_api_keys();
    let root = std::env::temp_dir().join(format!("octopus-runtime-adapter-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).expect("test root");
    root
}

fn ensure_test_provider_api_keys() {
    std::env::set_var(
        TEST_ANTHROPIC_API_KEY_ENV,
        format!("test-{TEST_ANTHROPIC_API_KEY_ENV}"),
    );
}

pub(crate) async fn builtin_agent_actor_ref(infra: &octopus_infra::InfraBundle) -> String {
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

pub(crate) async fn builtin_team_actor_ref(infra: &octopus_infra::InfraBundle) -> String {
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

pub(crate) fn canonical_test_agent_ref(agent_id: &str) -> String {
    format!("agent:{agent_id}")
}

pub(crate) fn canonical_test_member_refs(agent_ids: &[&str]) -> String {
    serde_json::to_string(
        &agent_ids
            .iter()
            .map(|agent_id| canonical_test_agent_ref(agent_id))
            .collect::<Vec<_>>(),
    )
    .expect("member refs")
}

pub(crate) fn legacy_runtime_sessions_dir(root: &Path) -> std::path::PathBuf {
    root.join("runtime").join("sessions")
}

pub(crate) fn write_json(path: &Path, value: serde_json::Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("config dir");
    }
    fs::write(path, serde_json::to_vec_pretty(&value).expect("json")).expect("write config");
}

pub(crate) fn write_workspace_config(path: &Path, total_tokens: Option<u64>) {
    let configured_model = if let Some(total_tokens) = total_tokens {
        json!({
            "configuredModelId": "quota-model",
            "name": "Quota Model",
            "providerId": "anthropic",
            "modelId": "claude-sonnet-4-5",
            "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
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
            "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
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

pub(crate) fn write_workspace_config_with_http_mcp(
    path: &Path,
    total_tokens: Option<u64>,
    server_name: &str,
) {
    let configured_model = if let Some(total_tokens) = total_tokens {
        json!({
            "configuredModelId": "quota-model",
            "name": "Quota Model",
            "providerId": "anthropic",
            "modelId": "claude-sonnet-4-5",
            "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
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
            "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
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

pub(crate) fn write_workspace_config_with_plugins(
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
            "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
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
            "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
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

pub(crate) fn write_external_plugin(
    root: &Path,
    plugin_dir_name: &str,
    plugin_name: &str,
    tool_name: &str,
) {
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

pub(crate) fn session_input(
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

pub(crate) fn home_session_input(
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

pub(crate) fn turn_input(content: &str, permission_mode: Option<&str>) -> SubmitRuntimeTurnInput {
    SubmitRuntimeTurnInput {
        content: content.into(),
        permission_mode: permission_mode.map(str::to_string),
        recall_mode: None,
        ignored_memory_ids: Vec::new(),
        memory_intent: None,
    }
}

pub(crate) fn grant_owner_permissions(infra: &octopus_infra::InfraBundle, user_id: &str) {
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
pub(crate) struct FixedTokenRuntimeModelDriver {
    pub(crate) total_tokens: Option<u32>,
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
pub(crate) struct ExplicitDeliverableRuntimeModelDriver {
    pub(crate) total_tokens: Option<u32>,
    pub(crate) content: String,
    pub(crate) deliverables: Vec<ModelExecutionDeliverable>,
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
pub(crate) struct InspectingPromptRuntimeModelDriver {
    request_policies: Mutex<Vec<octopus_core::ResolvedRequestPolicy>>,
}

impl InspectingPromptRuntimeModelDriver {
    pub(crate) fn last_request_policy(&self) -> Option<octopus_core::ResolvedRequestPolicy> {
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
pub(crate) struct FailingRuntimeSecretStore;

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
pub(crate) struct ScriptedConversationRuntimeModelDriver {
    responses: Mutex<Vec<Vec<runtime::AssistantEvent>>>,
    requests: Mutex<Vec<RuntimeConversationRequest>>,
}

impl ScriptedConversationRuntimeModelDriver {
    pub(crate) fn new(responses: Vec<Vec<runtime::AssistantEvent>>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().rev().collect()),
            requests: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn request_count(&self) -> usize {
        self.requests.lock().expect("requests mutex").len()
    }

    pub(crate) fn requests(&self) -> Vec<RuntimeConversationRequest> {
        self.requests.lock().expect("requests mutex").clone()
    }
}

pub(crate) fn last_user_text(request: &RuntimeConversationRequest) -> Option<&str> {
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
