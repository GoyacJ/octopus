use super::adapter_test_support::*;
use super::*;

fn write_workspace_config_with_generation_model(path: &std::path::Path) {
    write_json(
        path,
        json!({
            "configuredModels": {
                "generation-only-model": {
                    "configuredModelId": "generation-only-model",
                    "name": "Generation Only Model",
                    "providerId": "google",
                    "modelId": "gemini-2.5-flash",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
    );
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
async fn create_session_rejects_single_shot_generation_model_selection() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config_with_generation_model(
        &infra.paths.runtime_config_dir.join("workspace.json"),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let error = adapter
        .create_session(
            session_input(
                "conv-prompt-only-session",
                "",
                "Prompt Only Session",
                &agent_actor_ref,
                Some("generation-only-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect_err("single-shot generation model should be rejected for runtime sessions");
    assert!(error
        .to_string()
        .contains("does not expose a runtime-supported surface"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn resolve_submit_execution_rejects_single_shot_generation_model_selection() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config_with_generation_model(
        &infra.paths.runtime_config_dir.join("workspace.json"),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let snapshot = adapter
        .current_config_snapshot(None, Some("user-owner"))
        .expect("config snapshot");
    adapter
        .persist_config_snapshot(&snapshot)
        .expect("persist config snapshot");
    let manifest = adapter
        .compile_actor_manifest(&agent_actor_ref)
        .expect("actor manifest");
    let session_policy = adapter
        .compile_session_policy(
            "rt-prompt-only-session",
            &manifest,
            &snapshot,
            Some("generation-only-model"),
            "readonly",
            "user-owner",
            None,
            None,
        )
        .await
        .expect("session policy");

    let error = adapter
        .resolve_submit_execution(&session_policy, &turn_input("This should fail", None))
        .expect_err("single-shot generation model should be rejected on submit");
    assert!(error
        .to_string()
        .contains("does not expose a runtime-supported surface"));

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
        .contains("requires provider token usage for budget enforcement"));

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let projection_row: Option<(i64, i64)> = connection
        .query_row(
            "SELECT settled_tokens, active_reserved_tokens
             FROM configured_model_budget_projections
             WHERE configured_model_id = ?1",
            ["quota-model"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .expect("projection row");
    assert_eq!(projection_row, Some((0, 0)));
    let reservation_status: String = connection
        .query_row(
            "SELECT status
             FROM configured_model_budget_reservations
             WHERE configured_model_id = ?1
             ORDER BY created_at DESC
             LIMIT 1",
            ["quota-model"],
            |row| row.get(0),
        )
        .expect("reservation status");
    assert_eq!(reservation_status, "released");

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn unsupported_budget_accounting_modes_fail_before_runtime_execution_starts() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "configuredModels": {
                "quota-model": {
                    "configuredModelId": "quota-model",
                    "name": "Quota Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
                    "budgetPolicy": {
                        "totalBudgetTokens": 64,
                        "accountingMode": "estimated"
                    },
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

    let session = adapter
        .create_session(
            session_input(
                "conv-unsupported-accounting-mode",
                "",
                "Unsupported Accounting Mode",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");
    let error = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("This should fail early", None),
        )
        .await
        .expect_err("unsupported accounting mode should fail before execution");
    assert!(error
        .to_string()
        .contains("budgetPolicy.accountingMode `estimated` is not supported"));
    assert_eq!(driver.last_request_policy(), None);

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
