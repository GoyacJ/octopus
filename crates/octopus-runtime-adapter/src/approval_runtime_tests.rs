use super::adapter_test_support::*;
use super::*;

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
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
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
                canonical_test_agent_ref("agent-team-policy-leader"),
                canonical_test_member_refs(&[
                    "agent-team-policy-leader",
                    "agent-team-policy-worker",
                ]),
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
        Arc::new(MockRuntimeModelDriver),
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
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
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
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
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
                canonical_test_agent_ref("agent-team-spawn-leader"),
                canonical_test_member_refs(&[
                    "agent-team-spawn-leader",
                    "agent-team-spawn-worker",
                ]),
                "Team for team spawn approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team spawn approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Delegation plan ready.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 3,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Leader subrun completed the delegated task.".into(),
            ),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Worker subrun completed the delegated task.".into(),
            ),
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
    assert_eq!(executor.request_count(), 3);

    let resolved_detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("resolved session detail");
    assert!(resolved_detail.pending_approval.is_none());
    assert!(resolved_detail.subruns.len() >= 2);
    assert!(resolved_detail
        .subruns
        .iter()
        .all(|subrun| subrun.status == "completed"));
    assert!(resolved_detail
        .subruns
        .iter()
        .any(|subrun| subrun.actor_ref == "agent:agent-team-spawn-worker"));
    assert!(resolved_detail.workflow.is_some());
    assert!(resolved_detail.background_run.is_some());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn workflow_continuation_approval_blocks_workflow_projection_until_resolved() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
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
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
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
                canonical_test_agent_ref("agent-workflow-approval-leader"),
                canonical_test_member_refs(&[
                    "agent-workflow-approval-leader",
                    "agent-workflow-approval-worker",
                ]),
                "Team for workflow continuation approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workflow approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Workflow plan ready.".into()),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Workflow leader subrun completed its step.".into()),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Workflow worker subrun completed its step.".into()),
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
        .submit_turn(
            &session.summary.id,
            turn_input("Continue the workflow", None),
        )
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
    assert!(run.workflow_run.is_some());
    assert!(run.background_state.is_some());

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert!(detail.subrun_count >= 1);
    assert!(!detail.subruns.is_empty());
    assert!(detail.pending_mailbox.is_some());
    assert!(detail.workflow.is_some());
    assert!(detail.background_run.is_some());
    let initial_events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("initial workflow events");
    assert!(initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.started")));
    assert!(initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.step.started")));
    let initial_background_started = initial_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("background.started"))
        .expect("background started event");
    assert_eq!(
        initial_background_started.run_id.as_deref(),
        Some(run.id.as_str())
    );
    assert_eq!(
        initial_background_started.workflow_run_id.as_deref(),
        run.workflow_run.as_deref()
    );
    assert_eq!(
        initial_background_started.outcome.as_deref(),
        Some("paused")
    );
    let initial_background_paused = initial_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("background.paused"))
        .expect("background paused event");
    assert_eq!(
        initial_background_paused.run_id.as_deref(),
        Some(run.id.as_str())
    );
    assert_eq!(
        initial_background_paused.workflow_run_id.as_deref(),
        run.workflow_run.as_deref()
    );
    assert_eq!(initial_background_paused.outcome.as_deref(), Some("paused"));
    assert!(!initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.step.completed")));
    assert!(!initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.completed")));
    assert_eq!(
        initial_events
            .iter()
            .filter(|event| event.kind.as_deref() == Some("subrun.completed"))
            .count(),
        0
    );
    let replay_after = initial_events
        .last()
        .map(|event| event.id.clone())
        .expect("last initial event id");
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
    assert_eq!(executor.request_count(), 3);
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
    assert!(resolved_detail
        .subruns
        .iter()
        .all(|subrun| subrun.status == "completed"));
    let resolved_events = adapter
        .list_events(&session.summary.id, Some(&replay_after))
        .await
        .expect("resolved workflow events");
    assert!(resolved_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.step.completed")));
    assert!(resolved_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.completed")));
    let background_completed = resolved_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("background.completed"))
        .expect("background completed event");
    assert_eq!(
        background_completed.run_id.as_deref(),
        Some(resolved.id.as_str())
    );
    assert_eq!(
        background_completed.workflow_run_id.as_deref(),
        resolved.workflow_run.as_deref()
    );
    assert_eq!(background_completed.outcome.as_deref(), Some("completed"));
    assert_eq!(
        resolved_events
            .iter()
            .filter(|event| event.kind.as_deref() == Some("subrun.completed"))
            .count(),
        resolved_detail.subruns.len()
    );
    let workflow_terminal = resolved_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("workflow.completed"))
        .expect("workflow completed event");
    assert_eq!(
        workflow_terminal.run_id.as_deref(),
        Some(resolved.id.as_str())
    );
    assert_eq!(
        workflow_terminal.parent_run_id.as_deref(),
        Some(resolved.id.as_str())
    );
    assert_eq!(
        workflow_terminal.workflow_run_id.as_deref(),
        resolved.workflow_run.as_deref()
    );
    assert!(workflow_terminal.workflow_step_id.is_some());
    assert_eq!(workflow_terminal.outcome.as_deref(), Some("completed"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn workflow_continuation_approval_resume_survives_adapter_restart() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-workflow-restart-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Workflow Restart Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the workflow after restart.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for workflow restart approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workflow restart leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-workflow-restart-worker",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Workflow Restart Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["delivery"]).expect("tags"),
                "Do the delegated work after restart.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker for workflow restart approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workflow restart worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                "team-workflow-restart-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Workflow Restart Approval Team",
                Option::<String>::None,
                "Approval aware workflow team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Delegate, pause for workflow approval, then resume after restart.",
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
                canonical_test_agent_ref("agent-workflow-restart-leader"),
                canonical_test_member_refs(&[
                    "agent-workflow-restart-leader",
                    "agent-workflow-restart-worker",
                ]),
                "Team for workflow restart approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workflow restart approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Workflow plan ready for restart.".into()),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Workflow leader subrun resumed after restart.".into(),
            ),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Workflow worker subrun resumed after restart.".into(),
            ),
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
                "conv-workflow-restart-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "Workflow Restart Approval",
                "team:team-workflow-restart-approval",
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
            turn_input("Continue the workflow after restart", None),
        )
        .await
        .expect("pending workflow continuation approval");

    assert_eq!(run.status, "waiting_approval");
    assert_eq!(
        run.pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("workflow-continuation")
    );
    assert_eq!(executor.request_count(), 1);

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert!(!detail.subruns.is_empty());
    assert!(detail.pending_mailbox.is_some());
    assert!(detail.workflow.is_some());
    assert!(detail.background_run.is_some());
    let pending_subrun_count = detail.subruns.len();
    let workflow_run_id = run.workflow_run.clone().expect("workflow run id");
    let workflow_detail = detail
        .run
        .workflow_run_detail
        .as_ref()
        .expect("workflow run detail");
    assert_eq!(workflow_detail.steps.len(), pending_subrun_count + 1);
    assert_eq!(
        workflow_detail.current_step_id.as_deref(),
        Some(run.id.as_str())
    );
    assert_eq!(
        workflow_detail
            .blocking
            .as_ref()
            .map(|blocking| blocking.run_id.as_str()),
        Some(run.id.as_str())
    );
    assert_eq!(
        workflow_detail
            .blocking
            .as_ref()
            .map(|blocking| blocking.target_kind.as_str()),
        Some("workflow-continuation")
    );
    assert_eq!(
        detail
            .background_run
            .as_ref()
            .map(|background| background.continuation_state.as_str()),
        Some("paused")
    );
    assert_eq!(
        detail
            .background_run
            .as_ref()
            .and_then(|background| background.blocking.as_ref())
            .map(|blocking| blocking.target_kind.as_str()),
        Some("workflow-continuation")
    );
    let workflow_state_path = infra
        .paths
        .runtime_state_dir
        .join("workflows")
        .join(format!("{workflow_run_id}.json"));
    let workflow_state: serde_json::Value =
        serde_json::from_slice(&fs::read(&workflow_state_path).expect("workflow state bytes"))
            .expect("workflow state json");
    assert_eq!(
        workflow_state
            .pointer("/detail/steps/0/stepId")
            .and_then(serde_json::Value::as_str),
        Some(run.id.as_str())
    );
    assert_eq!(
        workflow_state
            .pointer("/detail/steps/0/status")
            .and_then(serde_json::Value::as_str),
        Some("waiting_approval")
    );
    assert_eq!(
        workflow_state
            .pointer("/detail/blocking/targetKind")
            .and_then(serde_json::Value::as_str),
        Some("workflow-continuation")
    );
    assert_eq!(
        workflow_state
            .pointer("/background/continuationState")
            .and_then(serde_json::Value::as_str),
        Some("paused")
    );
    let background_state_path = infra
        .paths
        .runtime_state_dir
        .join("background")
        .join(format!(
            "{}.json",
            detail.background_run.as_ref().expect("background").run_id
        ));
    let background_state: serde_json::Value =
        serde_json::from_slice(&fs::read(&background_state_path).expect("background state bytes"))
            .expect("background state json");
    assert_eq!(
        background_state
            .pointer("/summary/continuationState")
            .and_then(serde_json::Value::as_str),
        Some("paused")
    );
    assert_eq!(
        background_state
            .pointer("/summary/blocking/targetKind")
            .and_then(serde_json::Value::as_str),
        Some("workflow-continuation")
    );
    let initial_events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("initial background events");
    assert!(initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("background.started")));
    assert!(initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("background.paused")));
    let workflow_started = initial_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("workflow.started"))
        .expect("workflow started event");
    assert_eq!(
        workflow_started.workflow_step_id.as_deref(),
        Some(run.id.as_str())
    );
    let workflow_step_started = initial_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("workflow.step.started"))
        .expect("workflow step started event");
    assert_eq!(
        workflow_step_started.workflow_step_id.as_deref(),
        Some(run.id.as_str())
    );
    let replay_after = initial_events
        .last()
        .map(|event| event.id.clone())
        .expect("initial event id");
    let approval_id = detail
        .pending_approval
        .as_ref()
        .map(|approval| approval.id.clone())
        .expect("approval id");

    let mut persisted_workflow_state = workflow_state.clone();
    persisted_workflow_state["detail"]["steps"][0]["label"] = json!("Persisted leader node");
    fs::write(
        &workflow_state_path,
        serde_json::to_vec_pretty(&persisted_workflow_state)
            .expect("persisted workflow state bytes"),
    )
    .expect("overwrite workflow state");
    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "UPDATE runtime_workflow_projections
             SET detail_json = ?2
             WHERE workflow_run_id = ?1",
            params![workflow_run_id, "{invalid-workflow-detail"],
        )
        .expect("corrupt workflow detail json");

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );
    let reloaded_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("reloaded session detail");

    assert_eq!(reloaded_detail.subruns.len(), pending_subrun_count);
    assert!(reloaded_detail.pending_mailbox.is_some());
    assert!(reloaded_detail.workflow.is_some());
    assert!(reloaded_detail.background_run.is_some());
    assert_eq!(
        reloaded_detail
            .run
            .workflow_run_detail
            .as_ref()
            .map(|workflow| workflow.steps.len()),
        Some(pending_subrun_count + 1)
    );
    assert_eq!(
        reloaded_detail
            .run
            .workflow_run_detail
            .as_ref()
            .and_then(|workflow| workflow.blocking.as_ref())
            .map(|blocking| blocking.target_kind.as_str()),
        Some("workflow-continuation")
    );
    assert_eq!(
        reloaded_detail
            .run
            .workflow_run_detail
            .as_ref()
            .and_then(|workflow| workflow.steps.first())
            .map(|step| step.label.as_str()),
        Some("Persisted leader node")
    );
    assert_eq!(
        reloaded_detail
            .run
            .workflow_run_detail
            .as_ref()
            .and_then(|workflow| workflow.current_step_label.as_deref()),
        Some("Persisted leader node")
    );
    assert_eq!(
        reloaded_detail
            .background_run
            .as_ref()
            .map(|background| background.continuation_state.as_str()),
        Some("paused")
    );
    assert_eq!(
        reloaded_detail
            .pending_approval
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("workflow-continuation")
    );
    let reloaded_events = reloaded
        .list_events(&session.summary.id, None)
        .await
        .expect("reloaded background events");
    assert!(reloaded_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("background.paused")));

    let resolved = reloaded
        .resolve_approval(
            &session.summary.id,
            &approval_id,
            ResolveRuntimeApprovalInput {
                decision: "approve".into(),
            },
        )
        .await
        .expect("resolved workflow continuation approval after restart");

    assert_eq!(resolved.status, "completed");
    assert_eq!(executor.request_count(), 3);
    assert!(resolved.workflow_run.is_some());
    assert!(resolved.background_state.is_some());

    let resolved_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("resolved detail after restart");
    assert!(resolved_detail.workflow.is_some());
    assert!(resolved_detail.background_run.is_some());
    assert_eq!(resolved_detail.subruns.len(), pending_subrun_count);
    assert!(resolved_detail
        .subruns
        .iter()
        .all(|subrun| subrun.status == "completed"));
    assert_eq!(
        resolved_detail
            .run
            .workflow_run_detail
            .as_ref()
            .map(|workflow| workflow.steps.len()),
        Some(pending_subrun_count + 1)
    );
    assert!(resolved_detail
        .run
        .workflow_run_detail
        .as_ref()
        .and_then(|workflow| workflow.blocking.as_ref())
        .is_none());
    assert!(resolved_detail
        .run
        .workflow_run_detail
        .as_ref()
        .is_some_and(|workflow| workflow.steps.iter().all(|step| step.status == "completed")));
    let resolved_workflow_detail = resolved_detail
        .run
        .workflow_run_detail
        .as_ref()
        .expect("resolved workflow detail");
    let resolved_current_step_id = resolved_workflow_detail
        .current_step_id
        .as_deref()
        .expect("resolved workflow current step id");
    assert_ne!(resolved_current_step_id, "workflow-complete");
    assert!(resolved_workflow_detail
        .steps
        .iter()
        .any(|step| step.step_id == resolved_current_step_id));
    assert_eq!(
        resolved_detail
            .background_run
            .as_ref()
            .map(|background| background.continuation_state.as_str()),
        Some("completed")
    );
    let resolved_events = reloaded
        .list_events(&session.summary.id, Some(&replay_after))
        .await
        .expect("resolved background events");
    assert!(resolved_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("background.completed")));
    let workflow_terminal = resolved_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("workflow.completed"))
        .expect("workflow completed event");
    assert_eq!(
        workflow_terminal.workflow_step_id.as_deref(),
        Some(resolved_current_step_id)
    );
    let workflow_step_completed = resolved_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("workflow.step.completed"))
        .expect("workflow step completed event");
    assert_eq!(
        workflow_step_completed.workflow_step_id.as_deref(),
        Some(resolved_current_step_id)
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_worker_subrun_approval_resume_survives_restart_and_respects_scheduler_queue() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "configuredModels": {
                "scheduler-model": {
                    "configuredModelId": "scheduler-model",
                    "name": "Scheduler Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
    );
    grant_owner_permissions(&infra, "user-owner");

    let output_path = root.join("team-subrun-approval-output.txt");
    let output_path_string = output_path.display().to_string();

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Team Scheduler Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the queued workers.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for team subrun scheduler tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team scheduler leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                "agent-team-subrun-scheduler-worker-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Approval Worker",
                Option::<String>::None,
                "Writer",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Write the delegated file after approval.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker that blocks on capability approval.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert approval worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-worker-queued",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Second Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Finish after the approval worker resumes.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Second queued worker for scheduler tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, permission_envelope_json, delegation_policy_json, approval_preference_json, leader_ref, member_refs, worker_concurrency_limit, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "team-subrun-scheduler-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Subrun Scheduler Approval Team",
                Option::<String>::None,
                "Queue aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Run workers through the runtime scheduler.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({
                    "mode": "leader-orchestrated",
                    "allowBackgroundRuns": true,
                    "allowParallelWorkers": true,
                    "maxWorkerCount": 2
                }))
                .expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "agent:agent-team-subrun-scheduler-leader",
                serde_json::to_string(&vec![
                    "agent:agent-team-subrun-scheduler-worker-approval",
                    "agent:agent-team-subrun-scheduler-worker-queued"
                ])
                .expect("member refs"),
                1_i64,
                "Team for queued subrun approval restart tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Delegation plan ready.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 3,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Writing the queued worker file.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-team-subrun-write".into(),
                name: "bash".into(),
                input: serde_json::json!({
                    "command": format!(
                        "printf 'team subrun approval content\\n' > '{}'",
                        output_path_string
                    ),
                    "run_in_background": true
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
            runtime::AssistantEvent::TextDelta("Approval worker completed after restart.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 7,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Queued worker completed after slot release.".into(),
            ),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 4,
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
                "conv-team-subrun-scheduler-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "Queued Subrun Approval Session",
                "team:team-subrun-scheduler-approval",
                Some("scheduler-model"),
                octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Queue the workers and pause the first on approval", None),
        )
        .await
        .expect("pending queued subrun approval");

    assert_eq!(pending_run.status, "waiting_approval");

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert_eq!(detail.subruns.len(), 2);
    assert_eq!(
        detail
            .pending_approval
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("capability-call")
    );
    assert!(detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-approval"
            && subrun.status == "waiting_approval"
    }));
    assert!(detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-queued"
            && subrun.status == "queued"
    }));
    assert_eq!(executor.request_count(), 2);
    let approval_id = detail
        .pending_approval
        .as_ref()
        .map(|approval| approval.id.clone())
        .expect("approval id");
    let blocked_subrun_run_id = detail
        .subruns
        .iter()
        .find(|subrun| subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-approval")
        .map(|subrun| subrun.run_id.clone())
        .expect("blocked subrun run id");

    {
        let mut sessions = adapter.state.sessions.lock().expect("sessions mutex");
        let aggregate = sessions
            .get_mut(&session.summary.id)
            .expect("runtime aggregate");
        aggregate.detail.messages.push(RuntimeMessage {
            id: "msg-parent-drift".into(),
            session_id: session.summary.id.clone(),
            conversation_id: session.summary.conversation_id.clone(),
            sender_type: "user".into(),
            sender_label: "User".into(),
            content: "MUTATED parent prompt drift".into(),
            timestamp: timestamp_now(),
            configured_model_id: aggregate.detail.run.configured_model_id.clone(),
            configured_model_name: aggregate.detail.run.configured_model_name.clone(),
            model_id: aggregate.detail.run.model_id.clone(),
            status: aggregate.detail.run.status.clone(),
            requested_actor_kind: aggregate.detail.run.requested_actor_kind.clone(),
            requested_actor_id: aggregate.detail.run.requested_actor_id.clone(),
            resolved_actor_kind: aggregate.detail.run.resolved_actor_kind.clone(),
            resolved_actor_id: aggregate.detail.run.resolved_actor_id.clone(),
            resolved_actor_label: aggregate.detail.run.resolved_actor_label.clone(),
            used_default_actor: Some(false),
            resource_ids: Some(Vec::new()),
            attachments: Some(Vec::new()),
            artifacts: Some(Vec::new()),
            deliverable_refs: None,
            usage: None,
            tool_calls: None,
            process_entries: None,
        });
        aggregate.detail.summary.last_message_preview = Some("MUTATED parent prompt drift".into());

        let session_policy = adapter
            .load_session_policy_snapshot(&aggregate.metadata.session_policy_snapshot_ref)
            .expect("session policy");
        let actor_manifest = adapter
            .load_actor_manifest_snapshot(&session_policy.manifest_snapshot_ref)
            .expect("actor manifest");
        let actor_manifest::CompiledActorManifest::Team(team_manifest) = actor_manifest else {
            panic!("expected team manifest");
        };
        team_runtime::ensure_subrun_state_metadata_for_session(
            &adapter,
            aggregate,
            &team_manifest,
            &session_policy,
            timestamp_now(),
        )
        .expect("refresh subrun metadata");
        assert_eq!(
            aggregate
                .metadata
                .subrun_states
                .get(&blocked_subrun_run_id)
                .expect("blocked subrun state")
                .dispatch
                .worker_input
                .content,
            "Queue the workers and pause the first on approval"
        );
        adapter
            .persist_runtime_projections(aggregate)
            .expect("persist mutated aggregate");
    }

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );
    let reloaded_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("reloaded detail");

    assert_eq!(reloaded_detail.subruns.len(), 2);
    assert_eq!(
        reloaded_detail
            .pending_approval
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("capability-call")
    );
    assert!(reloaded_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-approval"
            && subrun.status == "waiting_approval"
    }));
    assert!(reloaded_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-queued"
            && subrun.status == "queued"
    }));

    let resolved = reloaded
        .resolve_approval(
            &session.summary.id,
            &approval_id,
            ResolveRuntimeApprovalInput {
                decision: "approve".into(),
            },
        )
        .await
        .expect("resolved queued subrun approval after restart");

    assert_eq!(resolved.status, "completed");
    assert_eq!(executor.request_count(), 4);
    let requests = executor.requests();
    assert_eq!(
        last_user_text(&requests[2]),
        Some("Queue the workers and pause the first on approval")
    );
    for _ in 0..20 {
        if fs::read_to_string(&output_path)
            .map(|content| content == "team subrun approval content\n")
            .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        fs::read_to_string(&output_path).expect("written output"),
        "team subrun approval content\n"
    );

    let resolved_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("resolved detail");
    assert!(resolved_detail.pending_approval.is_none());
    assert!(resolved_detail
        .subruns
        .iter()
        .all(|subrun| subrun.status == "completed"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_subrun_metadata_refresh_rehydrates_from_manifest_plan_without_detail_subruns() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
    grant_owner_permissions(&infra, "user-owner");

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
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
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
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                canonical_test_agent_ref("agent-orchestrator"),
                canonical_test_member_refs(&["agent-orchestrator", "agent-project-delivery"]),
                "Core workspace decision board.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workspace core team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Leader finished.".into()),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Worker one finished.".into()),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Worker two finished.".into()),
            runtime::AssistantEvent::MessageStop,
        ],
    ]));
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor,
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-team-refresh-state-first",
                octopus_core::DEFAULT_PROJECT_ID,
                "Team Refresh State First",
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
        .expect("team run");
    assert!(!run.status.is_empty());

    let original_detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("original detail");
    let original_subrun_ids = original_detail
        .subruns
        .iter()
        .map(|subrun| subrun.run_id.clone())
        .collect::<Vec<_>>();
    assert!(!original_subrun_ids.is_empty());

    {
        let mut sessions = adapter.state.sessions.lock().expect("sessions mutex");
        let aggregate = sessions
            .get_mut(&session.summary.id)
            .expect("runtime aggregate");
        aggregate.detail.subruns.clear();
        aggregate.detail.subrun_count = 0;
        aggregate.detail.handoffs.clear();
        aggregate.detail.pending_mailbox = None;
        aggregate.detail.workflow = None;
        aggregate.detail.background_run = None;
        aggregate.detail.run.worker_dispatch = None;
        aggregate.detail.run.workflow_run = None;
        aggregate.detail.run.workflow_run_detail = None;
        aggregate.detail.run.mailbox_ref = None;
        aggregate.detail.run.handoff_ref = None;
        aggregate.detail.run.background_state = None;

        let session_policy = adapter
            .load_session_policy_snapshot(&aggregate.metadata.session_policy_snapshot_ref)
            .expect("session policy");
        let actor_manifest = adapter
            .load_actor_manifest_snapshot(&session_policy.manifest_snapshot_ref)
            .expect("actor manifest");
        let actor_manifest::CompiledActorManifest::Team(team_manifest) = actor_manifest else {
            panic!("expected team manifest");
        };

        team_runtime::ensure_subrun_state_metadata_for_session(
            &adapter,
            aggregate,
            &team_manifest,
            &session_policy,
            timestamp_now(),
        )
        .expect("refresh subrun metadata from manifest plan");

        let rebuilt_subrun_ids = aggregate
            .detail
            .subruns
            .iter()
            .map(|subrun| subrun.run_id.clone())
            .collect::<Vec<_>>();
        assert_eq!(rebuilt_subrun_ids, original_subrun_ids);
        assert_eq!(
            aggregate.metadata.subrun_states.len(),
            original_subrun_ids.len()
        );
        assert!(aggregate.detail.pending_mailbox.is_some());
        assert!(aggregate.detail.workflow.is_some());
        assert!(aggregate.detail.background_run.is_some());
        assert!(aggregate.detail.run.worker_dispatch.is_some());
    }

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_worker_subrun_explicit_cancel_releases_scheduler_queue() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "configuredModels": {
                "scheduler-model": {
                    "configuredModelId": "scheduler-model",
                    "name": "Scheduler Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
    );
    grant_owner_permissions(&infra, "user-owner");

    let output_path = root.join("team-subrun-explicit-cancel-output.txt");
    let output_path_string = output_path.display().to_string();

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Team Scheduler Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the queued workers.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for team subrun cancellation tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team scheduler leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                "agent-team-subrun-scheduler-worker-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Approval Worker",
                Option::<String>::None,
                "Writer",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Write the delegated file after approval.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker that blocks on capability approval.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert approval worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-worker-queued",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Second Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Finish after the first worker is cancelled.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Second queued worker for cancellation tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, permission_envelope_json, delegation_policy_json, approval_preference_json, leader_ref, member_refs, worker_concurrency_limit, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "team-subrun-scheduler-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Subrun Scheduler Approval Team",
                Option::<String>::None,
                "Queue aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Run workers through the runtime scheduler.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({
                    "mode": "leader-orchestrated",
                    "allowBackgroundRuns": true,
                    "allowParallelWorkers": true,
                    "maxWorkerCount": 2
                }))
                .expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "agent:agent-team-subrun-scheduler-leader",
                serde_json::to_string(&vec![
                    "agent:agent-team-subrun-scheduler-worker-approval",
                    "agent:agent-team-subrun-scheduler-worker-queued"
                ])
                .expect("member refs"),
                1_i64,
                "Team for explicit subrun cancellation tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Delegation plan ready.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 3,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Writing the queued worker file.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-team-subrun-explicit-cancel-write".into(),
                name: "bash".into(),
                input: serde_json::json!({
                    "command": format!(
                        "printf 'team subrun explicit cancel content\\n' > '{}'",
                        output_path_string
                    ),
                    "run_in_background": true
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
            runtime::AssistantEvent::TextDelta(
                "Queued worker completed after explicit subrun cancellation released the slot."
                    .into(),
            ),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 4,
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
                "conv-team-subrun-explicit-cancel",
                octopus_core::DEFAULT_PROJECT_ID,
                "Queued Subrun Explicit Cancel Session",
                "team:team-subrun-scheduler-approval",
                Some("scheduler-model"),
                octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input(
                "Queue the workers and cancel the first blocked subrun",
                None,
            ),
        )
        .await
        .expect("pending queued subrun approval");

    assert_eq!(pending_run.status, "waiting_approval");

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert_eq!(detail.subruns.len(), 2);
    let blocked_subrun = detail
        .subruns
        .iter()
        .find(|subrun| subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-approval")
        .expect("blocked subrun")
        .clone();
    assert_eq!(blocked_subrun.status, "waiting_approval");
    assert!(detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-queued"
            && subrun.status == "queued"
    }));
    assert_eq!(executor.request_count(), 2);
    let replay_after = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("initial events")
        .last()
        .map(|event| event.id.clone())
        .expect("initial event id");

    let cancelled = adapter
        .cancel_subrun(
            &session.summary.id,
            &blocked_subrun.run_id,
            CancelRuntimeSubrunInput {
                note: Some("skip the first worker".into()),
            },
        )
        .await
        .expect("cancel subrun");

    assert_eq!(cancelled.status, "failed");
    assert_eq!(executor.request_count(), 3);
    assert!(!output_path.exists());

    let cancelled_detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("cancelled detail");
    assert!(cancelled_detail.pending_approval.is_none());
    assert!(cancelled_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-approval"
            && subrun.status == "cancelled"
    }));
    assert!(cancelled_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-queued"
            && subrun.status == "completed"
    }));

    let cancelled_events = adapter
        .list_events(&session.summary.id, Some(&replay_after))
        .await
        .expect("cancelled events");
    let cancelled_subrun_event = cancelled_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("subrun.cancelled"))
        .expect("subrun cancelled event");
    assert_eq!(
        cancelled_subrun_event.actor_ref.as_deref(),
        Some("agent:agent-team-subrun-scheduler-worker-approval")
    );
    assert_eq!(cancelled_subrun_event.outcome.as_deref(), Some("cancelled"));
    assert!(cancelled_events.iter().any(|event| {
        event.kind.as_deref() == Some("subrun.completed")
            && event.actor_ref.as_deref() == Some("agent:agent-team-subrun-scheduler-worker-queued")
    }));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_worker_subrun_auth_resume_survives_restart_and_respects_scheduler_queue() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "configuredModels": {
                "scheduler-model": {
                    "configuredModelId": "scheduler-model",
                    "name": "Scheduler Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
    );
    grant_owner_permissions(&infra, "user-owner");

    let output_path = root.join("team-subrun-auth-output.txt");
    let output_path_string = output_path.display().to_string();

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-auth-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Team Scheduler Auth Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the queued workers.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for team subrun auth scheduler tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team scheduler auth leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                "agent-team-subrun-scheduler-worker-auth",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Auth Worker",
                Option::<String>::None,
                "Writer",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Write the delegated file after mediation resumes.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker that will be rewritten into an auth-blocked subrun.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert auth worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-worker-auth-queued",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Auth Second Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Finish after the auth-blocked worker resumes.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Second queued worker for auth scheduler tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued auth worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, permission_envelope_json, delegation_policy_json, approval_preference_json, leader_ref, member_refs, worker_concurrency_limit, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "team-subrun-scheduler-auth",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Subrun Scheduler Auth Team",
                Option::<String>::None,
                "Queue aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Run workers through the runtime scheduler.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({
                    "mode": "leader-orchestrated",
                    "allowBackgroundRuns": true,
                    "allowParallelWorkers": true,
                    "maxWorkerCount": 2
                }))
                .expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "agent:agent-team-subrun-scheduler-auth-leader",
                serde_json::to_string(&vec![
                    "agent:agent-team-subrun-scheduler-worker-auth",
                    "agent:agent-team-subrun-scheduler-worker-auth-queued"
                ])
                .expect("member refs"),
                1_i64,
                "Team for queued subrun auth restart tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued auth team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Delegation plan ready.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 3,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Writing the queued worker file.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-team-subrun-auth-write".into(),
                name: "bash".into(),
                input: serde_json::json!({
                    "command": format!(
                        "printf 'team subrun auth content\\n' > '{}'",
                        output_path_string
                    ),
                    "run_in_background": true
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
            runtime::AssistantEvent::TextDelta("Auth worker completed after restart.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 7,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta(
                "Queued worker completed after auth slot release.".into(),
            ),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 4,
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
                "conv-team-subrun-scheduler-auth",
                octopus_core::DEFAULT_PROJECT_ID,
                "Queued Subrun Auth Session",
                "team:team-subrun-scheduler-auth",
                Some("scheduler-model"),
                octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Queue the workers and pause the first on approval", None),
        )
        .await
        .expect("pending queued subrun approval");

    assert_eq!(pending_run.status, "waiting_approval");

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert_eq!(detail.subruns.len(), 2);
    let blocked_subrun = detail
        .subruns
        .iter()
        .find(|subrun| subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth")
        .expect("blocked subrun")
        .clone();
    assert_eq!(blocked_subrun.status, "waiting_approval");
    assert!(detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth-queued"
            && subrun.status == "queued"
    }));
    assert_eq!(executor.request_count(), 2);

    let blocked_state_path = infra
        .paths
        .runtime_state_dir
        .join("subruns")
        .join(format!("{}.json", blocked_subrun.run_id));
    let mut blocked_state: serde_json::Value =
        serde_json::from_slice(&fs::read(&blocked_state_path).expect("subrun state bytes"))
            .expect("subrun state json");
    let capability_state_ref = blocked_state["run"]["capabilityStateRef"]
        .as_str()
        .expect("capability state ref")
        .to_string();
    let capability_store = adapter
        .load_capability_store(Some(&capability_state_ref))
        .expect("capability store");
    capability_store.approve_tool("bash");
    adapter
        .persist_capability_store(&capability_state_ref, &capability_store)
        .expect("persist capability store");

    let auth_challenge_id = format!("auth-{}", blocked_subrun.run_id);
    let auth_challenge = json!({
        "approvalLayer": "provider",
        "capabilityId": blocked_state["run"]["checkpoint"]["capabilityId"],
        "checkpointRef": blocked_state["run"]["checkpoint"]["checkpointArtifactRef"],
        "conversationId": session.summary.conversation_id,
        "createdAt": blocked_subrun.updated_at + 9,
        "detail": "Worker requires provider authentication before replay.",
        "escalationReason": "provider authentication is required for the delegated tool",
        "id": auth_challenge_id,
        "dispatchKind": "capability-call",
        "providerKey": "delegated-provider",
        "concurrencyPolicy": "serialized",
        "input": {
            "command": format!(
                "printf 'team subrun auth content\\n' > '{}'",
                output_path_string
            ),
            "run_in_background": true
        },
        "requiredPermission": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
        "requiresApproval": false,
        "requiresAuth": true,
        "runId": blocked_subrun.run_id,
        "sessionId": session.summary.id,
        "status": "pending",
        "summary": "Complete provider authentication for the delegated worker.",
        "targetKind": "capability-call",
        "targetRef": "bash",
        "toolName": "bash"
    });
    let pending_mediation = json!({
        "approvalLayer": "provider",
        "authChallengeId": auth_challenge_id,
        "capabilityId": blocked_state["run"]["checkpoint"]["capabilityId"],
        "checkpointRef": blocked_state["run"]["checkpoint"]["checkpointArtifactRef"],
        "detail": "Worker requires provider authentication before replay.",
        "escalationReason": "provider authentication is required for the delegated tool",
        "mediationId": format!("mediation-{}", blocked_subrun.run_id),
        "mediationKind": "auth",
        "dispatchKind": "capability-call",
        "providerKey": "delegated-provider",
        "concurrencyPolicy": "serialized",
        "input": {
            "command": format!(
                "printf 'team subrun auth content\\n' > '{}'",
                output_path_string
            ),
            "run_in_background": true
        },
        "reason": "provider authentication required",
        "requiredPermission": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
        "requiresApproval": false,
        "requiresAuth": true,
        "state": "pending",
        "summary": "Complete provider authentication for the delegated worker.",
        "targetKind": "capability-call",
        "targetRef": "bash",
        "toolName": "bash"
    });

    blocked_state["run"]["status"] = json!("auth-required");
    blocked_state["run"]["currentStep"] = json!("awaiting_auth");
    blocked_state["run"]["updatedAt"] = json!(blocked_subrun.updated_at + 11);
    blocked_state["run"]["nextAction"] = json!("auth");
    blocked_state["run"]["approvalState"] = json!("auth-required");
    blocked_state["run"]["approvalTarget"] = serde_json::Value::Null;
    blocked_state["run"]["authTarget"] = auth_challenge.clone();
    blocked_state["run"]["pendingMediation"] = pending_mediation.clone();
    blocked_state["dispatch"]["workerInput"]["content"] = json!("");
    blocked_state["run"]["checkpoint"]["pendingApproval"] = serde_json::Value::Null;
    blocked_state["run"]["checkpoint"]["pendingAuthChallenge"] = auth_challenge;
    blocked_state["run"]["checkpoint"]["pendingMediation"] = pending_mediation;
    fs::write(
        &blocked_state_path,
        serde_json::to_vec_pretty(&blocked_state).expect("mutated subrun state bytes"),
    )
    .expect("overwrite subrun state");

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );
    let reloaded_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("reloaded detail");

    assert_eq!(reloaded_detail.subruns.len(), 2);
    assert!(reloaded_detail.pending_approval.is_none());
    assert_eq!(
        reloaded_detail
            .run
            .auth_target
            .as_ref()
            .map(|challenge| challenge.id.as_str()),
        Some(auth_challenge_id.as_str())
    );
    assert!(reloaded_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth"
            && subrun.status == "auth-required"
    }));
    assert!(reloaded_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth-queued"
            && subrun.status == "queued"
    }));
    let initial_events = reloaded
        .list_events(&session.summary.id, None)
        .await
        .expect("initial auth workflow events");
    assert!(initial_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("background.started")));
    let background_paused = initial_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("background.paused"))
        .expect("background paused event");
    assert_eq!(
        background_paused.workflow_run_id.as_deref(),
        reloaded_detail.run.workflow_run.as_deref()
    );
    assert_eq!(background_paused.outcome.as_deref(), Some("paused"));
    let replay_after = initial_events
        .last()
        .map(|event| event.id.clone())
        .expect("initial auth event id");

    let resolved = reloaded
        .resolve_auth_challenge(
            &session.summary.id,
            &auth_challenge_id,
            ResolveRuntimeAuthChallengeInput {
                resolution: "resolved".into(),
                note: Some("provider linked".into()),
            },
        )
        .await
        .expect("resolved queued subrun auth after restart");

    assert_eq!(resolved.status, "completed");
    assert_eq!(executor.request_count(), 4);
    for _ in 0..20 {
        if fs::read_to_string(&output_path)
            .map(|content| content == "team subrun auth content\n")
            .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        fs::read_to_string(&output_path).expect("written output"),
        "team subrun auth content\n"
    );

    let resolved_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("resolved detail");
    assert!(resolved_detail.run.auth_target.is_none());
    assert!(resolved_detail.pending_approval.is_none());
    assert!(resolved_detail
        .subruns
        .iter()
        .all(|subrun| subrun.status == "completed"));
    let resolved_events = reloaded
        .list_events(&session.summary.id, Some(&replay_after))
        .await
        .expect("resolved auth workflow events");
    assert!(resolved_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.step.completed")));
    assert!(resolved_events
        .iter()
        .any(|event| event.kind.as_deref() == Some("workflow.completed")));
    let background_completed = resolved_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("background.completed"))
        .expect("background completed event");
    assert_eq!(
        background_completed.workflow_run_id.as_deref(),
        resolved.workflow_run.as_deref()
    );
    assert_eq!(background_completed.outcome.as_deref(), Some("completed"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_worker_subrun_auth_cancellation_releases_scheduler_queue_and_emits_cancelled_state() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "configuredModels": {
                "scheduler-model": {
                    "configuredModelId": "scheduler-model",
                    "name": "Scheduler Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }),
    );
    grant_owner_permissions(&infra, "user-owner");

    let output_path = root.join("team-subrun-auth-cancel-output.txt");
    let output_path_string = output_path.display().to_string();

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-auth-leader",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Team Scheduler Auth Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the queued workers.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for team subrun auth scheduler cancellation tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert team scheduler auth leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                "agent-team-subrun-scheduler-worker-auth",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Auth Worker",
                Option::<String>::None,
                "Writer",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Write the delegated file after mediation resumes.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker that will be rewritten into an auth-blocked subrun.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert auth worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-subrun-scheduler-worker-auth-queued",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Queued Auth Second Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Finish after the auth-blocked worker resumes or is cancelled.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Second queued worker for auth scheduler cancellation tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued auth worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, permission_envelope_json, delegation_policy_json, approval_preference_json, leader_ref, member_refs, worker_concurrency_limit, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "team-subrun-scheduler-auth",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Subrun Scheduler Auth Team",
                Option::<String>::None,
                "Queue aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Run workers through the runtime scheduler.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "defaultMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "maxMode": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
                    "escalationAllowed": true,
                    "allowedResourceScopes": ["agent-private", "project-shared"]
                }))
                .expect("permission envelope"),
                serde_json::to_string(&json!({
                    "mode": "leader-orchestrated",
                    "allowBackgroundRuns": true,
                    "allowParallelWorkers": true,
                    "maxWorkerCount": 2
                }))
                .expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "agent:agent-team-subrun-scheduler-auth-leader",
                serde_json::to_string(&vec![
                    "agent:agent-team-subrun-scheduler-worker-auth",
                    "agent:agent-team-subrun-scheduler-worker-auth-queued"
                ])
                .expect("member refs"),
                1_i64,
                "Team for queued subrun auth cancellation tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert queued auth team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Delegation plan ready.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 3,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Writing the queued worker file.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-team-subrun-auth-cancel-write".into(),
                name: "bash".into(),
                input: serde_json::json!({
                    "command": format!(
                        "printf 'team subrun auth cancel content\\n' > '{}'",
                        output_path_string
                    ),
                    "run_in_background": true
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
            runtime::AssistantEvent::TextDelta(
                "Queued worker completed after auth cancellation released the slot.".into(),
            ),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 4,
                output_tokens: 4,
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
                "conv-team-subrun-scheduler-auth-cancel",
                octopus_core::DEFAULT_PROJECT_ID,
                "Queued Subrun Auth Cancellation Session",
                "team:team-subrun-scheduler-auth",
                Some("scheduler-model"),
                octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Queue the workers and cancel the blocked auth worker", None),
        )
        .await
        .expect("pending queued subrun auth cancellation");

    assert_eq!(pending_run.status, "waiting_approval");

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert_eq!(detail.subruns.len(), 2);
    let blocked_subrun = detail
        .subruns
        .iter()
        .find(|subrun| subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth")
        .expect("blocked subrun")
        .clone();
    assert_eq!(blocked_subrun.status, "waiting_approval");
    assert!(detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth-queued"
            && subrun.status == "queued"
    }));
    assert_eq!(executor.request_count(), 2);

    let blocked_state_path = infra
        .paths
        .runtime_state_dir
        .join("subruns")
        .join(format!("{}.json", blocked_subrun.run_id));
    let mut blocked_state: serde_json::Value =
        serde_json::from_slice(&fs::read(&blocked_state_path).expect("subrun state bytes"))
            .expect("subrun state json");
    let capability_state_ref = blocked_state["run"]["capabilityStateRef"]
        .as_str()
        .expect("capability state ref")
        .to_string();
    let capability_store = adapter
        .load_capability_store(Some(&capability_state_ref))
        .expect("capability store");
    capability_store.approve_tool("bash");
    adapter
        .persist_capability_store(&capability_state_ref, &capability_store)
        .expect("persist capability store");

    let auth_challenge_id = format!("auth-{}", blocked_subrun.run_id);
    let auth_challenge = json!({
        "approvalLayer": "provider",
        "capabilityId": blocked_state["run"]["checkpoint"]["capabilityId"],
        "checkpointRef": blocked_state["run"]["checkpoint"]["checkpointArtifactRef"],
        "conversationId": session.summary.conversation_id,
        "createdAt": blocked_subrun.updated_at + 9,
        "detail": "Worker requires provider authentication before replay.",
        "escalationReason": "provider authentication is required for the delegated tool",
        "id": auth_challenge_id,
        "dispatchKind": "capability-call",
        "providerKey": "delegated-provider",
        "concurrencyPolicy": "serialized",
        "input": {
            "command": format!(
                "printf 'team subrun auth cancel content\\n' > '{}'",
                output_path_string
            ),
            "run_in_background": true
        },
        "requiredPermission": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
        "requiresApproval": false,
        "requiresAuth": true,
        "runId": blocked_subrun.run_id,
        "sessionId": session.summary.id,
        "status": "pending",
        "summary": "Complete provider authentication for the delegated worker.",
        "targetKind": "capability-call",
        "targetRef": "bash",
        "toolName": "bash"
    });
    let pending_mediation = json!({
        "approvalLayer": "provider",
        "authChallengeId": auth_challenge_id,
        "capabilityId": blocked_state["run"]["checkpoint"]["capabilityId"],
        "checkpointRef": blocked_state["run"]["checkpoint"]["checkpointArtifactRef"],
        "detail": "Worker requires provider authentication before replay.",
        "escalationReason": "provider authentication is required for the delegated tool",
        "mediationId": format!("mediation-{}", blocked_subrun.run_id),
        "mediationKind": "auth",
        "dispatchKind": "capability-call",
        "providerKey": "delegated-provider",
        "concurrencyPolicy": "serialized",
        "input": {
            "command": format!(
                "printf 'team subrun auth cancel content\\n' > '{}'",
                output_path_string
            ),
            "run_in_background": true
        },
        "reason": "provider authentication required",
        "requiredPermission": octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
        "requiresApproval": false,
        "requiresAuth": true,
        "state": "pending",
        "summary": "Complete provider authentication for the delegated worker.",
        "targetKind": "capability-call",
        "targetRef": "bash",
        "toolName": "bash"
    });

    blocked_state["run"]["status"] = json!("auth-required");
    blocked_state["run"]["currentStep"] = json!("awaiting_auth");
    blocked_state["run"]["updatedAt"] = json!(blocked_subrun.updated_at + 11);
    blocked_state["run"]["nextAction"] = json!("auth");
    blocked_state["run"]["approvalState"] = json!("auth-required");
    blocked_state["run"]["approvalTarget"] = serde_json::Value::Null;
    blocked_state["run"]["authTarget"] = auth_challenge.clone();
    blocked_state["run"]["pendingMediation"] = pending_mediation.clone();
    blocked_state["dispatch"]["workerInput"]["content"] = json!("");
    blocked_state["run"]["checkpoint"]["pendingApproval"] = serde_json::Value::Null;
    blocked_state["run"]["checkpoint"]["pendingAuthChallenge"] = auth_challenge;
    blocked_state["run"]["checkpoint"]["pendingMediation"] = pending_mediation;
    fs::write(
        &blocked_state_path,
        serde_json::to_vec_pretty(&blocked_state).expect("mutated subrun state bytes"),
    )
    .expect("overwrite subrun state");

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );
    let reloaded_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("reloaded detail");

    assert_eq!(reloaded_detail.subruns.len(), 2);
    assert!(reloaded_detail.pending_approval.is_none());
    assert_eq!(
        reloaded_detail
            .run
            .auth_target
            .as_ref()
            .map(|challenge| challenge.id.as_str()),
        Some(auth_challenge_id.as_str())
    );
    assert!(reloaded_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth"
            && subrun.status == "auth-required"
    }));
    assert!(reloaded_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth-queued"
            && subrun.status == "queued"
    }));
    let initial_events = reloaded
        .list_events(&session.summary.id, None)
        .await
        .expect("initial auth workflow events");
    let replay_after = initial_events
        .last()
        .map(|event| event.id.clone())
        .expect("initial auth event id");

    let cancelled = reloaded
        .resolve_auth_challenge(
            &session.summary.id,
            &auth_challenge_id,
            ResolveRuntimeAuthChallengeInput {
                resolution: "cancelled".into(),
                note: Some("provider login abandoned".into()),
            },
        )
        .await
        .expect("cancelled queued subrun auth after restart");

    assert_eq!(cancelled.status, "failed");
    assert_eq!(executor.request_count(), 3);
    assert!(!output_path.exists());

    let cancelled_detail = reloaded
        .get_session(&session.summary.id)
        .await
        .expect("cancelled detail");
    assert!(cancelled_detail.run.auth_target.is_none());
    assert!(cancelled_detail.pending_approval.is_none());
    assert!(cancelled_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth"
            && subrun.status == "cancelled"
    }));
    assert!(cancelled_detail.subruns.iter().any(|subrun| {
        subrun.actor_ref == "agent:agent-team-subrun-scheduler-worker-auth-queued"
            && subrun.status == "completed"
    }));

    let cancelled_events = reloaded
        .list_events(&session.summary.id, Some(&replay_after))
        .await
        .expect("cancelled auth workflow events");
    let cancelled_subrun_event = cancelled_events
        .iter()
        .find(|event| event.kind.as_deref() == Some("subrun.cancelled"))
        .expect("subrun cancelled event");
    assert_eq!(
        cancelled_subrun_event.actor_ref.as_deref(),
        Some("agent:agent-team-subrun-scheduler-worker-auth")
    );
    assert_eq!(cancelled_subrun_event.outcome.as_deref(), Some("cancelled"));
    assert!(cancelled_events.iter().any(|event| {
        event.kind.as_deref() == Some("subrun.completed")
            && event.actor_ref.as_deref()
                == Some("agent:agent-team-subrun-scheduler-worker-auth-queued")
    }));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn team_spawn_approval_chains_into_workflow_continuation_approval_when_required() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
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
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
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
                canonical_test_agent_ref("agent-team-spawn-workflow-leader"),
                canonical_test_member_refs(&[
                    "agent-team-spawn-workflow-leader",
                    "agent-team-spawn-workflow-worker",
                ]),
                "Team for chained workflow approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert chained approval team");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![vec![
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
    assert!(spawn_resolved.workflow_run.is_some());
    assert!(spawn_resolved.background_state.is_some());
    assert_eq!(executor.request_count(), 1);

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail after spawn approval");
    assert!(!detail.subruns.is_empty());
    assert!(detail.workflow.is_some());
    assert!(detail.background_run.is_some());
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
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
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
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, delegation_policy_json, approval_preference_json, leader_ref, member_refs, worker_concurrency_limit, description, status, updated_at)
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
                canonical_test_agent_ref("agent-team-policy-leader"),
                canonical_test_member_refs(&[
                    "agent-team-policy-leader",
                    "agent-team-policy-worker",
                ]),
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
        Arc::new(MockRuntimeModelDriver),
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

    let subrun_state_dir = infra.paths.runtime_state_dir.join("subruns");
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
