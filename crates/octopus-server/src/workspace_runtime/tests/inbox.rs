use super::support::*;
use super::*;

#[tokio::test]
async fn inbox_route_returns_only_current_users_project_delete_items() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let owner_session = bootstrap_owner(&state).await;
    let approver_session = create_user_session(&state, "inbox-approver", "Inbox Approver").await;
    let app = crate::routes::build_router(state.clone());

    let project = state
        .services
        .workspace
        .create_project(CreateProjectRequest {
            name: "Inbox Scoped Delete Project".into(),
            description: "Targeted inbox route coverage.".into(),
            resource_directory: "data/projects/inbox-scoped-delete-project/resources".into(),
            owner_user_id: None,
            member_user_ids: None,
            permission_overrides: None,
            linked_workspace_assets: None,
            leader_agent_id: None,
            manager_user_id: None,
            preset_code: None,
            assignments: None,
        })
        .await
        .expect("created project");
    let mut archive_request = update_request_from_project(project.clone());
    archive_request.status = "archived".into();
    state
        .services
        .workspace
        .update_project(&project.id, archive_request)
        .await
        .expect("archived project");
    let inbox_reviewer_role = state
        .services
        .access_control
        .create_role(RoleUpsertRequest {
            code: "custom.project-delete-inbox-reviewer".into(),
            name: "Project Delete Inbox Reviewer".into(),
            description: "Can review scoped project deletions and read inbox.".into(),
            status: "active".into(),
            permission_codes: vec!["project.manage".into(), "inbox.view".into()],
        })
        .await
        .expect("create inbox reviewer role");
    state
        .services
        .access_control
        .create_role_binding(RoleBindingUpsertRequest {
            role_id: inbox_reviewer_role.id,
            subject_type: "user".into(),
            subject_id: approver_session.user_id.clone(),
            effect: "allow".into(),
        })
        .await
        .expect("bind inbox reviewer role");
    state
        .services
        .access_control
        .create_data_policy(DataPolicyUpsertRequest {
            name: "inbox reviewer scope".into(),
            subject_type: "user".into(),
            subject_id: approver_session.user_id.clone(),
            resource_type: "project".into(),
            scope_type: "selected-projects".into(),
            project_ids: vec![project.id.clone()],
            tags: Vec::new(),
            classifications: Vec::new(),
            effect: "allow".into(),
        })
        .await
        .expect("create inbox reviewer policy");

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                .header("authorization", format!("Bearer {}", owner_session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&CreateProjectDeletionRequestInput {
                        reason: Some("Need targeted inbox".into()),
                    })
                    .expect("create deletion request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("create deletion request response");
    assert_eq!(create_response.status(), StatusCode::OK);

    let owner_inbox_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/inbox")
                .header("authorization", format!("Bearer {}", owner_session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("owner inbox response");
    assert_eq!(owner_inbox_response.status(), StatusCode::OK);
    let owner_inbox_body = to_bytes(owner_inbox_response.into_body(), usize::MAX)
        .await
        .expect("owner inbox body");
    let owner_items: Vec<octopus_core::InboxItemRecord> =
        serde_json::from_slice(&owner_inbox_body).expect("owner inbox json");
    assert_eq!(owner_items.len(), 1);
    assert_eq!(owner_items[0].target_user_id, owner_session.user_id);

    let approver_inbox_response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/inbox")
                .header(
                    "authorization",
                    format!("Bearer {}", approver_session.token),
                )
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("approver inbox response");
    assert_eq!(approver_inbox_response.status(), StatusCode::OK);
    let approver_inbox_body = to_bytes(approver_inbox_response.into_body(), usize::MAX)
        .await
        .expect("approver inbox body");
    let approver_items: Vec<octopus_core::InboxItemRecord> =
        serde_json::from_slice(&approver_inbox_body).expect("approver inbox json");
    assert_eq!(approver_items.len(), 1);
    assert_eq!(approver_items[0].target_user_id, approver_session.user_id);
}
