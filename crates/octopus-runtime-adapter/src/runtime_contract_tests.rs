use super::adapter_test_support::*;
use super::*;

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
