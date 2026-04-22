use super::*;
use crate::build_infra_bundle;
use octopus_core::{
    AccessUserUpsertRequest, AuthorizationRequest, AvatarUploadPayload, DataPolicyUpsertRequest,
    LoginRequest, RegisterBootstrapAdminRequest, ResourcePolicyUpsertRequest,
    RoleBindingUpsertRequest, RoleUpsertRequest, DEFAULT_PROJECT_ID,
};
use octopus_platform::{AccessControlService, AuthService, AuthorizationService, WorkspaceService};

fn avatar_payload() -> AvatarUploadPayload {
    AvatarUploadPayload {
        content_type: "image/png".into(),
        data_base64: "iVBORw0KGgo=".into(),
        file_name: "avatar.png".into(),
        byte_size: 8,
    }
}

fn bootstrap_admin(bundle: &crate::InfraBundle) -> SessionRecord {
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime
        .block_on(
            bundle
                .auth
                .register_bootstrap_admin(RegisterBootstrapAdminRequest {
                    client_app_id: "octopus-desktop".into(),
                    username: "owner".into(),
                    display_name: "Owner".into(),
                    password: "password123".into(),
                    confirm_password: "password123".into(),
                    avatar: avatar_payload(),
                    workspace_id: Some("ws-local".into()),
                    mapped_directory: None,
                }),
        )
        .expect("bootstrap admin")
        .session
}

fn create_user_session(
    bundle: &crate::InfraBundle,
    username: &str,
    display_name: &str,
) -> SessionRecord {
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        bundle
            .access_control
            .create_user(AccessUserUpsertRequest {
                username: username.into(),
                display_name: display_name.into(),
                status: "active".into(),
                password: Some("password123".into()),
                confirm_password: Some("password123".into()),
                reset_password: Some(false),
            })
            .await
            .expect("create user");

        bundle
            .auth
            .login(LoginRequest {
                client_app_id: "octopus-desktop".into(),
                username: username.into(),
                password: "password123".into(),
                workspace_id: Some("ws-local".into()),
            })
            .await
            .expect("login user")
            .session
    })
}

#[test]
fn bootstrap_admin_adopts_seeded_default_project_membership() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let session = bootstrap_admin(&bundle);

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let project = bundle
            .workspace
            .list_projects()
            .await
            .expect("projects")
            .into_iter()
            .find(|record| record.id == DEFAULT_PROJECT_ID)
            .expect("default project");

        assert_eq!(project.owner_user_id, session.user_id);
        assert!(project
            .member_user_ids
            .iter()
            .any(|user_id| user_id == &session.user_id));
        assert!(!project
            .member_user_ids
            .iter()
            .any(|user_id| user_id == "user-owner"));
    });
}

#[test]
fn default_project_seeds_model_assignments() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let project = bundle
            .workspace
            .list_projects()
            .await
            .expect("projects")
            .into_iter()
            .find(|record| record.id == DEFAULT_PROJECT_ID)
            .expect("default project");

        let models = project
            .assignments
            .as_ref()
            .and_then(|assignments| assignments.models.as_ref())
            .expect("default project model assignments");

        assert_eq!(models.default_configured_model_id, "claude-sonnet-4-5");
        assert!(models
            .configured_model_ids
            .iter()
            .any(|configured_model_id| configured_model_id == "claude-sonnet-4-5"));
    });
}

#[test]
fn loading_existing_workspace_backfills_missing_default_project_model_assignments() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");

    let db = bundle.workspace.state.open_db().expect("open db");
    db.execute(
        "UPDATE projects SET assignments_json = NULL WHERE id = ?1",
        params![DEFAULT_PROJECT_ID],
    )
    .expect("clear default project assignments");

    let reloaded = build_infra_bundle(temp.path()).expect("reloaded bundle");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let project = reloaded
            .workspace
            .list_projects()
            .await
            .expect("projects")
            .into_iter()
            .find(|record| record.id == DEFAULT_PROJECT_ID)
            .expect("default project");

        let models = project
            .assignments
            .as_ref()
            .and_then(|assignments| assignments.models.as_ref())
            .expect("backfilled default project model assignments");

        assert_eq!(models.default_configured_model_id, "claude-sonnet-4-5");
        assert!(models
            .configured_model_ids
            .iter()
            .any(|configured_model_id| configured_model_id == "claude-sonnet-4-5"));
    });
}

#[test]
fn loading_existing_workspace_backfills_placeholder_project_membership_to_owner() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let session = bootstrap_admin(&bundle);

    let db = bundle.workspace.state.open_db().expect("open db");
    db.execute(
            "UPDATE projects SET owner_user_id = 'user-owner', member_user_ids_json = '[\"user-owner\"]' WHERE id = ?1",
            params![DEFAULT_PROJECT_ID],
        )
        .expect("reset project placeholder owner");

    let reloaded = build_infra_bundle(temp.path()).expect("reloaded bundle");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let project = reloaded
            .workspace
            .list_projects()
            .await
            .expect("projects")
            .into_iter()
            .find(|record| record.id == DEFAULT_PROJECT_ID)
            .expect("default project");

        assert_eq!(project.owner_user_id, session.user_id);
        assert_eq!(project.member_user_ids, vec![session.user_id.clone()]);
    });
}

#[test]
fn bootstrap_admin_persists_requested_mapped_directory_in_workspace_summary() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let mapped_root = temp
        .path()
        .parent()
        .expect("temp parent")
        .join(format!("octopus-mapped-root-{}", uuid::Uuid::new_v4()));
    let mapped_root_string = mapped_root.to_string_lossy().to_string();

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let response = runtime
        .block_on(
            bundle
                .auth
                .register_bootstrap_admin(RegisterBootstrapAdminRequest {
                    client_app_id: "octopus-desktop".into(),
                    username: "owner".into(),
                    display_name: "Owner".into(),
                    password: "password123".into(),
                    confirm_password: "password123".into(),
                    avatar: avatar_payload(),
                    workspace_id: Some("ws-local".into()),
                    mapped_directory: Some(mapped_root_string.clone()),
                }),
        )
        .expect("bootstrap admin");

    assert_eq!(
        response.workspace.mapped_directory.as_deref(),
        Some(mapped_root_string.as_str())
    );
    assert_eq!(
        response.workspace.mapped_directory_default.as_deref(),
        Some(temp.path().to_string_lossy().as_ref())
    );

    let saved = fs::read_to_string(temp.path().join("config").join("workspace.toml"))
        .expect("workspace config");
    assert!(saved.contains("mapped_directory"));
    assert!(saved.contains(mapped_root_string.as_str()));
    assert!(mapped_root.join("data").join("main.db").exists());
    assert!(mapped_root.join("config").join("workspace.toml").exists());
    assert!(!temp.path().join("data").join("main.db").exists());

    let reloaded = build_infra_bundle(&mapped_root).expect("reloaded bundle");
    let reloaded_workspace = tokio::runtime::Runtime::new()
        .expect("reload runtime")
        .block_on(reloaded.workspace.workspace_summary())
        .expect("reloaded workspace summary");
    assert_eq!(
        reloaded_workspace.mapped_directory.as_deref(),
        Some(mapped_root_string.as_str())
    );
    assert_eq!(
        reloaded_workspace.mapped_directory_default.as_deref(),
        Some(temp.path().to_string_lossy().as_ref())
    );
}

#[test]
fn current_user_profile_returns_stored_avatar_summary() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let session = bootstrap_admin(&bundle);

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let profile = runtime
        .block_on(bundle.workspace.current_user_profile(&session.user_id))
        .expect("current profile");

    assert_eq!(profile.id, session.user_id);
    assert_eq!(
        profile.avatar.as_deref(),
        Some("data:image/png;base64,iVBORw0KGgo=")
    );
}

#[test]
fn authorization_denies_when_object_has_policies_but_subject_has_no_allow_match() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let _owner_session = bootstrap_admin(&bundle);
    let analyst_session = create_user_session(&bundle, "analyst", "Analyst");
    let reviewer_session = create_user_session(&bundle, "reviewer", "Reviewer");

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let role = bundle
            .access_control
            .create_role(RoleUpsertRequest {
                code: "tool-mcp-operator".into(),
                name: "Tool MCP Operator".into(),
                description: "Can invoke MCP tools.".into(),
                status: "active".into(),
                permission_codes: vec!["tool.mcp.invoke".into()],
            })
            .await
            .expect("create role");

        bundle
            .access_control
            .create_role_binding(RoleBindingUpsertRequest {
                role_id: role.id.clone(),
                subject_type: "user".into(),
                subject_id: analyst_session.user_id.clone(),
                effect: "allow".into(),
            })
            .await
            .expect("bind analyst");

        bundle
            .access_control
            .create_role_binding(RoleBindingUpsertRequest {
                role_id: role.id,
                subject_type: "user".into(),
                subject_id: reviewer_session.user_id.clone(),
                effect: "allow".into(),
            })
            .await
            .expect("bind reviewer");

        bundle
            .access_control
            .create_resource_policy(ResourcePolicyUpsertRequest {
                subject_type: "user".into(),
                subject_id: reviewer_session.user_id.clone(),
                resource_type: "tool.mcp".into(),
                resource_id: "mcp-prod".into(),
                action: "invoke".into(),
                effect: "allow".into(),
            })
            .await
            .expect("resource policy");

        let decision = bundle
            .authorization
            .authorize_request(
                &analyst_session,
                &AuthorizationRequest {
                    subject_id: analyst_session.user_id.clone(),
                    capability: "tool.mcp.invoke".into(),
                    project_id: None,
                    resource_type: Some("tool.mcp".into()),
                    resource_id: Some("mcp-prod".into()),
                    resource_subtype: None,
                    tags: Vec::new(),
                    classification: Some("internal".into()),
                    owner_subject_type: None,
                    owner_subject_id: None,
                },
            )
            .await
            .expect("decision");

        assert!(
            !decision.allowed,
            "object-scoped allow list should deny unmatched subject"
        );
    });
}

#[test]
fn authorization_denies_when_tag_scoped_policy_exists_but_request_misses_allow_match() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let _owner_session = bootstrap_admin(&bundle);
    let analyst_session = create_user_session(&bundle, "tag-user", "Tag User");

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
            let role = bundle
                .access_control
                .create_role(RoleUpsertRequest {
                    code: "resource-reader".into(),
                    name: "Resource Reader".into(),
                    description: "Can view protected resources.".into(),
                    status: "active".into(),
                    permission_codes: vec!["resource.view".into()],
                })
                .await
                .expect("create role");

            bundle
                .access_control
                .create_role_binding(RoleBindingUpsertRequest {
                    role_id: role.id,
                    subject_type: "user".into(),
                    subject_id: analyst_session.user_id.clone(),
                    effect: "allow".into(),
                })
                .await
                .expect("bind role");

            bundle
                .access_control
                .create_data_policy(DataPolicyUpsertRequest {
                    name: "confidential resources".into(),
                    subject_type: "user".into(),
                    subject_id: analyst_session.user_id.clone(),
                    resource_type: "resource".into(),
                    scope_type: "tag-match".into(),
                    project_ids: Vec::new(),
                    tags: vec!["confidential".into()],
                    classifications: Vec::new(),
                    effect: "allow".into(),
                })
                .await
                .expect("create data policy");

            let decision = bundle
                .authorization
                .authorize_request(
                    &analyst_session,
                    &AuthorizationRequest {
                        subject_id: analyst_session.user_id.clone(),
                        capability: "resource.view".into(),
                        project_id: Some("proj-alpha".into()),
                        resource_type: Some("resource".into()),
                        resource_id: Some("res-1".into()),
                        resource_subtype: None,
                        tags: vec!["public".into()],
                        classification: Some("internal".into()),
                        owner_subject_type: None,
                        owner_subject_id: None,
                    },
                )
                .await
                .expect("decision");

            assert!(
                !decision.allowed,
                "resource-scoped data policies should require an allow match when the domain is policy-controlled"
            );
        });
}

#[test]
fn bootstrap_admin_backfills_missing_default_owner_permissions() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let db = bundle.auth.state.open_db().expect("db");
    db.execute(
        "UPDATE access_roles SET permission_codes = ?1 WHERE id = ?2",
        params![
            serde_json::to_string(&vec!["runtime.session.read", "custom.permission",])
                .expect("owner permissions json"),
            SYSTEM_OWNER_ROLE_ID,
        ],
    )
    .expect("downgrade owner role");

    let _owner_session = bootstrap_admin(&bundle);

    let permission_codes_raw: String = db
        .query_row(
            "SELECT permission_codes FROM access_roles WHERE id = ?1",
            params![SYSTEM_OWNER_ROLE_ID],
            |row| row.get::<_, String>(0),
        )
        .expect("load owner role permissions");
    let permission_codes: Vec<String> =
        serde_json::from_str(&permission_codes_raw).expect("parse owner role permissions");

    assert!(
        permission_codes
            .iter()
            .any(|code| code == "runtime.submit_turn"),
        "bootstrap should backfill missing runtime submit permission"
    );
    assert!(
        permission_codes
            .iter()
            .any(|code| code == "custom.permission"),
        "bootstrap should preserve existing custom owner permissions"
    );
}

#[test]
fn loading_existing_workspace_backfills_missing_default_owner_permissions() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let _owner_session = bootstrap_admin(&bundle);
    let db = bundle.auth.state.open_db().expect("db");
    db.execute(
        "UPDATE access_roles SET permission_codes = ?1 WHERE id = ?2",
        params![
            serde_json::to_string(&vec!["runtime.session.read"])
                .expect("legacy owner permissions json"),
            SYSTEM_OWNER_ROLE_ID
        ],
    )
    .expect("downgrade owner role");
    drop(db);
    drop(bundle);

    let reloaded_bundle = build_infra_bundle(temp.path()).expect("reloaded bundle");
    let reloaded_db = reloaded_bundle.auth.state.open_db().expect("reloaded db");
    let permission_codes_raw: String = reloaded_db
        .query_row(
            "SELECT permission_codes FROM access_roles WHERE id = ?1",
            params![SYSTEM_OWNER_ROLE_ID],
            |row| row.get::<_, String>(0),
        )
        .expect("load reloaded owner role permissions");
    let permission_codes: Vec<String> =
        serde_json::from_str(&permission_codes_raw).expect("parse reloaded owner permissions");

    assert!(
        permission_codes
            .iter()
            .any(|code| code == "runtime.submit_turn"),
        "loading an existing workspace should backfill missing runtime submit permission"
    );
}
