use std::{env, fs, path::PathBuf, time::Duration};

use octopus_hub::{
    contracts::TriggerSource,
    runtime::{
        ApprovalResolutionRequest, AutomationCreateRequest, KnowledgeCandidateCreateRequest,
        KnowledgePromotionRequest, RuntimeService, TaskSubmissionRequest,
    },
};
use tokio::time::timeout;
use uuid::Uuid;

fn temp_db_path(name: &str) -> PathBuf {
    let path = env::temp_dir().join(format!("octopus-{name}-{}.sqlite3", Uuid::new_v4()));
    let _ = fs::remove_file(&path);
    path
}

#[test]
fn sqlite_runtime_persists_runs_automations_inbox_and_knowledge_across_restarts() {
    let database_path = temp_db_path("runtime-persistence");

    let direct_run_id = {
        let runtime = RuntimeService::sqlite(&database_path).expect("sqlite runtime should boot");

        let approval_detail = runtime.submit_task(TaskSubmissionRequest {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-alpha".into(),
            title: "Review remote hub policy".into(),
            description: Some("Need approval before artifact generation".into()),
            requested_by: "operator-1".into(),
            requires_approval: true,
        });
        let approval_id = approval_detail
            .approval
            .as_ref()
            .expect("approval should exist")
            .id
            .clone();

        let direct_detail = runtime.submit_task(TaskSubmissionRequest {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-alpha".into(),
            title: "Summarize workspace health".into(),
            description: Some("Artifact body for the workspace summary".into()),
            requested_by: "operator-1".into(),
            requires_approval: false,
        });

        runtime
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

        let candidate = runtime
            .create_candidate_from_run(KnowledgeCandidateCreateRequest {
                run_id: direct_detail.run.id.clone(),
                knowledge_space_id: "knowledge-space-alpha".into(),
                created_by: "operator-1".into(),
            })
            .expect("candidate creation should succeed");
        runtime
            .promote_candidate(
                &candidate.id,
                KnowledgePromotionRequest {
                    promoted_by: "owner-1".into(),
                },
            )
            .expect("promotion should succeed");

        runtime
            .resolve_approval(
                &approval_id,
                ApprovalResolutionRequest {
                    decision: "approved".into(),
                    reviewed_by: "reviewer-1".into(),
                },
            )
            .expect("approval resolution should succeed");

        let inbox_items = runtime.list_inbox_items(Some("workspace-alpha"));
        assert_eq!(inbox_items.len(), 1);

        let runs = runtime.list_runs(Some("workspace-alpha"), Some("project-alpha"));
        assert_eq!(runs.len(), 2);

        direct_detail.run.id
    };

    let restarted = RuntimeService::sqlite(&database_path).expect("sqlite runtime should restart");
    let direct_detail = restarted
        .get_run(&direct_run_id)
        .expect("direct run should survive restart");

    assert_eq!(direct_detail.run.status.as_str(), "completed");
    assert!(direct_detail.artifact.is_some());
    assert_eq!(
        restarted.list_knowledge_assets("knowledge-space-alpha").unwrap().items.len(),
        1
    );

    let _ = fs::remove_file(database_path);
}

#[tokio::test]
async fn sqlite_runtime_replays_persisted_events_and_streams_new_events() {
    let database_path = temp_db_path("runtime-events");
    let runtime = RuntimeService::sqlite(&database_path).expect("sqlite runtime should boot");

    let mut subscription = runtime.subscribe_events();

    let created = runtime.submit_task(TaskSubmissionRequest {
        workspace_id: "workspace-alpha".into(),
        project_id: "project-alpha".into(),
        title: "Review runtime wiring".into(),
        description: Some("Direct path without approval".into()),
        requested_by: "operator-1".into(),
        requires_approval: false,
    });

    let live_event = timeout(Duration::from_secs(1), subscription.recv())
        .await
        .expect("an event should arrive")
        .expect("subscription should stay open");
    assert_eq!(live_event.topic, "run.state_changed");
    assert_eq!(live_event.run_id.as_deref(), Some(created.run.id.as_str()));

    let persisted = runtime
        .list_events(None)
        .expect("persisted events should be readable");
    assert!(
        persisted
            .iter()
            .any(|entry| entry.topic == "run.state_changed" && entry.run_id.as_deref() == Some(created.run.id.as_str()))
    );

    let _ = fs::remove_file(database_path);
}
