use octopus_hub::{
    contracts::TriggerSource,
    runtime::{
        ApprovalResolutionRequest, ApprovalState, AutomationCreateRequest, AutomationState,
        AutomationStateUpdateRequest, InMemoryRuntimeService, KnowledgeCandidateCreateRequest,
        KnowledgePromotionRequest, McpBinding, McpEventDeliveryRequest, TaskSubmissionRequest,
        TriggerDeliveryRequest, TriggerDeliveryState,
    },
};

fn submit_run_requiring_approval(
    runtime: &InMemoryRuntimeService,
) -> (String, String, octopus_hub::runtime::RunDetailResponse) {
    let detail = runtime.submit_task(TaskSubmissionRequest {
        workspace_id: "workspace-alpha".into(),
        project_id: "project-alpha".into(),
        title: "Review remote hub policy".into(),
        description: Some("Need approval before artifact generation".into()),
        requested_by: "operator-1".into(),
        requires_approval: true,
    });

    let run_id = detail.run.id.clone();
    let approval_id = detail
        .approval
        .as_ref()
        .expect("approval should exist")
        .id
        .clone();

    (run_id, approval_id, detail)
}

#[test]
fn submitting_without_approval_completes_with_artifact_and_no_checkpoint() {
    let runtime = InMemoryRuntimeService::default();

    let detail = runtime.submit_task(TaskSubmissionRequest {
        workspace_id: "workspace-alpha".into(),
        project_id: "project-alpha".into(),
        title: "Generate final artifact".into(),
        description: Some("Direct path without approval".into()),
        requested_by: "operator-1".into(),
        requires_approval: false,
    });

    assert_eq!(detail.run.status.as_str(), "completed");
    assert_eq!(detail.run.checkpoint_token, None);
    assert!(detail.approval.is_none());
    assert!(detail.inbox_item.is_none());
    assert_eq!(
        detail
            .artifact
            .as_ref()
            .expect("artifact should exist")
            .run_id,
        detail.run.id
    );
    assert!(detail.audit.iter().any(|entry| entry.action == "artifact.created"));
}

#[test]
fn creating_and_listing_automations_exposes_single_trigger_metadata() {
    let runtime = InMemoryRuntimeService::default();

    let created = runtime
        .create_automation(AutomationCreateRequest {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-alpha".into(),
            name: "Daily policy review".into(),
            trigger_source: TriggerSource::Cron,
            requested_by: "operator-1".into(),
            requires_approval: false,
            mcp_binding: None,
        })
        .expect("automation creation should succeed");

    assert_eq!(created.automation.workspace_id, "workspace-alpha");
    assert_eq!(created.automation.project_id, "project-alpha");
    assert_eq!(created.automation.state, AutomationState::Active);
    assert_eq!(created.automation.trigger_ids, vec![created.trigger.id.clone()]);
    assert_eq!(created.trigger.source_type, TriggerSource::Cron);
    assert!(created.latest_delivery.is_none());
    assert!(created.latest_run.is_none());

    let listed = runtime.list_automations();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].automation.id, created.automation.id);
}

#[test]
fn manual_event_delivery_creates_watch_runs_and_preserves_workspace_context() {
    let runtime = InMemoryRuntimeService::default();
    let automation = runtime
        .create_automation(AutomationCreateRequest {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-alpha".into(),
            name: "Manual drift detector".into(),
            trigger_source: TriggerSource::ManualEvent,
            requested_by: "operator-1".into(),
            requires_approval: true,
            mcp_binding: None,
        })
        .expect("automation creation should succeed");

    let delivery = runtime
        .deliver_trigger(TriggerDeliveryRequest {
            trigger_id: automation.trigger.id.clone(),
            dedupe_key: "manual-event-001".into(),
            requested_by: "operator-1".into(),
            title: Some("Investigate configuration drift".into()),
            description: Some("Needs review before artifact generation".into()),
        })
        .expect("manual delivery should create a run");

    assert_eq!(delivery.delivery.state, TriggerDeliveryState::Succeeded);

    let run_detail = delivery.run.expect("delivery should hydrate the run");
    assert_eq!(run_detail.run.run_type.as_str(), "watch");
    assert_eq!(run_detail.run.project_id, "project-alpha");
    assert_eq!(
        run_detail
            .inbox_item
            .as_ref()
            .expect("approval path should create inbox item")
            .workspace_id,
        "workspace-alpha"
    );
}

#[test]
fn repeated_trigger_delivery_reuses_the_existing_run() {
    let runtime = InMemoryRuntimeService::default();
    let automation = runtime
        .create_automation(AutomationCreateRequest {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-alpha".into(),
            name: "Nightly workspace scan".into(),
            trigger_source: TriggerSource::Cron,
            requested_by: "operator-1".into(),
            requires_approval: false,
            mcp_binding: None,
        })
        .expect("automation creation should succeed");

    let first = runtime
        .deliver_trigger(TriggerDeliveryRequest {
            trigger_id: automation.trigger.id.clone(),
            dedupe_key: "cron-2026-03-26T00:00".into(),
            requested_by: "operator-1".into(),
            title: None,
            description: Some("Scan the workspace".into()),
        })
        .expect("first delivery should succeed");
    let second = runtime
        .deliver_trigger(TriggerDeliveryRequest {
            trigger_id: automation.trigger.id.clone(),
            dedupe_key: "cron-2026-03-26T00:00".into(),
            requested_by: "operator-1".into(),
            title: None,
            description: Some("Scan the workspace".into()),
        })
        .expect("replayed delivery should reuse the first run");

    assert_eq!(first.delivery.id, second.delivery.id);
    assert_eq!(
        first.run.as_ref().expect("first run should exist").run.id,
        second.run.as_ref().expect("replayed run should exist").run.id
    );
}

#[test]
fn paused_automations_reject_delivery_and_record_failure_classification() {
    let runtime = InMemoryRuntimeService::default();
    let automation = runtime
        .create_automation(AutomationCreateRequest {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-alpha".into(),
            name: "MCP feed watcher".into(),
            trigger_source: TriggerSource::McpEvent,
            requested_by: "operator-1".into(),
            requires_approval: false,
            mcp_binding: Some(McpBinding {
                server_name: "notion".into(),
                event_name: "page.updated".into(),
            }),
        })
        .expect("automation creation should succeed");

    runtime
        .update_automation_state(
            &automation.automation.id,
            AutomationStateUpdateRequest {
                state: AutomationState::Paused,
            },
        )
        .expect("automation state should update");

    let error = runtime
        .deliver_trigger(TriggerDeliveryRequest {
            trigger_id: automation.trigger.id.clone(),
            dedupe_key: "mcp-evt-01".into(),
            requested_by: "operator-1".into(),
            title: None,
            description: Some("Replay MCP event".into()),
        })
        .expect_err("paused automations should reject delivery");

    assert!(error.to_string().contains("automation"));

    let listed = runtime.list_automations();
    let paused = listed
        .into_iter()
        .find(|entry| entry.automation.id == automation.automation.id)
        .expect("automation should still be listed");

    assert_eq!(
        paused
            .latest_delivery
            .as_ref()
            .expect("failed delivery should be recorded")
            .state,
        TriggerDeliveryState::Failed
    );
    assert!(
        paused
            .latest_delivery
            .as_ref()
            .and_then(|entry| entry.failure_reason.as_deref())
            .expect("failure reason should be recorded")
            .contains("paused")
    );
}

#[test]
fn rejects_unknown_approval_decisions_without_mutating_the_run() {
    let runtime = InMemoryRuntimeService::default();
    let (run_id, approval_id, _) = submit_run_requiring_approval(&runtime);

    let error = runtime
        .resolve_approval(
            &approval_id,
            ApprovalResolutionRequest {
                decision: "later".into(),
                reviewed_by: "reviewer-1".into(),
            },
        )
        .expect_err("unknown decisions should be rejected");

    assert_eq!(error.to_string(), "invalid approval decision: later");

    let detail = runtime.get_run(&run_id).expect("run should remain readable");
    let approval = detail.approval.expect("approval should still exist");

    assert_eq!(detail.run.status.as_str(), "waiting_approval");
    assert_eq!(approval.state, ApprovalState::Pending);
    assert_eq!(approval.reviewed_by, None);
}

#[test]
fn rejected_runs_clear_checkpoint_and_cannot_resume() {
    let runtime = InMemoryRuntimeService::default();
    let (run_id, approval_id, _) = submit_run_requiring_approval(&runtime);

    let rejected = runtime
        .resolve_approval(
            &approval_id,
            ApprovalResolutionRequest {
                decision: "rejected".into(),
                reviewed_by: "reviewer-1".into(),
            },
        )
        .expect("rejection should succeed");

    assert_eq!(rejected.run.status.as_str(), "terminated");
    assert_eq!(rejected.run.checkpoint_token, None);
    assert_eq!(
        rejected
            .approval
            .as_ref()
            .expect("approval should exist")
            .state,
        ApprovalState::Rejected
    );

    let error = runtime
        .resume_run(&run_id)
        .expect_err("terminated runs should not resume");

    assert!(
        error
            .to_string()
            .contains("resume is only allowed after approval grants a checkpoint")
    );
}

#[test]
fn repeated_approval_attempts_do_not_overwrite_the_first_resolution() {
    let runtime = InMemoryRuntimeService::default();
    let (run_id, approval_id, _) = submit_run_requiring_approval(&runtime);

    let first_resolution = runtime
        .resolve_approval(
            &approval_id,
            ApprovalResolutionRequest {
                decision: "approved".into(),
                reviewed_by: "reviewer-1".into(),
            },
        )
        .expect("first approval should succeed");

    assert_eq!(first_resolution.run.status.as_str(), "paused");
    assert_eq!(
        first_resolution
            .approval
            .as_ref()
            .expect("approval should exist")
            .state,
        ApprovalState::Approved
    );

    let error = runtime
        .resolve_approval(
            &approval_id,
            ApprovalResolutionRequest {
                decision: "rejected".into(),
                reviewed_by: "reviewer-2".into(),
            },
        )
        .expect_err("repeated resolution should fail");

    assert!(
        error
            .to_string()
            .contains("approval can only be resolved while waiting_approval")
    );

    let detail = runtime.get_run(&run_id).expect("run should still exist");
    let approval = detail.approval.expect("approval should still exist");

    assert_eq!(detail.run.status.as_str(), "paused");
    assert_eq!(approval.state, ApprovalState::Approved);
    assert_eq!(approval.reviewed_by.as_deref(), Some("reviewer-1"));
}

#[test]
fn resuming_after_approval_records_artifact_audit_against_the_artifact() {
    let runtime = InMemoryRuntimeService::default();
    let (run_id, approval_id, _) = submit_run_requiring_approval(&runtime);

    runtime
        .resolve_approval(
            &approval_id,
            ApprovalResolutionRequest {
                decision: "approved".into(),
                reviewed_by: "reviewer-1".into(),
            },
        )
        .expect("approval should succeed");

    let resumed = runtime
        .resume_run(&run_id)
        .expect("resuming a paused run should succeed");
    let artifact = resumed.artifact.expect("resume should create an artifact");

    assert_eq!(resumed.run.checkpoint_token, None);
    assert!(resumed.audit.iter().any(|entry| {
        entry.action == "artifact.created" && entry.target_ref == artifact.id
    }));
}

#[test]
fn knowledge_candidates_are_created_from_runs_and_promoted_into_shared_assets() {
    let runtime = InMemoryRuntimeService::default();

    let detail = runtime.submit_task(TaskSubmissionRequest {
        workspace_id: "workspace-alpha".into(),
        project_id: "project-alpha".into(),
        title: "Summarize workspace health".into(),
        description: Some("Artifact body for the workspace summary".into()),
        requested_by: "operator-1".into(),
        requires_approval: false,
    });

    let spaces = runtime.list_knowledge_spaces();
    assert_eq!(spaces.len(), 1);
    assert_eq!(spaces[0].space.id, "knowledge-space-alpha");

    let candidate = runtime
        .create_candidate_from_run(KnowledgeCandidateCreateRequest {
            run_id: detail.run.id.clone(),
            knowledge_space_id: "knowledge-space-alpha".into(),
            created_by: "operator-1".into(),
        })
        .expect("candidate creation should succeed");

    assert_eq!(candidate.status.as_str(), "candidate");
    assert_eq!(candidate.trust_level.as_str(), "high");
    assert_eq!(candidate.source_ref, detail.run.id);

    let assets_before = runtime
        .list_knowledge_assets("knowledge-space-alpha")
        .expect("space should exist");
    assert!(assets_before.items.is_empty());

    let promoted = runtime
        .promote_candidate(
            &candidate.id,
            KnowledgePromotionRequest {
                promoted_by: "owner-1".into(),
            },
        )
        .expect("promotion should succeed");

    assert_eq!(promoted.asset.status.as_str(), "verified_shared");
    assert_eq!(promoted.asset.source_ref, candidate.id);
    assert_eq!(promoted.candidate.status.as_str(), "verified_shared");

    let assets_after = runtime
        .list_knowledge_assets("knowledge-space-alpha")
        .expect("space should exist");
    assert_eq!(assets_after.items.len(), 1);
}

#[test]
fn mcp_event_delivery_matches_binding_and_requires_manual_knowledge_promotion() {
    let runtime = InMemoryRuntimeService::default();
    let automation = runtime
        .create_automation(AutomationCreateRequest {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-alpha".into(),
            name: "Confluence sync".into(),
            trigger_source: TriggerSource::McpEvent,
            requested_by: "operator-1".into(),
            requires_approval: false,
            mcp_binding: Some(McpBinding {
                server_name: "confluence".into(),
                event_name: "page.updated".into(),
            }),
        })
        .expect("automation creation should succeed");

    let response = runtime
        .deliver_mcp_event(McpEventDeliveryRequest {
            server_name: "confluence".into(),
            event_name: "page.updated".into(),
            dedupe_key: "evt-001".into(),
            requested_by: "operator-1".into(),
            title: Some("Confluence page updated".into()),
            description: Some("Remote page update".into()),
        })
        .expect("mcp event delivery should succeed");

    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].delivery.trigger_id, automation.trigger.id);
    let run = response.items[0].run.as_ref().expect("run should exist");
    assert_eq!(run.run.run_type.as_str(), "watch");

    let candidate = runtime
        .create_candidate_from_run(KnowledgeCandidateCreateRequest {
            run_id: run.run.id.clone(),
            knowledge_space_id: "knowledge-space-alpha".into(),
            created_by: "operator-1".into(),
        })
        .expect("candidate creation should succeed");

    assert_eq!(candidate.trust_level.as_str(), "low");

    let assets = runtime
        .list_knowledge_assets("knowledge-space-alpha")
        .expect("space should exist");
    assert!(assets.items.is_empty());
}
