use super::adapter_test_support::*;
use super::*;

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
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

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
async fn runtime_config_validation_rejects_non_positive_budget_policy_total() {
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
                        "budgetPolicy": {
                            "totalBudgetTokens": 0
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
        .any(|error| error.contains("budgetPolicy.totalBudgetTokens")));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn runtime_config_validation_rejects_unsupported_budget_accounting_modes() {
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
                        "budgetPolicy": {
                            "totalBudgetTokens": 64,
                            "accountingMode": "estimated"
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
    assert!(validation.errors.iter().any(|error| {
        error.contains("budgetPolicy.accountingMode") && error.contains("provider_reported")
    }));

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
