use super::adapter_test_support::*;
use super::*;

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
