use super::adapter_test_support::*;
use super::*;

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
