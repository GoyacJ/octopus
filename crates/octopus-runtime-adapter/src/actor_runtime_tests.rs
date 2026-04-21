use super::adapter_test_support::*;
use super::*;

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
        Arc::new(MockRuntimeModelDriver),
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
                        "teamSpawn": "auto"
                        ,
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
        .runtime_state_dir
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
        .get("manifestSnapshotRef")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.is_empty()));
    assert!(first_subrun_state
        .get("sessionPolicySnapshotRef")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(
        first_subrun_state
            .get("dispatch")
            .and_then(|value| value.get("dispatchKey"))
            .and_then(serde_json::Value::as_str),
        Some("team-dispatch-1")
    );
    assert_eq!(
        first_subrun_state
            .get("dispatch")
            .and_then(|value| value.get("workerInput"))
            .and_then(|value| value.get("content"))
            .and_then(serde_json::Value::as_str),
        Some("Review the proposal")
    );
    assert_eq!(
        first_subrun_state
            .get("dispatch")
            .and_then(|value| value.get("mailboxPolicy"))
            .and_then(|value| value.get("mode"))
            .and_then(serde_json::Value::as_str),
        Some("leader-hub")
    );
    assert_eq!(
        first_subrun_state
            .get("dispatch")
            .and_then(|value| value.get("artifactHandoffPolicy"))
            .and_then(|value| value.get("mode"))
            .and_then(serde_json::Value::as_str),
        Some("leader-reviewed")
    );
    assert_eq!(
        first_subrun_state
            .get("serializedSession")
            .and_then(|value| value.get("session"))
            .and_then(|value| value.get("messages"))
            .and_then(serde_json::Value::as_array)
            .and_then(|messages| messages.first())
            .and_then(|value| value.get("blocks"))
            .and_then(serde_json::Value::as_array)
            .and_then(|blocks| blocks.first())
            .and_then(|value| value.get("text"))
            .and_then(serde_json::Value::as_str),
        Some("Review the proposal")
    );

    let mailbox = detail.pending_mailbox.as_ref().expect("mailbox summary");
    assert_eq!(mailbox.channel, "leader-hub");
    assert_eq!(mailbox.status, "completed");
    assert_eq!(mailbox.pending_count, 0);
    assert!(mailbox.total_messages >= 2);
    assert!(detail
        .handoffs
        .iter()
        .all(|handoff| handoff.state == "acknowledged"));
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
    let handoff_projection_rows = {
        let mut statement = connection
            .prepare(
                "SELECT handoff_ref, parent_run_id, delegated_by_tool_call_id, sender_actor_ref,
                        receiver_actor_ref, mailbox_ref, state, artifact_refs_json,
                        envelope_storage_path, envelope_content_hash
                 FROM runtime_handoff_projections
                 WHERE session_id = ?1 AND run_id = ?2
                 ORDER BY updated_at ASC, handoff_ref ASC",
            )
            .expect("handoff projection statement");
        statement
            .query_map(params![session.summary.id, run.id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                ))
            })
            .expect("handoff projection rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect handoff projection rows")
    };
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
    assert_eq!(
        handoff_projection_rows.len() as i64,
        handoff_projection_count
    );
    let subruns_by_handoff_ref = detail
        .subruns
        .iter()
        .filter_map(|subrun| {
            subrun
                .handoff_ref
                .as_ref()
                .map(|handoff_ref| (handoff_ref.as_str(), subrun))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let handoffs_by_ref = detail
        .handoffs
        .iter()
        .map(|handoff| (handoff.handoff_ref.as_str(), handoff))
        .collect::<std::collections::BTreeMap<_, _>>();
    for (
        handoff_ref,
        parent_run_id,
        delegated_by_tool_call_id,
        sender_actor_ref,
        receiver_actor_ref,
        mailbox_ref,
        state,
        artifact_refs_json,
        envelope_storage_path,
        envelope_content_hash,
    ) in &handoff_projection_rows
    {
        let subrun = subruns_by_handoff_ref
            .get(handoff_ref.as_str())
            .expect("subrun for handoff projection");
        let handoff = handoffs_by_ref
            .get(handoff_ref.as_str())
            .expect("handoff summary for projection");
        assert_eq!(parent_run_id.as_deref(), Some(run.id.as_str()));
        assert_eq!(
            delegated_by_tool_call_id.as_deref(),
            subrun.delegated_by_tool_call_id.as_deref()
        );
        assert_eq!(sender_actor_ref, &subrun.actor_ref);
        assert_eq!(receiver_actor_ref, &run.actor_ref);
        assert_eq!(
            mailbox_ref,
            subrun.mailbox_ref.as_deref().unwrap_or_default()
        );
        assert_eq!(state, &handoff.state);
        let artifact_refs: Vec<String> =
            serde_json::from_str(artifact_refs_json).expect("handoff artifact refs json");
        assert_eq!(&artifact_refs, &handoff.artifact_refs);
        assert!(envelope_storage_path
            .as_deref()
            .is_some_and(|path| root.join(path).exists()));
        assert!(envelope_content_hash
            .as_deref()
            .is_some_and(|hash| hash.starts_with("sha256-")));
    }
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
    let mailbox_body_path = root.join(
        mailbox_projection
            .0
            .clone()
            .expect("mailbox projection body path"),
    );
    let mailbox_body: serde_json::Value =
        serde_json::from_slice(&fs::read(&mailbox_body_path).expect("mailbox body bytes"))
            .expect("mailbox body json");
    let mailbox_handoffs = mailbox_body
        .get("handoffs")
        .and_then(serde_json::Value::as_array)
        .expect("mailbox handoff lineage array");
    assert_eq!(mailbox_handoffs.len(), detail.handoffs.len());
    for handoff_entry in mailbox_handoffs {
        let handoff_ref = handoff_entry
            .get("handoffRef")
            .and_then(serde_json::Value::as_str)
            .expect("mailbox handoff ref");
        let subrun = subruns_by_handoff_ref
            .get(handoff_ref)
            .expect("subrun for mailbox handoff");
        let handoff = handoffs_by_ref
            .get(handoff_ref)
            .expect("handoff summary for mailbox handoff");
        assert_eq!(
            handoff_entry
                .get("parentRunId")
                .and_then(serde_json::Value::as_str),
            Some(run.id.as_str())
        );
        assert_eq!(
            handoff_entry
                .get("delegatedByToolCallId")
                .and_then(serde_json::Value::as_str),
            subrun.delegated_by_tool_call_id.as_deref()
        );
        assert_eq!(
            handoff_entry
                .get("senderActorRef")
                .and_then(serde_json::Value::as_str),
            Some(subrun.actor_ref.as_str())
        );
        assert_eq!(
            handoff_entry
                .get("receiverActorRef")
                .and_then(serde_json::Value::as_str),
            Some(run.actor_ref.as_str())
        );
        assert_eq!(
            handoff_entry
                .get("mailboxRef")
                .and_then(serde_json::Value::as_str),
            subrun.mailbox_ref.as_deref()
        );
        assert_eq!(
            handoff_entry
                .get("handoffState")
                .and_then(serde_json::Value::as_str),
            Some(handoff.state.as_str())
        );
        assert_eq!(
            handoff_entry
                .get("artifactRefs")
                .and_then(serde_json::Value::as_array)
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .expect("mailbox handoff artifact refs"),
            handoff.artifact_refs
        );
    }
    assert!(background_projection
        .0
        .as_deref()
        .is_some_and(|path| root.join(path).exists()));
    assert!(background_projection
        .1
        .as_deref()
        .is_some_and(|hash| hash.starts_with("sha256-")));
    let corrupted_handoff_ref = handoff_projection_rows
        .first()
        .map(|row| row.0.clone())
        .expect("handoff projection row");
    connection
        .execute(
            "UPDATE runtime_handoff_projections
             SET summary_json = ?2
             WHERE handoff_ref = ?1",
            params![corrupted_handoff_ref, "{invalid-handoff-summary"],
        )
        .expect("corrupt handoff summary json");
    let reloaded_from_corrupt_handoff = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );
    let corrupt_handoff_detail = reloaded_from_corrupt_handoff
        .get_session(&session.summary.id)
        .await
        .expect("reload detail from handoff envelope fallback");
    assert_eq!(corrupt_handoff_detail.handoffs.len(), detail.handoffs.len());
    assert_eq!(
        corrupt_handoff_detail
            .handoffs
            .iter()
            .find(|handoff| handoff.handoff_ref == corrupted_handoff_ref)
            .expect("corrupt handoff reloaded")
            .artifact_refs,
        detail
            .handoffs
            .iter()
            .find(|handoff| handoff.handoff_ref == corrupted_handoff_ref)
            .expect("original handoff summary")
            .artifact_refs
    );
    let corrupted_handoff_envelope_path = root.join(
        handoff_projection_rows
            .first()
            .and_then(|row| row.8.clone())
            .expect("handoff envelope path"),
    );
    fs::write(
        &corrupted_handoff_envelope_path,
        b"{invalid-handoff-envelope",
    )
    .expect("corrupt handoff envelope json");
    let reloaded_from_corrupt_handoff_envelope = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );
    let corrupt_handoff_envelope_detail = reloaded_from_corrupt_handoff_envelope
        .get_session(&session.summary.id)
        .await
        .expect("reload detail from handoff projection fallback");
    assert_eq!(
        corrupt_handoff_envelope_detail.handoffs.len(),
        detail.handoffs.len()
    );
    assert_eq!(
        corrupt_handoff_envelope_detail
            .handoffs
            .iter()
            .find(|handoff| handoff.handoff_ref == corrupted_handoff_ref)
            .expect("corrupt handoff reloaded from projection row")
            .artifact_refs,
        detail
            .handoffs
            .iter()
            .find(|handoff| handoff.handoff_ref == corrupted_handoff_ref)
            .expect("original handoff summary")
            .artifact_refs
    );
    let artifact_refs = detail
        .handoffs
        .iter()
        .flat_map(|handoff| handoff.artifact_refs.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(!artifact_refs.is_empty());
    for artifact_ref in &artifact_refs {
        assert!(!artifact_ref.starts_with("artifact-"));
        let artifact_projection: (
            String,
            String,
            i64,
            String,
            String,
            Option<String>,
            Option<String>,
        ) = connection
            .query_row(
                "SELECT storage_path, content_hash, byte_size, content_type, actor_ref, parent_run_id, delegated_by_tool_call_id
                 FROM runtime_artifact_projections
                 WHERE artifact_ref = ?1",
                [artifact_ref],
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
            .expect("runtime artifact projection");
        assert!(root.join(&artifact_projection.0).exists());
        assert!(artifact_projection.1.starts_with("sha256-"));
        assert!(artifact_projection.2 > 0);
        assert_eq!(artifact_projection.3, "application/json");
        assert!(
            artifact_projection.4.starts_with("agent:")
                || artifact_projection.4.starts_with("team:")
        );
        assert_eq!(artifact_projection.5.as_deref(), Some(run.id.as_str()));
        assert!(artifact_projection.6.is_some());
    }

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
    assert_eq!(
        first_subrun_event.parent_run_id.as_deref(),
        Some(run.id.as_str())
    );
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
        Arc::new(MockRuntimeModelDriver),
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
        .is_some_and(|mailbox| mailbox.status == "failed" && mailbox.pending_count == 0));
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
async fn team_runtime_uses_manifest_mailbox_policy_and_keeps_all_worker_subruns_beyond_concurrency_ceiling(
) {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(&infra.paths.runtime_config_dir.join("workspace.json"), None);
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    for (id, name) in [
        ("agent-scheduler-leader", "Scheduler Leader"),
        ("agent-scheduler-worker-a", "Scheduler Worker A"),
        ("agent-scheduler-worker-b", "Scheduler Worker B"),
        ("agent-scheduler-worker-c", "Scheduler Worker C"),
    ] {
        connection
            .execute(
                "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    id,
                    octopus_core::DEFAULT_WORKSPACE_ID,
                    Option::<String>::None,
                    "workspace",
                    name,
                    Option::<String>::None,
                    "Cooperative",
                    serde_json::to_string(&vec!["coordination"]).expect("tags"),
                    "Handle delegated work.",
                    serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                    serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                    serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                    "Worker scheduling test agent.",
                    "active",
                    timestamp_now() as i64,
                ],
            )
            .expect("upsert scheduling agent");
    }
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, delegation_policy_json, approval_preference_json, leader_ref, member_refs, mailbox_policy_json, worker_concurrency_limit, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "team-scheduler-policy",
                octopus_core::DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Scheduler Policy Team",
                Option::<String>::None,
                "Scheduling aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Delegate the work across all available workers.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "mode": "leader-orchestrated",
                    "allowBackgroundRuns": true,
                    "allowParallelWorkers": true,
                    "maxWorkerCount": 3
                }))
                .expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                canonical_test_agent_ref("agent-scheduler-leader"),
                canonical_test_member_refs(&[
                    "agent-scheduler-worker-a",
                    "agent-scheduler-worker-b",
                    "agent-scheduler-worker-c",
                ]),
                serde_json::to_string(&json!({
                    "mode": "worker-direct",
                    "allowWorkerToWorker": true,
                    "retainMessages": true
                }))
                .expect("mailbox policy"),
                1_i64,
                "Team for scheduler and mailbox policy tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert scheduler team");
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
                "conv-scheduler-policy",
                octopus_core::DEFAULT_PROJECT_ID,
                "Scheduler Policy Session",
                "team:team-scheduler-policy",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(&session.summary.id, turn_input("Distribute the task", None))
        .await
        .expect("team run");

    assert_eq!(
        run.worker_dispatch
            .as_ref()
            .map(|dispatch| dispatch.total_subruns),
        Some(3)
    );

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("detail");
    assert_eq!(detail.subruns.len(), 3);
    assert_eq!(
        detail
            .pending_mailbox
            .as_ref()
            .map(|mailbox| mailbox.channel.as_str()),
        Some("worker-direct")
    );
    assert!(detail
        .subruns
        .iter()
        .any(|subrun| subrun.actor_ref == "agent:agent-scheduler-worker-c"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
