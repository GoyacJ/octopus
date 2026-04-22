use super::*;

#[tokio::test]
async fn workspace_summary_route_returns_persisted_mapped_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let mapped_root = temp.path().to_string_lossy().to_string();

    let session = state
        .services
        .auth
        .register_bootstrap_admin(RegisterBootstrapAdminRequest {
            client_app_id: "octopus-desktop".into(),
            username: "owner".into(),
            display_name: "Owner".into(),
            password: "password123".into(),
            confirm_password: "password123".into(),
            avatar: avatar_payload(),
            workspace_id: Some(DEFAULT_WORKSPACE_ID.into()),
            mapped_directory: Some(mapped_root.clone()),
        })
        .await
        .expect("bootstrap admin")
        .session;

    let app = crate::routes::build_router(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/workspace")
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("workspace summary response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("workspace summary body");
    let workspace: WorkspaceSummary =
        serde_json::from_slice(&body).expect("workspace summary json");
    assert_eq!(
        workspace.mapped_directory.as_deref(),
        Some(mapped_root.as_str())
    );
    assert_eq!(
        workspace.mapped_directory_default.as_deref(),
        Some(mapped_root.as_str())
    );
}

#[tokio::test]
async fn workspace_summary_patch_route_updates_workspace_settings() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let current_root = temp.path().to_string_lossy().to_string();
    let app = crate::routes::build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/v1/workspace")
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&UpdateWorkspaceRequest {
                        name: Some("Workspace Rebuilt".into()),
                        avatar: None,
                        remove_avatar: Some(true),
                        mapped_directory: Some(current_root.clone()),
                    })
                    .expect("workspace update json"),
                ))
                .expect("request"),
        )
        .await
        .expect("workspace update response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("workspace update body");
    let workspace: WorkspaceSummary =
        serde_json::from_slice(&body).expect("workspace update json");
    assert_eq!(workspace.name, "Workspace Rebuilt");
    assert_eq!(
        workspace.mapped_directory.as_deref(),
        Some(current_root.as_str())
    );
}

#[tokio::test]
async fn workspace_summary_patch_route_moves_workspace_root_and_preserves_shell_root_pointer() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let mapped_root = temp
        .path()
        .parent()
        .expect("temp parent")
        .join(format!("octopus-mapped-root-{}", uuid::Uuid::new_v4()));
    let mapped_root_string = mapped_root.to_string_lossy().to_string();
    let shell_root_string = temp.path().to_string_lossy().to_string();
    let app = crate::routes::build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/v1/workspace")
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&UpdateWorkspaceRequest {
                        name: Some("Workspace Moved".into()),
                        avatar: None,
                        remove_avatar: Some(false),
                        mapped_directory: Some(mapped_root_string.clone()),
                    })
                    .expect("workspace update json"),
                ))
                .expect("request"),
        )
        .await
        .expect("workspace update response");

    assert_eq!(response.status(), StatusCode::OK);
    assert!(mapped_root.join("data").join("main.db").exists());
    assert!(!temp.path().join("data").join("main.db").exists());

    let shell_pointer = fs::read_to_string(temp.path().join("config").join("workspace.toml"))
        .expect("shell pointer workspace config");
    assert!(shell_pointer.contains(mapped_root_string.as_str()));

    let reloaded = test_server_state(&mapped_root);
    let workspace = reloaded
        .services
        .workspace
        .workspace_summary()
        .await
        .expect("reloaded workspace summary");
    assert_eq!(workspace.name, "Workspace Moved");
    assert_eq!(
        workspace.mapped_directory.as_deref(),
        Some(mapped_root_string.as_str())
    );
    assert_eq!(
        workspace.mapped_directory_default.as_deref(),
        Some(shell_root_string.as_str())
    );
}

#[tokio::test]
async fn personal_center_profile_route_returns_stored_avatar_summary() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let app = crate::routes::build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/workspace/personal-center/profile")
                .header("authorization", format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("profile response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("profile body");
    let profile: octopus_core::UserRecordSummary =
        serde_json::from_slice(&body).expect("profile json");
    assert_eq!(profile.id, session.user_id);
    assert_eq!(
        profile.avatar.as_deref(),
        Some("data:image/png;base64,iVBORw0KGgo=")
    );
}
