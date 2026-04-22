use super::{
    default_owner_permission_codes, resolve_project_deletion_approver_user_ids,
    SYSTEM_ADMIN_ROLE_ID,
};
use crate::build_infra_bundle;
use octopus_core::{
    AccessUserPresetUpdateRequest, AccessUserUpsertRequest, AvatarUploadPayload,
    CreateProjectRequest, DataPolicyUpsertRequest, OrgUnitUpsertRequest,
    RegisterBootstrapAdminRequest, ResourcePolicyUpsertRequest, RoleBindingUpsertRequest,
    RoleUpsertRequest, UserOrgAssignmentUpsertRequest,
};
use octopus_platform::{AccessControlService, AuthService, WorkspaceService};

fn avatar_payload() -> AvatarUploadPayload {
    AvatarUploadPayload {
        content_type: "image/png".into(),
        data_base64: "iVBORw0KGgo=".into(),
        file_name: "avatar.png".into(),
        byte_size: 8,
    }
}

#[test]
fn owner_permission_catalog_includes_enterprise_access_matrix() {
    let permissions = default_owner_permission_codes();

    for code in [
        "agent.debug",
        "agent.publish",
        "agent.grant",
        "resource.export",
        "resource.grant",
        "knowledge.create",
        "knowledge.edit",
        "knowledge.publish",
        "knowledge.delete",
        "knowledge.grant",
        "tool.builtin.enable",
        "tool.builtin.grant",
        "tool.skill.enable",
        "tool.skill.publish",
        "tool.skill.grant",
        "tool.mcp.enable",
        "tool.mcp.bind-credential",
        "tool.mcp.publish",
        "tool.mcp.grant",
        "runtime.submit_turn",
    ] {
        assert!(
            permissions.iter().any(|permission| permission == code),
            "missing owner permission code: {code}"
        );
    }
}

#[test]
fn access_control_system_roles_are_seeded_with_system_namespace_and_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let roles = runtime
        .block_on(bundle.access_control.list_roles())
        .expect("list roles");

    for (role_code, role_name) in [
        ("system.owner", "Owner"),
        ("system.admin", "Admin"),
        ("system.member", "Member"),
        ("system.viewer", "Viewer"),
        ("system.auditor", "Auditor"),
    ] {
        let role = roles
            .iter()
            .find(|record| record.code == role_code)
            .unwrap_or_else(|| panic!("missing system role {role_code}"));
        assert_eq!(role.id, role_code);
        assert_eq!(role.name, role_name);
        assert_eq!(role.source, "system");
        assert!(!role.editable, "system role should not be editable");
    }

    assert!(
        roles.iter().all(|role| role.code != "owner"),
        "legacy owner role should be migrated away"
    );
}

#[test]
fn access_control_reloads_legacy_owner_role_as_system_owner() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let session = runtime
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
        .session;

    let db = bundle.access_control.state.open_db().expect("db");
    db.execute(
        "UPDATE role_bindings SET role_id = 'owner', id = ?1 WHERE role_id = 'system.owner'",
        rusqlite::params![format!("binding-user-{}-owner", session.user_id)],
    )
    .expect("rewrite role binding");
    db.execute(
        "UPDATE access_roles SET id = 'owner', code = 'owner' WHERE id = 'system.owner'",
        [],
    )
    .expect("rewrite owner role");
    drop(db);
    drop(bundle);

    let reloaded = build_infra_bundle(temp.path()).expect("reloaded bundle");
    let roles = runtime
        .block_on(reloaded.access_control.list_roles())
        .expect("list roles");
    let owner_role = roles
        .iter()
        .find(|record| record.id == "system.owner")
        .expect("system owner role");
    assert_eq!(owner_role.code, "system.owner");
    assert_eq!(owner_role.source, "system");

    let reloaded_db = reloaded
        .access_control
        .state
        .open_db()
        .expect("reloaded db");
    let binding_role_id: String = reloaded_db
        .query_row(
            "SELECT role_id FROM role_bindings WHERE subject_type = 'user' AND subject_id = ?1 LIMIT 1",
            rusqlite::params![session.user_id],
            |row| row.get(0),
        )
        .expect("load migrated role binding");
    assert_eq!(binding_role_id, "system.owner");
    let legacy_owner_count: i64 = reloaded_db
        .query_row(
            "SELECT COUNT(*) FROM access_roles WHERE id = 'owner' OR code = 'owner'",
            [],
            |row| row.get(0),
        )
        .expect("count legacy owner role");
    assert_eq!(legacy_owner_count, 0);
}

#[test]
fn access_control_custom_role_flows_reject_reserved_system_namespace() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");

    let create_error = runtime
        .block_on(bundle.access_control.create_role(RoleUpsertRequest {
            code: "system.custom".into(),
            name: "System Custom".into(),
            description: "should be rejected".into(),
            status: "active".into(),
            permission_codes: vec!["workspace.overview.read".into()],
        }))
        .expect_err("system.* create should fail");
    assert!(
        create_error.to_string().contains("system."),
        "unexpected create error: {create_error}"
    );

    let custom_role = runtime
        .block_on(bundle.access_control.create_role(RoleUpsertRequest {
            code: "custom.member-helper".into(),
            name: "Member Helper".into(),
            description: "custom role".into(),
            status: "active".into(),
            permission_codes: vec!["access.users.read".into()],
        }))
        .expect("create custom role");

    let update_error = runtime
        .block_on(bundle.access_control.update_role(
            &custom_role.id,
            RoleUpsertRequest {
                code: "system.viewer".into(),
                name: "Should Fail".into(),
                description: "reserved".into(),
                status: "active".into(),
                permission_codes: vec!["access.users.read".into()],
            },
        ))
        .expect_err("system.* update should fail");
    assert!(
        update_error.to_string().contains("system."),
        "unexpected update error: {update_error}"
    );
}

#[test]
fn access_control_experience_snapshot_distinguishes_advanced_and_resource_governance() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let session = runtime
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
        .session;

    runtime
        .block_on(
            bundle
                .access_control
                .create_resource_policy(ResourcePolicyUpsertRequest {
                    subject_type: "user".into(),
                    subject_id: session.user_id.clone(),
                    resource_type: "resource".into(),
                    resource_id: "res-confidential".into(),
                    action: "view".into(),
                    effect: "allow".into(),
                }),
        )
        .expect("create resource policy");

    let snapshot = runtime
        .block_on(bundle.access_control.get_experience_snapshot())
        .expect("experience snapshot");
    assert!(
        !snapshot.has_advanced_policies,
        "resource governance alone should not trigger advanced policies"
    );
    assert!(
        snapshot.has_resource_governance,
        "resource policy should trigger resource governance"
    );

    runtime
        .block_on(
            bundle
                .access_control
                .create_data_policy(DataPolicyUpsertRequest {
                    name: "confidential".into(),
                    subject_type: "user".into(),
                    subject_id: session.user_id,
                    resource_type: "resource".into(),
                    scope_type: "tag-match".into(),
                    project_ids: Vec::new(),
                    tags: vec!["confidential".into()],
                    classifications: Vec::new(),
                    effect: "allow".into(),
                }),
        )
        .expect("create advanced data policy");

    let advanced_snapshot = runtime
        .block_on(bundle.access_control.get_experience_snapshot())
        .expect("advanced experience snapshot");
    assert!(advanced_snapshot.has_advanced_policies);
}

#[test]
fn access_control_experience_snapshot_ignores_basic_project_access_policies_for_advanced_governance(
) {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let session = runtime
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
        .session;

    runtime
        .block_on(
            bundle
                .access_control
                .create_data_policy(DataPolicyUpsertRequest {
                    name: "owner project access".into(),
                    subject_type: "user".into(),
                    subject_id: session.user_id,
                    resource_type: "project".into(),
                    scope_type: "selected-projects".into(),
                    project_ids: vec!["proj-redesign".into()],
                    tags: Vec::new(),
                    classifications: Vec::new(),
                    effect: "allow".into(),
                }),
        )
        .expect("create basic project policy");

    let snapshot = runtime
        .block_on(bundle.access_control.get_experience_snapshot())
        .expect("experience snapshot");
    assert!(
        !snapshot.has_advanced_policies,
        "basic project access policy should not trigger advanced policies"
    );
    assert_eq!(snapshot.experience_level, "personal");
}

#[test]
fn access_control_member_summaries_report_primary_presets_and_org_participation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
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
        .expect("bootstrap admin");

    let member = runtime
        .block_on(bundle.access_control.create_user(AccessUserUpsertRequest {
            username: "member".into(),
            display_name: "Member".into(),
            status: "active".into(),
            password: Some("password123".into()),
            confirm_password: Some("password123".into()),
            reset_password: Some(false),
        }))
        .expect("create member");

    runtime
        .block_on(bundle.access_control.create_org_unit(OrgUnitUpsertRequest {
            parent_id: Some("org-root".into()),
            code: "engineering".into(),
            name: "Engineering".into(),
            status: "active".into(),
        }))
        .expect("create org unit");
    runtime
        .block_on(bundle.access_control.upsert_user_org_assignment(
            UserOrgAssignmentUpsertRequest {
                user_id: member.id.clone(),
                org_unit_id: "org-engineering".into(),
                is_primary: true,
                position_ids: Vec::new(),
                user_group_ids: Vec::new(),
            },
        ))
        .expect("assign member org unit");
    runtime
        .block_on(
            bundle
                .access_control
                .create_role_binding(RoleBindingUpsertRequest {
                    role_id: "system.member".into(),
                    subject_type: "user".into(),
                    subject_id: member.id.clone(),
                    effect: "allow".into(),
                }),
        )
        .expect("bind member preset");

    let summaries = runtime
        .block_on(bundle.access_control.list_member_summaries())
        .expect("member summaries");
    let member_summary = summaries
        .iter()
        .find(|record| record.user.id == member.id)
        .expect("member summary");
    assert_eq!(member_summary.primary_preset_code, Some("member".into()));
    assert_eq!(member_summary.primary_preset_name, "Member");
    assert!(member_summary
        .effective_role_names
        .iter()
        .any(|name| name == "Member"));
    assert!(member_summary.has_org_assignments);
}

#[test]
fn access_control_assign_user_preset_replaces_direct_system_bindings_but_preserves_custom_and_inherited_roles(
) {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
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
        .expect("bootstrap admin");

    let member = runtime
        .block_on(bundle.access_control.create_user(AccessUserUpsertRequest {
            username: "operator".into(),
            display_name: "Operator".into(),
            status: "active".into(),
            password: Some("password123".into()),
            confirm_password: Some("password123".into()),
            reset_password: Some(false),
        }))
        .expect("create operator");
    let custom_role = runtime
        .block_on(bundle.access_control.create_role(RoleUpsertRequest {
            code: "custom.member-helper".into(),
            name: "Member Helper".into(),
            description: "custom role".into(),
            status: "active".into(),
            permission_codes: vec!["access.users.read".into()],
        }))
        .expect("create custom role");

    runtime
        .block_on(bundle.access_control.create_org_unit(OrgUnitUpsertRequest {
            parent_id: Some("org-root".into()),
            code: "risk".into(),
            name: "Risk".into(),
            status: "active".into(),
        }))
        .expect("create risk org unit");
    runtime
        .block_on(bundle.access_control.upsert_user_org_assignment(
            UserOrgAssignmentUpsertRequest {
                user_id: member.id.clone(),
                org_unit_id: "org-risk".into(),
                is_primary: true,
                position_ids: Vec::new(),
                user_group_ids: Vec::new(),
            },
        ))
        .expect("assign risk org unit");
    runtime
        .block_on(
            bundle
                .access_control
                .create_role_binding(RoleBindingUpsertRequest {
                    role_id: "system.viewer".into(),
                    subject_type: "user".into(),
                    subject_id: member.id.clone(),
                    effect: "allow".into(),
                }),
        )
        .expect("bind direct viewer");
    runtime
        .block_on(
            bundle
                .access_control
                .create_role_binding(RoleBindingUpsertRequest {
                    role_id: custom_role.id.clone(),
                    subject_type: "user".into(),
                    subject_id: member.id.clone(),
                    effect: "allow".into(),
                }),
        )
        .expect("bind direct custom role");
    runtime
        .block_on(
            bundle
                .access_control
                .create_role_binding(RoleBindingUpsertRequest {
                    role_id: "system.auditor".into(),
                    subject_type: "org-unit".into(),
                    subject_id: "org-risk".into(),
                    effect: "allow".into(),
                }),
        )
        .expect("bind inherited auditor role");

    let summary = runtime
        .block_on(bundle.access_control.assign_user_preset(
            &member.id,
            AccessUserPresetUpdateRequest {
                preset_code: "admin".into(),
            },
        ))
        .expect("assign admin preset");

    assert_eq!(summary.user.id, member.id);
    assert_eq!(summary.primary_preset_code, Some("mixed".into()));
    assert_eq!(summary.primary_preset_name, "Mixed access");
    assert!(summary
        .effective_role_names
        .iter()
        .any(|name| name == "Admin"));
    assert!(summary
        .effective_role_names
        .iter()
        .any(|name| name == "Member Helper"));
    assert!(summary
        .effective_role_names
        .iter()
        .any(|name| name == "Auditor"));

    let bindings = runtime
        .block_on(bundle.access_control.list_role_bindings())
        .expect("list role bindings");
    let direct_system_bindings = bindings
        .iter()
        .filter(|binding| binding.subject_type == "user" && binding.subject_id == member.id)
        .filter(|binding| binding.role_id.starts_with("system."))
        .collect::<Vec<_>>();
    assert_eq!(direct_system_bindings.len(), 1);
    assert_eq!(direct_system_bindings[0].role_id, "system.admin");
    assert!(
        bindings.iter().any(|binding| {
            binding.subject_type == "user"
                && binding.subject_id == member.id
                && binding.role_id == custom_role.id
        }),
        "custom role binding should be preserved",
    );
    assert!(
        bindings.iter().any(|binding| {
            binding.subject_type == "org-unit"
                && binding.subject_id == "org-risk"
                && binding.role_id == "system.auditor"
        }),
        "inherited org-unit role binding should be preserved",
    );
}

#[test]
fn access_control_resolves_project_deletion_approvers_from_project_admins_and_system_roles() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = build_infra_bundle(temp.path()).expect("bundle");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let owner_session = runtime
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
        .session;

    let project = runtime
        .block_on(bundle.workspace.create_project(CreateProjectRequest {
            name: "Governed Delete Project".into(),
            description: "Project deletion approver resolution.".into(),
            resource_directory: "data/projects/governed-delete-project/resources".into(),
            owner_user_id: None,
            member_user_ids: None,
            permission_overrides: None,
            linked_workspace_assets: None,
            leader_agent_id: None,
            manager_user_id: None,
            preset_code: None,
            assignments: None,
        }))
        .expect("create project");

    let project_admin = runtime
        .block_on(bundle.access_control.create_user(AccessUserUpsertRequest {
            username: "project-admin".into(),
            display_name: "Project Admin".into(),
            status: "active".into(),
            password: Some("password123".into()),
            confirm_password: Some("password123".into()),
            reset_password: Some(false),
        }))
        .expect("create project admin");
    let system_admin = runtime
        .block_on(bundle.access_control.create_user(AccessUserUpsertRequest {
            username: "system-admin".into(),
            display_name: "System Admin".into(),
            status: "active".into(),
            password: Some("password123".into()),
            confirm_password: Some("password123".into()),
            reset_password: Some(false),
        }))
        .expect("create system admin");
    let outsider = runtime
        .block_on(bundle.access_control.create_user(AccessUserUpsertRequest {
            username: "outsider".into(),
            display_name: "Outsider".into(),
            status: "active".into(),
            password: Some("password123".into()),
            confirm_password: Some("password123".into()),
            reset_password: Some(false),
        }))
        .expect("create outsider");

    let project_admin_role = runtime
        .block_on(bundle.access_control.create_role(RoleUpsertRequest {
            code: "custom.project-delete-admin".into(),
            name: "Project Delete Admin".into(),
            description: "Can manage scoped projects.".into(),
            status: "active".into(),
            permission_codes: vec!["project.manage".into()],
        }))
        .expect("create project admin role");
    runtime
        .block_on(
            bundle
                .access_control
                .create_role_binding(RoleBindingUpsertRequest {
                    role_id: project_admin_role.id.clone(),
                    subject_type: "user".into(),
                    subject_id: project_admin.id.clone(),
                    effect: "allow".into(),
                }),
        )
        .expect("bind project admin role");
    runtime
        .block_on(
            bundle
                .access_control
                .create_role_binding(RoleBindingUpsertRequest {
                    role_id: SYSTEM_ADMIN_ROLE_ID.into(),
                    subject_type: "user".into(),
                    subject_id: system_admin.id.clone(),
                    effect: "allow".into(),
                }),
        )
        .expect("bind system admin role");
    runtime
        .block_on(
            bundle
                .access_control
                .create_data_policy(DataPolicyUpsertRequest {
                    name: "project admin scoped policy".into(),
                    subject_type: "user".into(),
                    subject_id: project_admin.id.clone(),
                    resource_type: "project".into(),
                    scope_type: "selected-projects".into(),
                    project_ids: vec![project.id.clone()],
                    tags: Vec::new(),
                    classifications: Vec::new(),
                    effect: "allow".into(),
                }),
        )
        .expect("create project admin data policy");

    let connection = bundle.access_control.state.open_db().expect("db");
    let approver_ids =
        resolve_project_deletion_approver_user_ids(&connection, &project.id).expect("ids");

    assert!(
        approver_ids
            .iter()
            .any(|user_id| user_id == &owner_session.user_id),
        "workspace owner should remain an approver"
    );
    assert!(
        approver_ids
            .iter()
            .any(|user_id| user_id == &project_admin.id),
        "scoped project manager should be resolved as an approver"
    );
    assert!(
        approver_ids
            .iter()
            .any(|user_id| user_id == &system_admin.id),
        "system admin should be resolved as an approver"
    );
    assert!(
        !approver_ids.iter().any(|user_id| user_id == &outsider.id),
        "user without project manage permission should not be treated as an approver"
    );
}
