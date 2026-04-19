use super::adapter_test_support::*;
use super::*;
use async_trait::async_trait;

#[derive(Debug, Clone)]
struct FailingRuntimeModelDriver;

#[async_trait]
impl RuntimeModelDriver for FailingRuntimeModelDriver {
    async fn execute_prompt(
        &self,
        _target: &ResolvedExecutionTarget,
        _request_policy: &octopus_core::ResolvedRequestPolicy,
        _input: &str,
        _system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError> {
        Err(AppError::runtime("simulated generation failure"))
    }

    async fn execute_conversation_execution(
        &self,
        _target: &ResolvedExecutionTarget,
        _request_policy: &octopus_core::ResolvedRequestPolicy,
        _request: &RuntimeConversationRequest,
    ) -> Result<RuntimeConversationExecution, AppError> {
        Err(AppError::runtime("simulated conversation failure"))
    }
}

#[tokio::test]
async fn submit_turn_updates_configured_model_budget_usage_and_catalog_snapshot() {
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
                        "totalBudgetTokens": 100,
                        "reservationStrategy": "fixed"
                    },
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
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
            .budget_policy
            .as_ref()
            .and_then(|policy| policy.total_budget_tokens),
        Some(100)
    );
    assert_eq!(configured_model.token_usage.used_tokens, 32);
    assert_eq!(configured_model.token_usage.remaining_tokens, Some(68));
    assert!(!configured_model.token_usage.exhausted);

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let (settled_tokens, active_reserved_tokens): (i64, i64) = connection
        .query_row(
            "SELECT settled_tokens, active_reserved_tokens
                 FROM configured_model_budget_projections
                 WHERE configured_model_id = ?1",
            ["quota-model"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("budget projection");
    assert_eq!(settled_tokens, 32);
    assert_eq!(active_reserved_tokens, 0);
    let reservation_status: String = connection
        .query_row(
            "SELECT status FROM configured_model_budget_reservations WHERE id = ?1",
            [&run.id],
            |row| row.get(0),
        )
        .expect("reservation status");
    assert_eq!(reservation_status, "settled");
    let settlement_tokens: i64 = connection
            .query_row(
                "SELECT settled_tokens FROM configured_model_budget_settlements WHERE reservation_id = ?1",
                [&run.id],
                |row| row.get(0),
            )
            .expect("settlement tokens");
    assert_eq!(settlement_tokens, 32);
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
async fn run_generation_releases_fixed_budget_reservation_when_execution_fails() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    std::env::set_var("OPENAI_API_KEY", "test-openai-api-key");
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "configuredModels": {
                "quota-model": {
                    "configuredModelId": "quota-model",
                    "name": "Quota Model",
                    "providerId": "openai",
                    "modelId": "gpt-5.4",
                    "credentialRef": "env:OPENAI_API_KEY",
                    "budgetPolicy": {
                        "totalBudgetTokens": 100,
                        "reservationStrategy": "fixed"
                    },
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FailingRuntimeModelDriver),
    );

    let error = adapter
        .run_generation(
            RunRuntimeGenerationInput {
                project_id: Some("proj-redesign".into()),
                configured_model_id: "quota-model".into(),
                content: "Generate a summary".into(),
                system_prompt: None,
            },
            "user-owner",
        )
        .await
        .expect_err("generation should fail");
    assert!(error.to_string().contains("simulated generation failure"));

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let reservation_status: String = connection
        .query_row(
            "SELECT status FROM configured_model_budget_reservations ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("released reservation");
    assert_eq!(reservation_status, "released");
    let (settled_tokens, active_reserved_tokens): (i64, i64) = connection
        .query_row(
            "SELECT settled_tokens, active_reserved_tokens
             FROM configured_model_budget_projections
             WHERE configured_model_id = ?1",
            ["quota-model"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("budget projection");
    assert_eq!(settled_tokens, 0);
    assert_eq!(active_reserved_tokens, 0);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn probe_configured_model_uses_probe_budget_reservation_and_settlement() {
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
                    "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
                    "budgetPolicy": {
                        "totalBudgetTokens": 100,
                        "reservationStrategy": "fixed",
                        "trafficClasses": ["probe"]
                    },
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
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

    let probe = adapter
        .probe_configured_model(RuntimeConfiguredModelProbeInput {
            scope: "workspace".into(),
            configured_model_id: "quota-model".into(),
            patch: json!({}),
            api_key: None,
        })
        .await
        .expect("probe");
    assert!(probe.reachable);
    assert_eq!(probe.consumed_tokens, Some(12));

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let (traffic_class, reservation_status): (String, String) = connection
        .query_row(
            "SELECT traffic_class, status
             FROM configured_model_budget_reservations
             ORDER BY created_at DESC
             LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("probe reservation");
    assert_eq!(traffic_class, "probe");
    assert_eq!(reservation_status, "settled");
    let (settlement_class, settlement_tokens): (String, i64) = connection
        .query_row(
            "SELECT traffic_class, settled_tokens
             FROM configured_model_budget_settlements
             ORDER BY created_at DESC
             LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("probe settlement");
    assert_eq!(settlement_class, "probe");
    assert_eq!(settlement_tokens, 12);
    let (settled_tokens, active_reserved_tokens): (i64, i64) = connection
        .query_row(
            "SELECT settled_tokens, active_reserved_tokens
             FROM configured_model_budget_projections
             WHERE configured_model_id = ?1",
            ["quota-model"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("probe projection");
    assert_eq!(settled_tokens, 12);
    assert_eq!(active_reserved_tokens, 0);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn probe_configured_model_does_not_charge_interactive_only_budget_classes() {
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
                    "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
                    "budgetPolicy": {
                        "totalBudgetTokens": 100,
                        "reservationStrategy": "fixed",
                        "trafficClasses": ["interactive_turn"]
                    },
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
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

    let probe = adapter
        .probe_configured_model(RuntimeConfiguredModelProbeInput {
            scope: "workspace".into(),
            configured_model_id: "quota-model".into(),
            patch: json!({}),
            api_key: None,
        })
        .await
        .expect("probe");
    assert!(probe.reachable);
    assert_eq!(probe.consumed_tokens, Some(12));

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let reservation_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM configured_model_budget_reservations WHERE configured_model_id = ?1",
            ["quota-model"],
            |row| row.get(0),
        )
        .expect("reservation count");
    assert_eq!(reservation_count, 0);
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
    assert_eq!(projection_row, None);

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
async fn submit_turn_blocks_when_configured_model_budget_is_exhausted() {
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
