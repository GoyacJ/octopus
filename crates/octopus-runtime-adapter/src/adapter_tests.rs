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

#[tokio::test]
async fn runtime_config_resolution_respects_user_workspace_project_precedence() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
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
                "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, leader_agent_id, member_agent_ids, description, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
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

    let workflow_events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events")
        .into_iter()
        .filter_map(|event| event.kind)
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

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        Arc::new(MockRuntimeModelExecutor),
    );
    let reloaded_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("reloaded session detail");
    assert_eq!(reloaded_detail.subrun_count, detail.subrun_count);
    assert_eq!(reloaded_detail.workflow, detail.workflow);
    assert_eq!(reloaded_detail.pending_mailbox, detail.pending_mailbox);
    assert_eq!(reloaded_detail.background_run, detail.background_run);
    assert_eq!(reloaded_detail.subruns.len(), detail.subruns.len());
    assert_eq!(reloaded_detail.handoffs.len(), detail.handoffs.len());

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
    assert_eq!(run.freshness_summary.as_ref().map(|value| value.fresh_count), Some(1));
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
    assert!(events.iter().any(|event| event.event_type == "memory.selected"));
    assert!(events.iter().any(|event| event.event_type == "memory.proposed"));

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
        proposal.review.as_ref().and_then(|review| review.note.as_deref()),
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
        adapter.runtime_memory_body_path(&proposal.memory_id).exists(),
        "memory body should be persisted under data/knowledge"
    );

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    assert!(events.iter().any(|event| event.event_type == "memory.approved"));

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
async fn submit_turn_requiring_approval_persists_real_mediation_and_outcome() {
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

    let pending = run.pending_mediation.clone().expect("pending mediation");
    assert_eq!(pending.tool_name.as_deref(), Some("runtime.turn"));
    assert_eq!(pending.mediation_kind, "approval");
    assert!(run.capability_state_ref.is_some());
    let outcome = run
        .last_execution_outcome
        .clone()
        .expect("last execution outcome");
    assert_eq!(outcome.tool_name.as_deref(), Some("runtime.turn"));
    assert_eq!(outcome.outcome, "require_approval");
    assert!(outcome.requires_approval);
    assert_eq!(
        run.checkpoint
            .pending_mediation
            .as_ref()
            .and_then(|value| value.tool_name.as_deref()),
        Some("runtime.turn")
    );
    assert_eq!(
        run.checkpoint
            .last_execution_outcome
            .as_ref()
            .map(|value| value.outcome.as_str()),
        Some("require_approval")
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
    assert_eq!(persisted_pending.tool_name.as_deref(), Some("runtime.turn"));
    assert_eq!(
        persisted.1,
        run.capability_state_ref.clone().expect("state ref")
    );
    let persisted_outcome: RuntimeCapabilityExecutionOutcome =
        serde_json::from_str(&persisted.2).expect("outcome json");
    assert_eq!(persisted_outcome.outcome, "require_approval");
    let persisted_plan: RuntimeCapabilityPlanSummary =
        serde_json::from_str(&persisted.3).expect("plan json");
    assert_eq!(persisted_plan.visible_tools, vec!["bash".to_string()]);
    assert_eq!(persisted.4, 0);

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
    assert!(resolved
        .provider_state_summary
        .iter()
        .any(|provider| provider.provider_key == "remote"));

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
