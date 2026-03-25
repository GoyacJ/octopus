use octopus_hub::runtime::{
    ApprovalResolutionRequest, ApprovalState, InMemoryRuntimeService, TaskSubmissionRequest,
};

fn submit_run_requiring_approval(
    runtime: &InMemoryRuntimeService,
) -> (String, String, octopus_hub::runtime::RunDetailResponse) {
    let detail = runtime.submit_task(TaskSubmissionRequest {
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

    assert!(resumed.audit.iter().any(|entry| {
        entry.action == "artifact.created" && entry.target_ref == artifact.id
    }));
}
