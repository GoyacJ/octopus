use octopus_application::{
    CreateRunInput, InteractionResponsePayload, InteractionResponseType, ResumeRunInput,
};
use octopus_domain::{InteractionKind, RunStatus};
use octopus_infra_sqlite::SqlitePhase3Store;
use octopus_runtime::Phase3Service;

#[tokio::test]
async fn persists_waiting_run_and_deduplicates_resume_requests() {
    let store = SqlitePhase3Store::connect("sqlite::memory:").await.unwrap();
    let service = Phase3Service::new(store);

    let created = service
        .create_run(CreateRunInput {
            workspace_id: "workspace-1".to_owned(),
            agent_id: "agent-1".to_owned(),
            input: "Collect missing context for the quarterly review".to_owned(),
            interaction_type: InteractionKind::AskUser,
        })
        .await
        .unwrap();

    assert_eq!(created.status, RunStatus::WaitingInput);

    let inbox_items = service.list_inbox_items().await.unwrap();
    assert_eq!(inbox_items.len(), 1);

    let pending = &inbox_items[0];
    assert_eq!(pending.run_id, created.id);
    assert_eq!(pending.kind, InteractionKind::AskUser);

    let resumed = service
        .resume_run(
            &created.id,
            ResumeRunInput {
                inbox_item_id: Some(pending.id.clone()),
                resume_token: pending.resume_token.clone(),
                idempotency_key: "resume-1".to_owned(),
                response: InteractionResponsePayload {
                    response_type: InteractionResponseType::Text,
                    values: vec![],
                    text: Some("Use the updated OKR narrative".to_owned()),
                    approved: None,
                    goal_changed: true,
                },
            },
        )
        .await
        .unwrap();

    assert_eq!(resumed.run.status, RunStatus::Completed);
    assert!(!resumed.deduplicated);

    let duplicate = service
        .resume_run(
            &created.id,
            ResumeRunInput {
                inbox_item_id: Some(pending.id.clone()),
                resume_token: pending.resume_token.clone(),
                idempotency_key: "resume-1".to_owned(),
                response: InteractionResponsePayload {
                    response_type: InteractionResponseType::Text,
                    values: vec![],
                    text: Some("Ignored duplicate".to_owned()),
                    approved: None,
                    goal_changed: false,
                },
            },
        )
        .await
        .unwrap();

    assert_eq!(duplicate.run.status, RunStatus::Completed);
    assert!(duplicate.deduplicated);

    let timeline = service.list_run_timeline(&created.id).await.unwrap();
    assert!(timeline
        .iter()
        .any(|event| event.event_type == "run.resuming"));
    assert!(timeline
        .iter()
        .any(|event| event.event_type == "run.freshness_checked"));

    let audit = service.list_audit_events().await.unwrap();
    assert!(audit
        .iter()
        .any(|event| event.action == "run.resume.deduplicated"));
}

#[tokio::test]
async fn marks_rejected_approval_runs_as_failed() {
    let store = SqlitePhase3Store::connect("sqlite::memory:").await.unwrap();
    let service = Phase3Service::new(store);

    let created = service
        .create_run(CreateRunInput {
            workspace_id: "workspace-1".to_owned(),
            agent_id: "agent-1".to_owned(),
            input: "Approve the production deployment".to_owned(),
            interaction_type: InteractionKind::Approval,
        })
        .await
        .unwrap();

    let pending = service.list_inbox_items().await.unwrap().remove(0);

    let resumed = service
        .resume_run(
            &created.id,
            ResumeRunInput {
                inbox_item_id: Some(pending.id),
                resume_token: pending.resume_token,
                idempotency_key: "approval-1".to_owned(),
                response: InteractionResponsePayload {
                    response_type: InteractionResponseType::Approval,
                    values: vec![],
                    text: Some("Risk changed while waiting".to_owned()),
                    approved: Some(false),
                    goal_changed: true,
                },
            },
        )
        .await
        .unwrap();

    assert_eq!(resumed.run.status, RunStatus::Failed);

    let audit = service.list_audit_events().await.unwrap();
    assert!(audit
        .iter()
        .any(|event| event.action == "approval.rejected"));
}
