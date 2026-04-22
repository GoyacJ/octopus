use super::*;

#[test]
fn validate_create_project_request_requires_and_trims_resource_directory() {
    let validated = validate_create_project_request(CreateProjectRequest {
        name: "  Resource Project  ".into(),
        description: "  Resource import coverage.  ".into(),
        resource_directory: "  data/projects/resource-project/resources  ".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: None,
        manager_user_id: None,
        preset_code: None,
        assignments: None,
    })
    .expect("validated request");

    assert_eq!(validated.name, "Resource Project");
    assert_eq!(validated.description, "Resource import coverage.");
    assert_eq!(
        validated.resource_directory,
        "data/projects/resource-project/resources"
    );

    assert!(validate_create_project_request(CreateProjectRequest {
        name: "Project".into(),
        description: String::new(),
        resource_directory: "   ".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: None,
        manager_user_id: None,
        preset_code: None,
        assignments: None,
    })
    .is_err());
}

#[test]
fn validate_update_project_request_requires_status_and_resource_directory() {
    let validated = validate_update_project_request(UpdateProjectRequest {
        name: "  Resource Project  ".into(),
        description: "  Updated description.  ".into(),
        status: " archived ".into(),
        resource_directory: "  data/projects/resource-project/resources  ".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: None,
        manager_user_id: None,
        preset_code: None,
        assignments: None,
    })
    .expect("validated update");

    assert_eq!(validated.name, "Resource Project");
    assert_eq!(validated.description, "Updated description.");
    assert_eq!(validated.status, "archived");
    assert_eq!(
        validated.resource_directory,
        "data/projects/resource-project/resources"
    );

    assert!(validate_update_project_request(UpdateProjectRequest {
        name: "Project".into(),
        description: String::new(),
        status: "disabled".into(),
        resource_directory: "data/projects/resource-project/resources".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: None,
        manager_user_id: None,
        preset_code: None,
        assignments: None,
    })
    .is_err());
    assert!(validate_update_project_request(UpdateProjectRequest {
        name: "Project".into(),
        description: String::new(),
        status: "active".into(),
        resource_directory: " ".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: None,
        manager_user_id: None,
        preset_code: None,
        assignments: None,
    })
    .is_err());
}

#[test]
fn validate_create_project_request_trims_manager_preset_and_leader() {
    let validated = validate_create_project_request(CreateProjectRequest {
        name: "  Leader Project  ".into(),
        description: "  Use live inheritance.  ".into(),
        resource_directory: "  data/projects/leader-project/resources  ".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: Some("  agent-leader  ".into()),
        manager_user_id: Some("  user-manager  ".into()),
        preset_code: Some("  preset-ops  ".into()),
        assignments: None,
    })
    .expect("validated request");

    assert_eq!(validated.leader_agent_id.as_deref(), Some("agent-leader"));
    assert_eq!(validated.manager_user_id.as_deref(), Some("user-manager"));
    assert_eq!(validated.preset_code.as_deref(), Some("preset-ops"));
    assert!(validated.assignments.is_none());

    assert!(validate_create_project_request(CreateProjectRequest {
        name: "Project".into(),
        description: String::new(),
        resource_directory: "data/projects/leader-project/resources".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: Some("   ".into()),
        manager_user_id: None,
        preset_code: None,
        assignments: None,
    })
    .is_err());
}
