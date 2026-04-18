use super::adapter_test_support::*;
use super::*;

#[tokio::test]
async fn submit_turn_selects_runtime_memory_and_emits_memory_events() {
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
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the first explicit deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("First deliverable".into()),
                preview_kind: "markdown".into(),
                file_name: Some("first-deliverable.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some("# First deliverable\n\nProduce the first draft".into()),
                data_base64: None,
            }],
        }),
    );
    adapter
        .persist_runtime_memory_record(
            &memory_runtime::PersistedRuntimeMemoryRecord {
                memory_id: "mem-user-preference".into(),
                project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
                owner_ref: Some("user:user-owner".into()),
                source_run_id: Some("seed-run".into()),
                kind: "user".into(),
                scope: "user-private".into(),
                title: "user memory".into(),
                summary: "Remember the user's approval preference.".into(),
                freshness_state: "fresh".into(),
                last_validated_at: Some(1),
                proposal_state: "approved".into(),
                storage_path: None,
                content_hash: None,
                updated_at: 1,
            },
            &json!({ "summary": "Remember the user's approval preference." }),
        )
        .expect("persist runtime memory");

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-events",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Events",
                &agent_actor_ref,
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
    assert_eq!(
        run.freshness_summary
            .as_ref()
            .map(|value| value.fresh_count),
        Some(1)
    );
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
    assert!(events
        .iter()
        .any(|event| event.event_type == "memory.selected"));
    assert!(events
        .iter()
        .any(|event| event.event_type == "memory.proposed"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_filters_runtime_memory_by_actor_scope_kind_and_owner() {
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
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the first explicit deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("First deliverable".into()),
                preview_kind: "markdown".into(),
                file_name: Some("first-deliverable.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some("# First deliverable\n\nProduce the first draft".into()),
                data_base64: None,
            }],
        }),
    );
    for record in [
        memory_runtime::PersistedRuntimeMemoryRecord {
            memory_id: "mem-owned-agent".into(),
            project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
            owner_ref: Some(agent_actor_ref.clone()),
            source_run_id: Some("seed-run".into()),
            kind: "reference".into(),
            scope: "agent-private".into(),
            title: "Owned agent memory".into(),
            summary: "Provide concise implementation summaries with direct next steps.".into(),
            freshness_state: "fresh".into(),
            last_validated_at: Some(5),
            proposal_state: "approved".into(),
            storage_path: None,
            content_hash: None,
            updated_at: 5,
        },
        memory_runtime::PersistedRuntimeMemoryRecord {
            memory_id: "mem-owned-user".into(),
            project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
            owner_ref: Some("user:user-owner".into()),
            source_run_id: Some("seed-run".into()),
            kind: "user".into(),
            scope: "user-private".into(),
            title: "User preference".into(),
            summary: "The user prefers concise implementation summaries.".into(),
            freshness_state: "fresh".into(),
            last_validated_at: Some(4),
            proposal_state: "approved".into(),
            storage_path: None,
            content_hash: None,
            updated_at: 4,
        },
        memory_runtime::PersistedRuntimeMemoryRecord {
            memory_id: "mem-other-agent".into(),
            project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
            owner_ref: Some("agent:agent-other".into()),
            source_run_id: Some("seed-run".into()),
            kind: "reference".into(),
            scope: "agent-private".into(),
            title: "Foreign agent memory".into(),
            summary: "Do not expose this memory to another actor.".into(),
            freshness_state: "fresh".into(),
            last_validated_at: Some(6),
            proposal_state: "approved".into(),
            storage_path: None,
            content_hash: None,
            updated_at: 6,
        },
        memory_runtime::PersistedRuntimeMemoryRecord {
            memory_id: "mem-unknown-kind".into(),
            project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
            owner_ref: Some("user:user-owner".into()),
            source_run_id: Some("seed-run".into()),
            kind: "scratchpad".into(),
            scope: "user-private".into(),
            title: "Unsupported kind".into(),
            summary: "Unsupported runtime memory kind should not be selected.".into(),
            freshness_state: "fresh".into(),
            last_validated_at: Some(7),
            proposal_state: "approved".into(),
            storage_path: None,
            content_hash: None,
            updated_at: 7,
        },
    ] {
        adapter
            .persist_runtime_memory_record(&record, &json!({ "summary": record.summary }))
            .expect("persist runtime memory");
    }

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-selector-gating",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Selector Gating",
                &agent_actor_ref,
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
                content: "Give concise implementation summaries with the next steps.".into(),
                permission_mode: None,
                recall_mode: Some("default".into()),
                ignored_memory_ids: Vec::new(),
                memory_intent: None,
            },
        )
        .await
        .expect("run");
    let selected_ids = run
        .selected_memory
        .iter()
        .map(|item| item.memory_id.as_str())
        .collect::<Vec<_>>();
    assert!(selected_ids.contains(&"mem-owned-agent"));
    assert!(selected_ids.contains(&"mem-owned-user"));
    assert!(!selected_ids.contains(&"mem-other-agent"));
    assert!(!selected_ids.contains(&"mem-unknown-kind"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_prefers_project_memory_from_subrun_lineage_over_unrelated_branch_memory() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let team_actor_ref = builtin_team_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the first explicit deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("First deliverable".into()),
                preview_kind: "markdown".into(),
                file_name: Some("first-deliverable.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some("# First deliverable\n\nProduce the first draft".into()),
                data_base64: None,
            }],
        }),
    );
    for record in [
        memory_runtime::PersistedRuntimeMemoryRecord {
            memory_id: "mem-lineage-related".into(),
            project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
            owner_ref: Some(format!("project:{}", octopus_core::DEFAULT_PROJECT_ID)),
            source_run_id: Some("run-lineage-subrun".into()),
            kind: "project".into(),
            scope: "project-shared".into(),
            title: "Workflow checklist".into(),
            summary: "Approval reviews need the finance tag on every request.".into(),
            freshness_state: "fresh".into(),
            last_validated_at: Some(2),
            proposal_state: "approved".into(),
            storage_path: None,
            content_hash: None,
            updated_at: 2,
        },
        memory_runtime::PersistedRuntimeMemoryRecord {
            memory_id: "mem-lineage-unrelated".into(),
            project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
            owner_ref: Some(format!("project:{}", octopus_core::DEFAULT_PROJECT_ID)),
            source_run_id: Some("run-unrelated".into()),
            kind: "project".into(),
            scope: "project-shared".into(),
            title: "Workflow checklist".into(),
            summary: "Approval reviews need the finance tag on every request.".into(),
            freshness_state: "fresh".into(),
            last_validated_at: Some(20),
            proposal_state: "approved".into(),
            storage_path: None,
            content_hash: None,
            updated_at: 20,
        },
    ] {
        adapter
            .persist_runtime_memory_record(&record, &json!({ "summary": record.summary }))
            .expect("persist runtime memory");
    }

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-lineage",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Lineage",
                &team_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    {
        let mut sessions = adapter
            .state
            .sessions
            .lock()
            .expect("runtime sessions mutex");
        let aggregate = sessions
            .get_mut(&session.summary.id)
            .expect("runtime aggregate");
        let run_id = aggregate.detail.run.id.clone();
        aggregate.detail.run.parent_run_id = Some("run-lineage-parent".into());
        aggregate.detail.run.workflow_run = Some("workflow-lineage".into());
        aggregate.detail.run.workflow_run_detail = Some(RuntimeWorkflowRunDetail {
            workflow_run_id: "workflow-lineage".into(),
            status: "running".into(),
            current_step_id: Some("run-lineage-subrun".into()),
            current_step_label: Some("Worker".into()),
            total_steps: 2,
            completed_steps: 1,
            background_capable: false,
            steps: vec![
                RuntimeWorkflowStepSummary {
                    step_id: run_id.clone(),
                    node_kind: "leader".into(),
                    label: "Leader plan".into(),
                    actor_ref: aggregate.detail.run.actor_ref.clone(),
                    run_id: Some(run_id.clone()),
                    parent_run_id: Some("run-lineage-parent".into()),
                    delegated_by_tool_call_id: None,
                    mailbox_ref: None,
                    handoff_ref: None,
                    status: "completed".into(),
                    started_at: aggregate.detail.run.started_at,
                    updated_at: aggregate.detail.run.updated_at,
                },
                RuntimeWorkflowStepSummary {
                    step_id: "run-lineage-subrun".into(),
                    node_kind: "worker".into(),
                    label: "Worker".into(),
                    actor_ref: "agent:worker-runtime".into(),
                    run_id: Some("run-lineage-subrun".into()),
                    parent_run_id: Some(run_id.clone()),
                    delegated_by_tool_call_id: Some("tool-lineage".into()),
                    mailbox_ref: Some("mailbox-lineage".into()),
                    handoff_ref: Some("handoff-lineage".into()),
                    status: "completed".into(),
                    started_at: aggregate.detail.run.started_at,
                    updated_at: aggregate.detail.run.updated_at,
                },
            ],
            blocking: None,
        });
        aggregate.detail.subruns = vec![RuntimeSubrunSummary {
            run_id: "run-lineage-subrun".into(),
            parent_run_id: Some(run_id),
            actor_ref: "agent:worker-runtime".into(),
            label: "Worker".into(),
            status: "completed".into(),
            run_kind: "subrun".into(),
            delegated_by_tool_call_id: Some("tool-lineage".into()),
            workflow_run_id: Some("workflow-lineage".into()),
            mailbox_ref: Some("mailbox-lineage".into()),
            handoff_ref: Some("handoff-lineage".into()),
            started_at: aggregate.detail.run.started_at,
            updated_at: aggregate.detail.run.updated_at,
        }];
    }

    let run = adapter
        .submit_turn(
            &session.summary.id,
            SubmitRuntimeTurnInput {
                content: "Approval reviews need the finance tag on every request.".into(),
                permission_mode: None,
                recall_mode: Some("default".into()),
                ignored_memory_ids: Vec::new(),
                memory_intent: None,
            },
        )
        .await
        .expect("run");

    assert_eq!(
        run.selected_memory
            .first()
            .map(|item| item.memory_id.as_str()),
        Some("mem-lineage-related")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_rejects_memory_pollution_candidates() {
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
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the first explicit deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("First deliverable".into()),
                preview_kind: "markdown".into(),
                file_name: Some("first-deliverable.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some("# First deliverable\n\nProduce the first draft".into()),
                data_base64: None,
            }],
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-pollution",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Pollution",
                &agent_actor_ref,
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
                content: "For this task, keep the TODO list open for now.".into(),
                permission_mode: None,
                recall_mode: Some("default".into()),
                ignored_memory_ids: Vec::new(),
                memory_intent: Some("project".into()),
            },
        )
        .await
        .expect("run");

    assert!(run.pending_memory_proposal.is_none());
    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    assert!(!events
        .iter()
        .any(|event| event.event_type == "memory.proposed"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn resolving_memory_proposal_persists_runtime_memory_record_and_event() {
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
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the first explicit deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("First deliverable".into()),
                preview_kind: "markdown".into(),
                file_name: Some("first-deliverable.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some("# First deliverable\n\nProduce the first draft".into()),
                data_base64: None,
            }],
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-resolution",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Resolution",
                &agent_actor_ref,
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
    assert!(resolved.pending_memory_proposal.is_none());

    let records = adapter
        .load_runtime_memory_records(octopus_core::DEFAULT_PROJECT_ID)
        .expect("memory records");
    assert!(records.iter().any(|record| {
        record.summary == "Please remember that approval reviews need the finance tag."
            && record.proposal_state == "approved"
            && record.freshness_state == "fresh"
    }));
    assert!(
        records
            .iter()
            .find(|record| record.summary
                == "Please remember that approval reviews need the finance tag.")
            .map(|record| adapter.runtime_memory_body_path(&record.memory_id))
            .is_some_and(|path| path.exists()),
        "memory body should be persisted under data/knowledge"
    );

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    assert!(events
        .iter()
        .any(|event| event.event_type == "memory.approved"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn revalidating_existing_memory_refreshes_existing_record_in_place() {
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
            total_tokens: Some(12),
        }),
    );
    adapter
        .persist_runtime_memory_record(
            &memory_runtime::PersistedRuntimeMemoryRecord {
                memory_id: "mem-stale-feedback".into(),
                project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
                owner_ref: Some("user:runtime".into()),
                source_run_id: Some("seed-run".into()),
                kind: "feedback".into(),
                scope: "user-private".into(),
                title: "feedback memory".into(),
                summary: "Approval reviews need the finance tag on every request.".into(),
                freshness_state: "stale".into(),
                last_validated_at: Some(1),
                proposal_state: "approved".into(),
                storage_path: None,
                content_hash: None,
                updated_at: 1,
            },
            &json!({
                "kind": "feedback",
                "normalizedContent": "Approval reviews need the finance tag on every request.",
                "summary": "Approval reviews need the finance tag on every request."
            }),
        )
        .expect("seed stale memory");

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-revalidation",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Revalidation",
                &agent_actor_ref,
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
                content: "Approval reviews need the finance tag on every request.".into(),
                permission_mode: None,
                recall_mode: Some("skip".into()),
                ignored_memory_ids: Vec::new(),
                memory_intent: Some("feedback".into()),
            },
        )
        .await
        .expect("submitted");
    let proposal = submitted
        .pending_memory_proposal
        .as_ref()
        .expect("pending proposal");
    assert_eq!(proposal.memory_id, "mem-stale-feedback");

    let resolved = adapter
        .resolve_memory_proposal(
            &session.summary.id,
            &proposal.proposal_id,
            ResolveRuntimeMemoryProposalInput {
                decision: "revalidate".into(),
                note: Some("freshened".into()),
            },
        )
        .await
        .expect("resolved");
    assert!(resolved.pending_memory_proposal.is_none());

    let records = adapter
        .load_runtime_memory_records(octopus_core::DEFAULT_PROJECT_ID)
        .expect("memory records");
    let record = records
        .iter()
        .find(|record| record.memory_id == "mem-stale-feedback")
        .expect("revalidated memory record");
    assert_eq!(record.freshness_state, "revalidated");
    assert_eq!(record.proposal_state, "revalidated");

    let body: serde_json::Value = serde_json::from_slice(
        &fs::read(adapter.runtime_memory_body_path("mem-stale-feedback"))
            .expect("memory body bytes"),
    )
    .expect("memory body json");
    assert_eq!(
        body.get("normalizedContent")
            .and_then(serde_json::Value::as_str),
        Some("Approval reviews need the finance tag on every request.")
    );
    assert_eq!(
        body.get("review")
            .and_then(|value| value.get("decision"))
            .and_then(serde_json::Value::as_str),
        Some("revalidate")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn memory_proposal_mediation_targets_specific_proposal_not_durable_memory_id() {
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
            total_tokens: Some(12),
        }),
    );
    adapter
        .persist_runtime_memory_record(
            &memory_runtime::PersistedRuntimeMemoryRecord {
                memory_id: "mem-stale-feedback".into(),
                project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
                owner_ref: Some("user:runtime".into()),
                source_run_id: Some("seed-run".into()),
                kind: "feedback".into(),
                scope: "user-private".into(),
                title: "feedback memory".into(),
                summary: "Approval reviews need the finance tag on every request.".into(),
                freshness_state: "stale".into(),
                last_validated_at: Some(1),
                proposal_state: "approved".into(),
                storage_path: None,
                content_hash: None,
                updated_at: 1,
            },
            &json!({
                "kind": "feedback",
                "normalizedContent": "Approval reviews need the finance tag on every request.",
                "summary": "Approval reviews need the finance tag on every request."
            }),
        )
        .expect("seed stale memory");

    let session = adapter
        .create_session(
            session_input(
                "conv-memory-mediation-target",
                octopus_core::DEFAULT_PROJECT_ID,
                "Memory Mediation Target",
                &agent_actor_ref,
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
                content: "Approval reviews need the finance tag on every request.".into(),
                permission_mode: None,
                recall_mode: Some("skip".into()),
                ignored_memory_ids: Vec::new(),
                memory_intent: Some("feedback".into()),
            },
        )
        .await
        .expect("submitted");
    let proposal = submitted
        .pending_memory_proposal
        .as_ref()
        .expect("pending proposal");
    assert_eq!(proposal.memory_id, "mem-stale-feedback");
    assert_ne!(proposal.proposal_id, proposal.memory_id);
    assert_eq!(
        submitted
            .pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("memory-write")
    );
    assert_eq!(
        submitted
            .pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_ref.as_str()),
        Some(proposal.proposal_id.as_str())
    );

    let resolved = adapter
        .resolve_memory_proposal(
            &session.summary.id,
            &proposal.proposal_id,
            ResolveRuntimeMemoryProposalInput {
                decision: "revalidate".into(),
                note: Some("freshened".into()),
            },
        )
        .await
        .expect("resolved");
    assert_eq!(
        resolved
            .last_mediation_outcome
            .as_ref()
            .map(|outcome| outcome.target_kind.as_str()),
        Some("memory-write")
    );
    assert_eq!(
        resolved
            .last_mediation_outcome
            .as_ref()
            .map(|outcome| outcome.target_ref.as_str()),
        Some(proposal.proposal_id.as_str())
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
