use super::*;

#[tokio::test]
async fn project_delete_request_routes_create_and_list_archived_project_requests() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let headers = auth_headers(&session.token);
    let app = crate::routes::build_router(state.clone());

    let project = state
        .services
        .workspace
        .create_project(CreateProjectRequest {
            name: "Delete Governed Project".into(),
            description: "Deletion request route coverage.".into(),
            resource_directory: "data/projects/delete-governed-project/resources".into(),
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

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&CreateProjectDeletionRequestInput {
                        reason: Some("Retired project".into()),
                    })
                    .expect("create deletion request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("create deletion request response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("create deletion request body");
    let created: ProjectDeletionRequest =
        serde_json::from_slice(&body).expect("project deletion request json");
    assert_eq!(created.project_id, project.id);
    assert_eq!(created.requested_by_user_id, session.user_id);
    assert_eq!(created.status, "pending");
    assert_eq!(created.reason.as_deref(), Some("Retired project"));
    let inbox_items = state.services.inbox.list_inbox().await.expect("list inbox");
    let inbox_item = inbox_items
        .iter()
        .find(|item| {
            item.project_id.as_deref() == Some(project.id.as_str())
                && item.item_type == "project-deletion-request"
                && item.target_user_id == session.user_id
        })
        .expect("project deletion request inbox item");
    assert_eq!(
        inbox_item.route_to.as_deref(),
        Some(
            format!(
                "/workspaces/{}/projects/{}/settings",
                DEFAULT_WORKSPACE_ID, project.id
            )
            .as_str()
        )
    );
    assert_eq!(inbox_item.action_label.as_deref(), Some("Review approval"));

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("list deletion requests response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("list deletion requests body");
    let listed: Vec<ProjectDeletionRequest> =
        serde_json::from_slice(&body).expect("project deletion request list json");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);
    assert_eq!(listed[0].status, "pending");
    assert_eq!(headers.get("x-workspace-id").is_some(), true);
}

#[tokio::test]
async fn project_delete_request_approve_route_records_reviewer_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let app = crate::routes::build_router(state.clone());

    let project = state
        .services
        .workspace
        .create_project(CreateProjectRequest {
            name: "Approve Delete Project".into(),
            description: "Deletion approval route coverage.".into(),
            resource_directory: "data/projects/approve-delete-project/resources".into(),
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

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&CreateProjectDeletionRequestInput {
                        reason: Some("Sunset flow".into()),
                    })
                    .expect("create deletion request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("create deletion request response");
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .expect("create deletion request body");
    let created: ProjectDeletionRequest =
        serde_json::from_slice(&create_body).expect("project deletion request json");

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/projects/{}/deletion-requests/{}/approve",
                    project.id, created.id
                ))
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ReviewProjectDeletionRequestInput {
                        review_comment: Some("Approved for cleanup".into()),
                    })
                    .expect("approve deletion request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("approve deletion request response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("approve deletion request body");
    let approved: ProjectDeletionRequest =
        serde_json::from_slice(&body).expect("approved deletion request json");
    assert_eq!(approved.status, "approved");
    assert_eq!(
        approved.reviewed_by_user_id.as_deref(),
        Some(session.user_id.as_str())
    );
    assert_eq!(
        approved.review_comment.as_deref(),
        Some("Approved for cleanup")
    );
    assert!(approved.reviewed_at.is_some());
}

#[tokio::test]
async fn project_delete_request_reject_route_records_reviewer_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let app = crate::routes::build_router(state.clone());

    let project = state
        .services
        .workspace
        .create_project(CreateProjectRequest {
            name: "Reject Delete Project".into(),
            description: "Deletion rejection route coverage.".into(),
            resource_directory: "data/projects/reject-delete-project/resources".into(),
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

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&CreateProjectDeletionRequestInput {
                        reason: Some("Rejected path".into()),
                    })
                    .expect("create deletion request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("create deletion request response");
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .expect("create deletion request body");
    let created: ProjectDeletionRequest =
        serde_json::from_slice(&create_body).expect("project deletion request json");

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/projects/{}/deletion-requests/{}/reject",
                    project.id, created.id
                ))
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ReviewProjectDeletionRequestInput {
                        review_comment: Some("Need to retain project history".into()),
                    })
                    .expect("reject deletion request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("reject deletion request response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("reject deletion request body");
    let rejected: ProjectDeletionRequest =
        serde_json::from_slice(&body).expect("rejected deletion request json");
    assert_eq!(rejected.status, "rejected");
    assert_eq!(
        rejected.reviewed_by_user_id.as_deref(),
        Some(session.user_id.as_str())
    );
    assert_eq!(
        rejected.review_comment.as_deref(),
        Some("Need to retain project history")
    );
    assert!(rejected.reviewed_at.is_some());
}

#[tokio::test]
async fn project_delete_request_delete_route_requires_archived_approved_project() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let app = crate::routes::build_router(state.clone());

    let project = state
        .services
        .workspace
        .create_project(CreateProjectRequest {
            name: "Delete Project Route".into(),
            description: "Deletion route guard coverage.".into(),
            resource_directory: "data/projects/delete-project-route/resources".into(),
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

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/projects/{}", project.id))
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("delete active project response");
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let mut archive_request = update_request_from_project(project.clone());
    archive_request.status = "archived".into();
    state
        .services
        .workspace
        .update_project(&project.id, archive_request)
        .await
        .expect("archived project");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/projects/{}", project.id))
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("delete archived project response");
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&CreateProjectDeletionRequestInput {
                        reason: Some("Final cleanup".into()),
                    })
                    .expect("create deletion request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("create deletion request response");
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .expect("create deletion request body");
    let created: ProjectDeletionRequest =
        serde_json::from_slice(&create_body).expect("project deletion request json");

    let approve_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/projects/{}/deletion-requests/{}/approve",
                    project.id, created.id
                ))
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ReviewProjectDeletionRequestInput {
                        review_comment: Some("Ready to delete".into()),
                    })
                    .expect("approve deletion request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("approve deletion request response");
    assert_eq!(approve_response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/projects/{}", project.id))
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("delete approved project response");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let projects = state
        .services
        .workspace
        .list_projects()
        .await
        .expect("list projects");
    assert!(!projects.iter().any(|record| record.id == project.id));
}

#[tokio::test]
async fn project_delete_request_approve_route_allows_project_scoped_admin_reviewers() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let owner_session = bootstrap_owner(&state).await;
    let approver_session = create_user_session(&state, "project-admin", "Project Admin").await;
    let app = crate::routes::build_router(state.clone());

    let project_admin_role = state
        .services
        .access_control
        .create_role(RoleUpsertRequest {
            code: "custom.project-delete-reviewer".into(),
            name: "Project Delete Reviewer".into(),
            description: "Can approve project deletion for selected projects.".into(),
            status: "active".into(),
            permission_codes: vec!["project.manage".into()],
        })
        .await
        .expect("create project admin role");
    state
        .services
        .access_control
        .create_role_binding(RoleBindingUpsertRequest {
            role_id: project_admin_role.id,
            subject_type: "user".into(),
            subject_id: approver_session.user_id.clone(),
            effect: "allow".into(),
        })
        .await
        .expect("bind project admin role");

    let project = state
        .services
        .workspace
        .create_project(CreateProjectRequest {
            name: "Scoped Admin Delete Project".into(),
            description: "Deletion approval by scoped admin.".into(),
            resource_directory: "data/projects/scoped-admin-delete-project/resources".into(),
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
    state
        .services
        .access_control
        .create_data_policy(DataPolicyUpsertRequest {
            name: "project delete reviewer scope".into(),
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
        .expect("create scoped data policy");

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
                        reason: Some("Scoped admin should review".into()),
                    })
                    .expect("create deletion request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("create deletion request response");
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .expect("create deletion request body");
    let created: ProjectDeletionRequest =
        serde_json::from_slice(&create_body).expect("project deletion request json");

    let approve_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/projects/{}/deletion-requests/{}/approve",
                    project.id, created.id
                ))
                .header(
                    "authorization",
                    format!("Bearer {}", approver_session.token),
                )
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ReviewProjectDeletionRequestInput {
                        review_comment: Some("Scoped admin approved".into()),
                    })
                    .expect("approve deletion request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("approve deletion request response");

    assert_eq!(approve_response.status(), StatusCode::OK);
    let approve_body = to_bytes(approve_response.into_body(), usize::MAX)
        .await
        .expect("approve deletion request body");
    let approved: ProjectDeletionRequest =
        serde_json::from_slice(&approve_body).expect("approved deletion request json");
    assert_eq!(
        approved.reviewed_by_user_id.as_deref(),
        Some(approver_session.user_id.as_str())
    );
    assert_eq!(approved.status, "approved");
}

