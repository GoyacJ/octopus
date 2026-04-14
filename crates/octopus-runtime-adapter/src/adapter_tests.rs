use super::*;

use std::{fs, path::Path};

use async_trait::async_trait;
use octopus_core::CreateRuntimeSessionInput;
use octopus_core::{
    ResolveRuntimeMemoryProposalInput, RuntimeCapabilityExecutionOutcome,
    RuntimePendingMediationSummary,
};
use octopus_infra::build_infra_bundle;
use octopus_platform::{
    ModelRegistryService, RuntimeConfigService, RuntimeExecutionService, RuntimeSessionService,
};
use rusqlite::params;
use serde_json::json;

fn test_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("octopus-runtime-adapter-{}", Uuid::new_v4()));
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
        project_id: project_id.into(),
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

fn grant_owner_permissions(infra: &octopus_infra::InfraBundle, user_id: &str) {
    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO role_bindings (id, role_id, subject_type, subject_id, effect)
             VALUES (?1, 'owner', 'user', ?2, 'allow')",
            params![format!("binding-user-{user_id}-owner"), user_id],
        )
        .expect("grant owner permissions");
}

fn persist_memory_record(
    adapter: &RuntimeAdapter,
    project_id: &str,
    memory_id: &str,
    kind: &str,
    scope: &str,
    summary: &str,
) {
    adapter
        .persist_runtime_memory_record(
            &memory_runtime::PersistedRuntimeMemoryRecord {
                memory_id: memory_id.into(),
                project_id: Some(project_id.into()),
                owner_ref: Some(format!("project:{project_id}")),
                source_run_id: Some("seed-run".into()),
                kind: kind.into(),
                scope: scope.into(),
                title: format!("{kind} memory"),
                summary: summary.into(),
                freshness_state: "fresh".into(),
                last_validated_at: Some(1),
                proposal_state: "approved".into(),
                storage_path: None,
                content_hash: None,
                updated_at: 1,
            },
            &json!({ "summary": summary }),
        )
        .expect("persist runtime memory");
}

#[derive(Debug, Clone)]
struct FixedTokenRuntimeModelExecutor {
    total_tokens: Option<u32>,
}

#[async_trait]
impl RuntimeModelExecutor for FixedTokenRuntimeModelExecutor {
    async fn execute_turn(
        &self,
        _target: &ResolvedExecutionTarget,
        input: &str,
        system_prompt: Option<&str>,
    ) -> Result<ExecutionResponse, AppError> {
        let prompt_prefix = system_prompt
            .map(|value| format!(" [{value}]"))
            .unwrap_or_default();
        Ok(ExecutionResponse {
            content: format!("fixed token response{prompt_prefix} -> {input}"),
            request_id: Some("fixed-token-request".into()),
            total_tokens: self.total_tokens,
        })
    }
}

#[derive(Debug)]
struct ScriptedConversationRuntimeModelExecutor {
    responses: Mutex<Vec<Vec<runtime::AssistantEvent>>>,
    requests: Mutex<Vec<RuntimeConversationRequest>>,
}

impl ScriptedConversationRuntimeModelExecutor {
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

#[async_trait]
impl RuntimeModelExecutor for ScriptedConversationRuntimeModelExecutor {
    async fn execute_turn(
        &self,
        _target: &ResolvedExecutionTarget,
        _input: &str,
        _system_prompt: Option<&str>,
    ) -> Result<ExecutionResponse, AppError> {
        Err(AppError::runtime(
            "scripted conversation executor should use execute_conversation",
        ))
    }

    async fn execute_conversation(
        &self,
        _target: &ResolvedExecutionTarget,
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
        Arc::new(MockRuntimeModelExecutor),
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
async fn runtime_session_snapshot_uses_scope_order_from_user_to_project() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelExecutor),
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
                "agent:agent-project-delivery",
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
    assert_eq!(
        detail.summary.selected_actor_ref,
        "agent:agent-project-delivery"
    );
    assert_eq!(
        detail.summary.manifest_revision,
        octopus_core::ASSET_MANIFEST_REVISION_V2
    );
    assert_eq!(
        detail.summary.session_policy.execution_permission_mode,
        octopus_core::RUNTIME_PERMISSION_READ_ONLY
    );
    assert_eq!(
        detail.summary.session_policy.selected_configured_model_id,
        ""
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
        Arc::new(MockRuntimeModelExecutor),
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
        Arc::new(MockRuntimeModelExecutor),
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
async fn runtime_config_validation_warns_for_unknown_and_deprecated_top_level_keys() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelExecutor),
    );

    let validation = adapter
        .validate_config(RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: json!({
                "telemetry": true,
                "allowedTools": ["read_file"]
            }),
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
        Arc::new(MockRuntimeModelExecutor),
    );

    let validation = adapter
        .validate_config(RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: json!({
                "trustedRoots": "not-an-array"
            }),
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
        Arc::new(MockRuntimeModelExecutor),
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
async fn submit_turn_updates_configured_model_token_usage_and_catalog_snapshot() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelExecutor {
            total_tokens: Some(32),
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-quota",
                "",
                "Quota Session",
                "agent:agent-orchestrator",
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
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelExecutor {
            total_tokens: Some(24),
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-restart",
                "",
                "Restart Session",
                "agent:agent-orchestrator",
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
        Arc::new(FixedTokenRuntimeModelExecutor {
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
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(32),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelExecutor {
            total_tokens: Some(32),
        }),
    );

    let first_session = adapter
        .create_session(
            session_input(
                "conv-first",
                "",
                "First Session",
                "agent:agent-orchestrator",
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
                "agent:agent-orchestrator",
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
        Arc::new(MockRuntimeModelExecutor),
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
                "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_agent_id, member_agent_ids, description, status, updated_at)
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
                    "agent-orchestrator",
                    serde_json::to_string(&vec!["agent-orchestrator", "agent-project-delivery"]).expect("member ids"),
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
        Arc::new(MockRuntimeModelExecutor),
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
        .runtime_sessions_dir
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

    let mailbox = detail.pending_mailbox.as_ref().expect("mailbox summary");
    assert_eq!(mailbox.channel, "team-mailbox");
    assert!(mailbox.total_messages >= 2);

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
    assert!(background_projection
        .0
        .as_deref()
        .is_some_and(|path| root.join(path).exists()));
    assert!(background_projection
        .1
        .as_deref()
        .is_some_and(|hash| hash.starts_with("sha256-")));

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
    assert_eq!(first_subrun_event.parent_run_id.as_deref(), Some(run.id.as_str()));
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
        Arc::new(MockRuntimeModelExecutor),
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
        .is_some_and(|mailbox| mailbox.pending_count >= 1));
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
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_agent_id, member_agent_ids, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                "team-workspace-core",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Workspace Core",
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
                "agent-orchestrator",
                serde_json::to_string(&vec!["agent-orchestrator"]).expect("member ids"),
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
        Arc::new(MockRuntimeModelExecutor),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-team-reload-artifacts",
                octopus_core::DEFAULT_PROJECT_ID,
                "Team Artifact Reload",
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
        .runtime_sessions_dir
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
        Arc::new(MockRuntimeModelExecutor),
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
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_agent_id, member_agent_ids, description, status, updated_at)
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
                "agent-team-policy-leader",
                serde_json::to_string(&vec![
                    "agent-team-policy-leader",
                    "agent-team-policy-worker"
                ])
                .expect("member ids"),
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
        Arc::new(MockRuntimeModelExecutor),
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
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        None,
    );
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
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_agent_id, member_agent_ids, description, status, updated_at)
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
                "agent-team-spawn-leader",
                serde_json::to_string(&vec![
                    "agent-team-spawn-leader",
                    "agent-team-spawn-worker"
                ])
                .expect("member ids"),
                "Team for team spawn approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team spawn approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelExecutor::new(vec![vec![
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
    assert_eq!(executor.request_count(), 1);

    let resolved_detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("resolved session detail");
    assert!(resolved_detail.pending_approval.is_none());
    assert!(resolved_detail.subruns.len() >= 2);
    assert!(resolved_detail
        .subruns
        .iter()
        .any(|subrun| subrun.actor_ref == "agent:agent-team-spawn-worker"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn workflow_continuation_approval_blocks_workflow_projection_until_resolved() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        None,
    );
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
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_agent_id, member_agent_ids, description, status, updated_at)
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
                "agent-workflow-approval-leader",
                serde_json::to_string(&vec![
                    "agent-workflow-approval-leader",
                    "agent-workflow-approval-worker"
                ])
                .expect("member ids"),
                "Team for workflow continuation approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workflow approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelExecutor::new(vec![vec![
        runtime::AssistantEvent::TextDelta("Workflow plan ready.".into()),
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
        .submit_turn(&session.summary.id, turn_input("Continue the workflow", None))
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
    assert!(run.workflow_run.is_none());
    assert!(run.background_state.is_none());

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert!(detail.subrun_count >= 1);
    assert!(!detail.subruns.is_empty());
    assert!(detail.pending_mailbox.is_some());
    assert!(detail.workflow.is_none());
    assert!(detail.background_run.is_none());
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
    assert_eq!(executor.request_count(), 1);
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

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_spawn_approval_chains_into_workflow_continuation_approval_when_required() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        None,
    );
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
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_agent_id, member_agent_ids, description, status, updated_at)
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
                "agent-team-spawn-workflow-leader",
                serde_json::to_string(&vec![
                    "agent-team-spawn-workflow-leader",
                    "agent-team-spawn-workflow-worker"
                ])
                .expect("member ids"),
                "Team for chained workflow approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert chained approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelExecutor::new(vec![vec![
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
    assert!(spawn_resolved.workflow_run.is_none());
    assert_eq!(executor.request_count(), 1);

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail after spawn approval");
    assert!(!detail.subruns.is_empty());
    assert!(detail.workflow.is_none());
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
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        None,
    );
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
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, delegation_policy_json, approval_preference_json, leader_agent_id, member_agent_ids, worker_concurrency_limit, description, status, updated_at)
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
                "agent-team-policy-leader",
                serde_json::to_string(&vec![
                    "agent-team-policy-leader",
                    "agent-team-policy-worker"
                ])
                .expect("member ids"),
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
        Arc::new(MockRuntimeModelExecutor),
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

    let subrun_state_dir = infra.paths.runtime_sessions_dir.join("subruns");
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
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelExecutor {
            total_tokens: Some(16),
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-phase-two-shape",
                octopus_core::DEFAULT_PROJECT_ID,
                "Phase 2 Contract Shape",
                "agent:agent-project-delivery",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    assert_eq!(session.selected_actor_ref, "agent:agent-project-delivery");
    assert_eq!(
        session.manifest_revision,
        octopus_core::ASSET_MANIFEST_REVISION_V2
    );
    assert_eq!(session.active_run_id, session.run.id);
    assert_eq!(session.subrun_count, 0);
    assert_eq!(
        session.session_policy.selected_actor_ref,
        "agent:agent-project-delivery"
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
    assert_eq!(session_projection.0, "agent:agent-project-delivery");
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
    assert_eq!(run_projection.2, "agent:agent-project-delivery");
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
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelExecutor {
            total_tokens: Some(8),
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-phase-two-events",
                octopus_core::DEFAULT_PROJECT_ID,
                "Phase 2 Events",
                "agent:agent-project-delivery",
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
        "subrun.completed",
        "subrun.failed",
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
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelExecutor {
            total_tokens: Some(12),
        }),
    );
    persist_memory_record(
        &adapter,
        octopus_core::DEFAULT_PROJECT_ID,
        "mem-user-preference",
        "user",
        "user",
        "Remember the user's approval preference.",
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-events",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Events",
                "agent:agent-project-delivery",
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
async fn submit_turn_rejects_memory_pollution_candidates() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelExecutor {
            total_tokens: Some(12),
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-pollution",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Pollution",
                "agent:agent-project-delivery",
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
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelExecutor {
            total_tokens: Some(12),
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-resolution",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Resolution",
                "agent:agent-project-delivery",
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

    let proposal = resolved
        .pending_memory_proposal
        .as_ref()
        .expect("resolved proposal");
    assert_eq!(proposal.proposal_state, "approved");
    assert_eq!(
        proposal
            .review
            .as_ref()
            .and_then(|review| review.note.as_deref()),
        Some("validated")
    );

    let records = adapter
        .load_runtime_memory_records(octopus_core::DEFAULT_PROJECT_ID)
        .expect("memory records");
    assert!(records.iter().any(|record| {
        record.memory_id == proposal.memory_id
            && record.proposal_state == "approved"
            && record.freshness_state == "fresh"
    }));
    assert!(
        adapter
            .runtime_memory_body_path(&proposal.memory_id)
            .exists(),
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
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelExecutor {
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
                "agent:agent-project-delivery",
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
    assert_eq!(
        resolved
            .pending_memory_proposal
            .as_ref()
            .map(|proposal| proposal.proposal_state.as_str()),
        Some("revalidated")
    );

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
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelExecutor {
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
                "agent:agent-project-delivery",
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
        Arc::new(MockRuntimeModelExecutor),
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
        Arc::new(MockRuntimeModelExecutor),
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

    let executor = Arc::new(ScriptedConversationRuntimeModelExecutor::new(vec![
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

    let serialized_messages = run
        .checkpoint
        .serialized_session
        .get("session")
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
                && block.get("toolName").and_then(serde_json::Value::as_str)
                    == Some("plugin_echo")
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
        event.capability_plan_summary.as_ref().is_some_and(|summary| {
            summary.visible_tools.contains(&"plugin_echo".to_string())
        })
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

    let executor = Arc::new(ScriptedConversationRuntimeModelExecutor::new(vec![
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
        event.capability_plan_summary.as_ref().is_some_and(|summary| {
            summary.visible_tools.contains(&"plugin_echo".to_string())
        })
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
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
        Arc::new(MockRuntimeModelExecutor),
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
        pending.input.as_ref().and_then(|value| value.get("content")),
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
        run.checkpoint.input.as_ref().and_then(|value| value.get("content")),
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
    assert_eq!(persisted_pending.dispatch_kind.as_deref(), Some("model_execution"));
    assert_eq!(
        persisted_pending.concurrency_policy.as_deref(),
        Some("serialized")
    );
    assert_eq!(
        persisted_pending.input.as_ref().and_then(|value| value.get("content")),
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

    let executor = Arc::new(ScriptedConversationRuntimeModelExecutor::new(vec![vec![
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

    let executor = Arc::new(ScriptedConversationRuntimeModelExecutor::new(vec![vec![
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
        Arc::new(MockRuntimeModelExecutor),
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
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
        Arc::new(MockRuntimeModelExecutor),
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
async fn submit_turn_executes_runtime_tool_loop_on_main_path() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

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

    let executor = Arc::new(ScriptedConversationRuntimeModelExecutor::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Inspecting the note.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-read-note".into(),
                name: "read_file".into(),
                input: serde_json::json!({
                    "path": note_path.display().to_string()
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
            runtime::AssistantEvent::TextDelta("Summary: runtime loop content.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 8,
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
        .any(|message| matches!(message.role, runtime::MessageRole::User)));
    assert!(requests[1]
        .messages
        .iter()
        .any(|message| matches!(message.role, runtime::MessageRole::Tool)));

    let serialized_session = run
        .checkpoint
        .serialized_session
        .get("session")
        .expect("serialized runtime session");
    let serialized_messages = serialized_session
        .get("messages")
        .and_then(serde_json::Value::as_array)
        .expect("serialized session messages");
    assert_eq!(serialized_messages.len(), 4);
    assert_eq!(
        run.checkpoint
            .serialized_session
            .get("content")
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
    assert_eq!(tool_started.actor_ref.as_deref(), Some(run.actor_ref.as_str()));
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
    grant_owner_permissions(&infra, "user-owner");

    let note_path = root.join("approval-loop-note.txt");
    fs::write(&note_path, "approval loop content\n").expect("seed note");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert runtime approval loop agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelExecutor::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Inspecting the approved note.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-approved-read-note".into(),
                name: "read_file".into(),
                input: serde_json::json!({
                    "path": note_path.display().to_string()
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
            runtime::AssistantEvent::TextDelta("Approved summary: approval loop content.".into()),
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
        .any(|message| matches!(message.role, runtime::MessageRole::Tool)));

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
    grant_owner_permissions(&infra, "user-owner");

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
                serde_json::to_string(&vec!["write_file"]).expect("builtin tool keys"),
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

    let executor = Arc::new(ScriptedConversationRuntimeModelExecutor::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Writing the requested file.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-write-approved-note".into(),
                name: "write_file".into(),
                input: serde_json::json!({
                    "path": output_path_string,
                    "content": "capability approval content\n"
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
            runtime::AssistantEvent::TextDelta("Completed the approved file write.".into()),
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
                "conv-capability-call-approval-loop",
                octopus_core::DEFAULT_PROJECT_ID,
                "Capability Call Approval Loop Session",
                "agent:agent-capability-call-approval-loop",
                Some("quota-model"),
                octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
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
        Some("write_file")
    );
    assert_eq!(
        pending_run
            .checkpoint
            .serialized_session
            .get("pendingToolUses")
            .and_then(serde_json::Value::as_array)
            .map(|items| items.len()),
        Some(1)
    );
    assert_eq!(
        pending_run
            .checkpoint
            .serialized_session
            .pointer("/pendingToolUses/0/toolUseId")
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
    assert_eq!(
        fs::read_to_string(&output_path).expect("written file"),
        "capability approval content\n"
    );
    assert_eq!(
        resolved
            .checkpoint
            .serialized_session
            .get("pendingToolUses")
            .and_then(serde_json::Value::as_array)
            .map(|items| items.len()),
        Some(0)
    );

    let requests = executor.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests[1]
        .messages
        .iter()
        .any(|message| matches!(message.role, runtime::MessageRole::Tool)));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn loads_legacy_runtime_projection_missing_selected_actor_ref() {
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
            "runKind": "primary",
            "parentRunId": null,
            "actorRef": "agent:agent-architect",
            "delegatedByToolCallId": null,
            "approvalState": "not-required",
            "usageSummary": {
                "inputTokens": 0,
                "outputTokens": 0,
                "totalTokens": 0
            },
            "artifactRefs": [],
            "traceContext": {
                "sessionId": "rt-legacy-selected-actor",
                "traceId": "trace-legacy-selected-actor",
                "turnId": "turn-legacy-selected-actor",
                "parentRunId": null
            },
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
        Arc::new(MockRuntimeModelExecutor),
    );

    let sessions = adapter.list_sessions().await.expect("sessions");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, "rt-legacy-selected-actor");
    assert_eq!(sessions[0].selected_actor_ref, "agent:agent-architect");

    let detail = adapter
        .get_session("rt-legacy-selected-actor")
        .await
        .expect("legacy session detail");
    assert_eq!(detail.selected_actor_ref, "agent:agent-architect");
    assert_eq!(detail.summary.selected_actor_ref, "agent:agent-architect");

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn quota_enabled_models_require_provider_token_usage_metadata() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(64),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelExecutor { total_tokens: None }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-missing-usage",
                "",
                "Missing Usage",
                "agent:agent-orchestrator",
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
