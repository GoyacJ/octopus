use super::support::*;
use super::*;

#[test]
fn generic_agent_catalog_filter_excludes_pet_records() {
    let visible = vec![
        sample_agent("default", None),
        sample_agent("pet", Some("user-owner")),
    ]
    .into_iter()
    .filter(agent_visible_in_generic_catalog)
    .collect::<Vec<_>>();

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].asset_role, "default");
}

fn sample_runtime_event() -> octopus_core::RuntimeEventEnvelope {
    octopus_core::RuntimeEventEnvelope {
        id: "evt-1".into(),
        event_type: "runtime.run.updated".into(),
        workspace_id: "ws-local".into(),
        project_id: Some("project-1".into()),
        session_id: "session-1".into(),
        conversation_id: "conversation-1".into(),
        run_id: Some("run-1".into()),
        emitted_at: 20,
        sequence: 1,
        run: Some(sample_runtime_run_snapshot()),
        capability_plan_summary: Some(octopus_core::RuntimeCapabilityPlanSummary::default()),
        provider_state_summary: Some(Vec::new()),
        ..Default::default()
    }
}

#[test]
fn runtime_session_detail_transport_preserves_phase_four_fields_without_escape_hatches() {
    let json =
        runtime_transport_payload(&sample_runtime_session_detail(), "req-test").expect("json");

    assert_eq!(
        json.pointer("/workflow/workflowRunId")
            .and_then(Value::as_str),
        Some("workflow-1")
    );
    assert_eq!(
        json.pointer("/pendingMailbox/channel")
            .and_then(Value::as_str),
        Some("leader-hub")
    );
    assert_eq!(
        json.pointer("/backgroundRun/status")
            .and_then(Value::as_str),
        Some("background_running")
    );
    assert_eq!(
        json.pointer("/subruns/0/workflowRunId")
            .and_then(Value::as_str),
        Some("workflow-1")
    );
    assert_eq!(
        json.pointer("/handoffs/0/artifactRefs/0")
            .and_then(Value::as_str),
        Some("runtime-artifact-run-1")
    );
    assert_eq!(
        json.pointer("/run/workflowRunDetail/currentStepId")
            .and_then(Value::as_str),
        Some("step-1")
    );
    assert!(json.pointer("/run/checkpoint/serializedSession").is_none());
    assert!(json.pointer("/run/checkpoint/compactionMetadata").is_none());
}

#[test]
fn runtime_run_transport_preserves_phase_four_fields_without_escape_hatches() {
    let json = runtime_transport_payload(&sample_runtime_run_snapshot(), "req-test")
        .expect("runtime run json");

    assert_eq!(
        json.pointer("/workflowRun").and_then(Value::as_str),
        Some("workflow-1")
    );
    assert_eq!(
        json.pointer("/workflowRunDetail/status")
            .and_then(Value::as_str),
        Some("background_running")
    );
    assert_eq!(
        json.pointer("/mailboxRef").and_then(Value::as_str),
        Some("mailbox-1")
    );
    assert_eq!(
        json.pointer("/backgroundState").and_then(Value::as_str),
        Some("background_running")
    );
    assert_eq!(
        json.pointer("/workerDispatch/totalSubruns")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        json.pointer("/artifactRefs/0").and_then(Value::as_str),
        Some("runtime-artifact-run-1")
    );
    assert!(json.pointer("/checkpoint/serializedSession").is_none());
    assert!(json.pointer("/checkpoint/compactionMetadata").is_none());
}

#[test]
fn runtime_event_transport_drops_payload_escape_hatch() {
    let json = runtime_transport_payload(&sample_runtime_event(), "req-test").expect("json");

    assert!(json.pointer("/payload").is_none());
    assert_eq!(
        json.pointer("/run/workflowRun").and_then(Value::as_str),
        Some("workflow-1")
    );
}

#[test]
fn resource_visibility_allows_private_resources_only_for_the_owner() {
    let session = sample_session();

    assert!(resource_visibility_allows(
        &session,
        &sample_resource("public", "another-user")
    ));
    assert!(resource_visibility_allows(
        &session,
        &sample_resource("private", "user-owner")
    ));
    assert!(!resource_visibility_allows(
        &session,
        &sample_resource("private", "another-user")
    ));
}

#[test]
fn knowledge_visibility_allows_personal_records_only_for_the_owner() {
    let session = sample_session();

    assert!(knowledge_visibility_allows(
        &session,
        &sample_knowledge("workspace", "public", None)
    ));
    assert!(knowledge_visibility_allows(
        &session,
        &sample_knowledge("personal", "private", Some("user-owner"))
    ));
    assert!(!knowledge_visibility_allows(
        &session,
        &sample_knowledge("personal", "private", Some("another-user"))
    ));
}

#[test]
fn resolved_fork_target_project_id_preserves_workspace_scope() {
    assert_eq!(resolved_fork_target_project_id(None, ""), None);
    assert_eq!(resolved_fork_target_project_id(Some("   "), ""), None);
    assert_eq!(
        resolved_fork_target_project_id(Some(" project-2 "), ""),
        Some("project-2".into())
    );
}
