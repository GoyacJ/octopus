use super::*;

use std::{fs, path::Path};

use async_trait::async_trait;
use octopus_core::CreateRuntimeSessionInput;
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
            CreateRuntimeSessionInput {
                conversation_id: "conv-1".into(),
                project_id: project_id.into(),
                title: "Runtime precedence".into(),
                session_kind: None,
            },
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
            CreateRuntimeSessionInput {
                conversation_id: "conv-quota".into(),
                project_id: "".into(),
                title: "Quota Session".into(),
                session_kind: None,
            },
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(
            &session.summary.id,
            SubmitRuntimeTurnInput {
                content: "Count tokens".into(),
                model_id: None,
                configured_model_id: Some("quota-model".into()),
                permission_mode: "readonly".into(),
                actor_kind: None,
                actor_id: None,
            },
        )
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
            CreateRuntimeSessionInput {
                conversation_id: "conv-restart".into(),
                project_id: "".into(),
                title: "Restart Session".into(),
                session_kind: None,
            },
            "user-owner",
        )
        .await
        .expect("session");
    adapter
        .submit_turn(
            &session.summary.id,
            SubmitRuntimeTurnInput {
                content: "Persist usage".into(),
                model_id: None,
                configured_model_id: Some("quota-model".into()),
                permission_mode: "readonly".into(),
                actor_kind: None,
                actor_id: None,
            },
        )
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
            CreateRuntimeSessionInput {
                conversation_id: "conv-first".into(),
                project_id: "".into(),
                title: "First Session".into(),
                session_kind: None,
            },
            "user-owner",
        )
        .await
        .expect("first session");
    let first_run = adapter
        .submit_turn(
            &first_session.summary.id,
            SubmitRuntimeTurnInput {
                content: "Use the full quota".into(),
                model_id: None,
                configured_model_id: Some("quota-model".into()),
                permission_mode: "readonly".into(),
                actor_kind: None,
                actor_id: None,
            },
        )
        .await
        .expect("first run");
    assert_eq!(first_run.consumed_tokens, Some(32));

    let second_session = adapter
        .create_session(
            CreateRuntimeSessionInput {
                conversation_id: "conv-second".into(),
                project_id: "".into(),
                title: "Second Session".into(),
                session_kind: None,
            },
            "user-owner",
        )
        .await
        .expect("second session");
    let error = adapter
        .submit_turn(
            &second_session.summary.id,
            SubmitRuntimeTurnInput {
                content: "This should be blocked".into(),
                model_id: None,
                configured_model_id: Some("quota-model".into()),
                permission_mode: "readonly".into(),
                actor_kind: None,
                actor_id: None,
            },
        )
        .await
        .expect_err("quota exhaustion should block new requests");
    assert!(error
        .to_string()
        .contains("has reached its total token limit"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_injects_selected_agent_prompt_into_execution() {
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
            CreateRuntimeSessionInput {
                conversation_id: "conv-agent-actor".into(),
                project_id: octopus_core::DEFAULT_PROJECT_ID.into(),
                title: "Agent Actor Session".into(),
                session_kind: None,
            },
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(
            &session.summary.id,
            SubmitRuntimeTurnInput {
                content: "Design the rollout".into(),
                model_id: None,
                configured_model_id: Some("quota-model".into()),
                permission_mode: "readonly".into(),
                actor_kind: Some("agent".into()),
                actor_id: Some("agent-project-delivery".into()),
            },
        )
        .await
        .expect("run");

    assert_eq!(run.resolved_actor_kind.as_deref(), Some("agent"));
    assert_eq!(
        run.resolved_actor_id.as_deref(),
        Some("agent-project-delivery")
    );

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
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
async fn resolve_approval_reuses_selected_team_prompt_for_execution() {
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
            CreateRuntimeSessionInput {
                conversation_id: "conv-team-actor".into(),
                project_id: octopus_core::DEFAULT_PROJECT_ID.into(),
                title: "Team Actor Session".into(),
                session_kind: None,
            },
            "user-owner",
        )
        .await
        .expect("session");

    let pending = adapter
        .submit_turn(
            &session.summary.id,
            SubmitRuntimeTurnInput {
                content: "Review the proposal".into(),
                model_id: None,
                configured_model_id: Some("quota-model".into()),
                permission_mode: "workspace-write".into(),
                actor_kind: Some("team".into()),
                actor_id: Some("team-workspace-core".into()),
            },
        )
        .await
        .expect("pending run");
    assert_eq!(pending.status, "waiting_approval");

    let approval_id = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail")
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
        .expect("approved run");
    assert_eq!(resolved.status, "completed");

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail after approval");
    let assistant_message = detail
        .messages
        .iter()
        .rev()
        .find(|message| message.sender_type == "assistant")
        .expect("assistant message");
    assert!(assistant_message
        .content
        .contains("You are the team `Workspace Core` operating as a single execution actor."));
    assert!(assistant_message
        .content
        .contains("Team personality: Cross-functional design review board"));
    assert!(assistant_message
        .content
        .contains("Team instructions: Debate options, then return a single aligned answer."));
    assert!(assistant_message
        .content
        .contains("Leader agent id: agent-orchestrator"));
    assert!(assistant_message
        .content
        .contains("Member agent ids: agent-orchestrator, agent-project-delivery"));

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
            CreateRuntimeSessionInput {
                conversation_id: "conv-missing-usage".into(),
                project_id: "".into(),
                title: "Missing Usage".into(),
                session_kind: None,
            },
            "user-owner",
        )
        .await
        .expect("session");
    let error = adapter
        .submit_turn(
            &session.summary.id,
            SubmitRuntimeTurnInput {
                content: "This should fail".into(),
                model_id: None,
                configured_model_id: Some("quota-model".into()),
                permission_mode: "readonly".into(),
                actor_kind: None,
                actor_id: None,
            },
        )
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
