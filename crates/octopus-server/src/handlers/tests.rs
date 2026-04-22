use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::AUTHORIZATION;
    use octopus_core::{
        AccessUserPresetUpdateRequest, AccessUserUpsertRequest, AvatarUploadPayload,
        DataPolicyUpsertRequest, LoginRequest, RegisterBootstrapAdminRequest,
        RoleBindingUpsertRequest, RoleUpsertRequest, DEFAULT_WORKSPACE_ID,
    };

    use crate::test_runtime_sdk::test_server_state;

    fn avatar_payload() -> AvatarUploadPayload {
        AvatarUploadPayload {
            content_type: "image/png".into(),
            data_base64: "iVBORw0KGgo=".into(),
            file_name: "avatar.png".into(),
            byte_size: 8,
        }
    }

    fn auth_headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {token}")
                .parse()
                .expect("authorization header"),
        );
        headers
    }

    async fn bootstrap_owner(state: &ServerState) -> SessionRecord {
        state
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
                mapped_directory: None,
            })
            .await
            .expect("bootstrap admin")
            .session
    }

    #[tokio::test]
    async fn access_experience_reports_progressive_summary_and_section_grants() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;

        state
            .services
            .access_control
            .create_role(RoleUpsertRequest {
                code: "custom.member-operator".into(),
                name: "Member Operator".into(),
                description: "Custom access role".into(),
                status: "active".into(),
                permission_codes: vec!["access.users.read".into()],
            })
            .await
            .expect("create custom role");
        state
            .services
            .access_control
            .create_data_policy(DataPolicyUpsertRequest {
                name: "confidential".into(),
                subject_type: "user".into(),
                subject_id: session.user_id.clone(),
                resource_type: "resource".into(),
                scope_type: "tag-match".into(),
                project_ids: Vec::new(),
                tags: vec!["confidential".into()],
                classifications: Vec::new(),
                effect: "allow".into(),
            })
            .await
            .expect("create data policy");
        state
            .services
            .access_control
            .upsert_menu_policy(
                "menu-workspace-access-control-users",
                MenuPolicyUpsertRequest {
                    enabled: true,
                    order: 210,
                    group: None,
                    visibility: "hidden".into(),
                },
            )
            .await
            .expect("hide access users menu");

        let Json(payload) =
            get_access_experience(State(state.clone()), auth_headers(&session.token))
                .await
                .expect("access experience");

        assert_eq!(payload.summary.experience_level, "enterprise");
        assert_eq!(payload.summary.member_count, 1);
        assert!(payload.summary.has_custom_roles);
        assert!(payload.summary.has_advanced_policies);
        assert!(payload.summary.has_menu_governance);
        assert_eq!(payload.summary.recommended_landing_section, "access");
        assert_eq!(payload.counts.custom_role_count, 1);
        assert_eq!(payload.counts.data_policy_count, 1);
        assert_eq!(payload.counts.menu_policy_count, 1);
        assert_eq!(payload.role_templates.len(), 5);
        assert_eq!(payload.role_presets.len(), 5);
        assert_eq!(payload.capability_bundles.len(), 5);
        assert!(
            payload
                .section_grants
                .iter()
                .any(|grant| grant.section == "members" && grant.allowed),
            "section grants should not be removed by hidden menu policy"
        );
        assert!(
            payload
                .section_grants
                .iter()
                .any(|grant| grant.section == "governance" && grant.allowed),
            "owner should retain governance access"
        );
    }

    #[tokio::test]
    async fn access_experience_recommends_access_for_personal_workspaces() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;

        let Json(payload) =
            get_access_experience(State(state.clone()), auth_headers(&session.token))
                .await
                .expect("access experience");

        assert_eq!(payload.summary.experience_level, "personal");
        assert_eq!(payload.summary.member_count, 1);
        assert_eq!(payload.summary.recommended_landing_section, "access");
    }

    #[tokio::test]
    async fn access_experience_recommends_members_for_team_workspaces() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;

        state
            .services
            .access_control
            .create_user(AccessUserUpsertRequest {
                username: "member".into(),
                display_name: "Member".into(),
                status: "active".into(),
                password: Some("password123".into()),
                confirm_password: Some("password123".into()),
                reset_password: Some(false),
            })
            .await
            .expect("create member");

        let Json(payload) =
            get_access_experience(State(state.clone()), auth_headers(&session.token))
                .await
                .expect("access experience");

        assert_eq!(payload.summary.experience_level, "team");
        assert_eq!(payload.summary.member_count, 2);
        assert_eq!(payload.summary.recommended_landing_section, "members");
    }

    #[tokio::test]
    async fn access_experience_recommends_governance_for_governance_only_enterprise_workspaces() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let owner = bootstrap_owner(&state).await;

        let governance_user = state
            .services
            .access_control
            .create_user(AccessUserUpsertRequest {
                username: "auditor".into(),
                display_name: "Auditor".into(),
                status: "active".into(),
                password: Some("password123".into()),
                confirm_password: Some("password123".into()),
                reset_password: Some(false),
            })
            .await
            .expect("create governance user");

        let governance_role = state
            .services
            .access_control
            .create_role(RoleUpsertRequest {
                code: "custom.governance-reader".into(),
                name: "Governance Reader".into(),
                description: "Read governance only".into(),
                status: "active".into(),
                permission_codes: vec!["access.org.read".into(), "audit.read".into()],
            })
            .await
            .expect("create governance role");
        state
            .services
            .access_control
            .create_role_binding(RoleBindingUpsertRequest {
                role_id: governance_role.id,
                subject_type: "user".into(),
                subject_id: governance_user.id.clone(),
                effect: "allow".into(),
            })
            .await
            .expect("bind governance role");
        state
            .services
            .access_control
            .create_data_policy(DataPolicyUpsertRequest {
                name: "owner-confidential".into(),
                subject_type: "user".into(),
                subject_id: owner.user_id.clone(),
                resource_type: "resource".into(),
                scope_type: "tag-match".into(),
                project_ids: Vec::new(),
                tags: vec!["confidential".into()],
                classifications: Vec::new(),
                effect: "allow".into(),
            })
            .await
            .expect("create enterprise signal");

        let governance_session = state
            .services
            .auth
            .login(LoginRequest {
                client_app_id: "octopus-desktop".into(),
                username: "auditor".into(),
                password: "password123".into(),
                workspace_id: Some(DEFAULT_WORKSPACE_ID.into()),
            })
            .await
            .expect("login governance user")
            .session;

        let Json(payload) = get_access_experience(
            State(state.clone()),
            auth_headers(&governance_session.token),
        )
        .await
        .expect("access experience");

        assert_eq!(payload.summary.experience_level, "enterprise");
        assert_eq!(payload.summary.recommended_landing_section, "governance");
        assert!(payload
            .section_grants
            .iter()
            .any(|grant| grant.section == "governance" && grant.allowed));
        assert!(payload
            .section_grants
            .iter()
            .all(|grant| grant.section == "governance" || !grant.allowed));
    }

    #[tokio::test]
    async fn access_experience_does_not_grant_access_without_member_directory_read() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let owner = bootstrap_owner(&state).await;

        let access_user = state
            .services
            .access_control
            .create_user(AccessUserUpsertRequest {
                username: "role-reader".into(),
                display_name: "Role Reader".into(),
                status: "active".into(),
                password: Some("password123".into()),
                confirm_password: Some("password123".into()),
                reset_password: Some(false),
            })
            .await
            .expect("create role reader");

        let access_role = state
            .services
            .access_control
            .create_role(RoleUpsertRequest {
                code: "custom.role-reader".into(),
                name: "Role Reader".into(),
                description: "Read access roles only".into(),
                status: "active".into(),
                permission_codes: vec!["access.roles.read".into()],
            })
            .await
            .expect("create role reader role");
        state
            .services
            .access_control
            .create_role_binding(RoleBindingUpsertRequest {
                role_id: access_role.id,
                subject_type: "user".into(),
                subject_id: access_user.id.clone(),
                effect: "allow".into(),
            })
            .await
            .expect("bind role reader role");
        state
            .services
            .access_control
            .create_data_policy(DataPolicyUpsertRequest {
                name: "owner confidential".into(),
                subject_type: "user".into(),
                subject_id: owner.user_id,
                resource_type: "resource".into(),
                scope_type: "tag-match".into(),
                project_ids: Vec::new(),
                tags: vec!["confidential".into()],
                classifications: Vec::new(),
                effect: "allow".into(),
            })
            .await
            .expect("create governance signal");

        let access_session = state
            .services
            .auth
            .login(LoginRequest {
                client_app_id: "octopus-desktop".into(),
                username: "role-reader".into(),
                password: "password123".into(),
                workspace_id: Some(DEFAULT_WORKSPACE_ID.into()),
            })
            .await
            .expect("login role reader")
            .session;

        let Json(payload) =
            get_access_experience(State(state.clone()), auth_headers(&access_session.token))
                .await
                .expect("access experience");

        assert!(payload
            .section_grants
            .iter()
            .any(|grant| grant.section == "access" && !grant.allowed));
    }

    #[test]
    fn system_menu_definitions_include_project_tasks_entry() {
        let menu = system_menu_definitions()
            .into_iter()
            .find(|record| record.id == "menu-project-tasks")
            .expect("project tasks menu entry");

        assert_eq!(menu.route_name.as_deref(), Some("project-tasks"));
        assert_eq!(menu.feature_code, "feature:project-tasks");
    }

    #[tokio::test]
    async fn access_members_lists_progressive_member_summaries() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;

        let member = state
            .services
            .access_control
            .create_user(AccessUserUpsertRequest {
                username: "member".into(),
                display_name: "Member".into(),
                status: "active".into(),
                password: Some("password123".into()),
                confirm_password: Some("password123".into()),
                reset_password: Some(false),
            })
            .await
            .expect("create member");
        state
            .services
            .access_control
            .create_role_binding(RoleBindingUpsertRequest {
                role_id: "system.member".into(),
                subject_type: "user".into(),
                subject_id: member.id.clone(),
                effect: "allow".into(),
            })
            .await
            .expect("bind member role");

        let Json(payload) = list_access_members(State(state.clone()), auth_headers(&session.token))
            .await
            .expect("list access members");

        let member_summary = payload
            .iter()
            .find(|record| record.user.id == member.id)
            .expect("member summary");
        assert_eq!(member_summary.primary_preset_code, Some("member".into()));
        assert_eq!(member_summary.primary_preset_name, "Member");
        assert!(member_summary
            .effective_role_names
            .iter()
            .any(|name| name == "Member"));

        let unassigned_member = state
            .services
            .access_control
            .create_user(AccessUserUpsertRequest {
                username: "unassigned".into(),
                display_name: "Unassigned".into(),
                status: "active".into(),
                password: Some("password123".into()),
                confirm_password: Some("password123".into()),
                reset_password: Some(false),
            })
            .await
            .expect("create unassigned member");

        let Json(unassigned_payload) =
            list_access_members(State(state.clone()), auth_headers(&session.token))
                .await
                .expect("list access members");

        let unassigned_summary = unassigned_payload
            .iter()
            .find(|record| record.user.id == unassigned_member.id)
            .expect("unassigned summary");
        assert_eq!(unassigned_summary.primary_preset_code, None);
        assert_eq!(unassigned_summary.primary_preset_name, "No preset assigned");
    }

    #[tokio::test]
    async fn update_access_user_preset_replaces_direct_system_bindings_and_preserves_other_access()
    {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;

        let member = state
            .services
            .access_control
            .create_user(AccessUserUpsertRequest {
                username: "operator".into(),
                display_name: "Operator".into(),
                status: "active".into(),
                password: Some("password123".into()),
                confirm_password: Some("password123".into()),
                reset_password: Some(false),
            })
            .await
            .expect("create operator");
        let custom_role = state
            .services
            .access_control
            .create_role(RoleUpsertRequest {
                code: "custom.member-helper".into(),
                name: "Member Helper".into(),
                description: "custom role".into(),
                status: "active".into(),
                permission_codes: vec!["access.users.read".into()],
            })
            .await
            .expect("create custom role");
        state
            .services
            .access_control
            .create_role_binding(RoleBindingUpsertRequest {
                role_id: "system.viewer".into(),
                subject_type: "user".into(),
                subject_id: member.id.clone(),
                effect: "allow".into(),
            })
            .await
            .expect("bind viewer");
        state
            .services
            .access_control
            .create_role_binding(RoleBindingUpsertRequest {
                role_id: custom_role.id.clone(),
                subject_type: "user".into(),
                subject_id: member.id.clone(),
                effect: "allow".into(),
            })
            .await
            .expect("bind custom role");

        let Json(summary) = update_access_user_preset(
            State(state.clone()),
            auth_headers(&session.token),
            Path(member.id.clone()),
            Json(AccessUserPresetUpdateRequest {
                preset_code: "admin".into(),
            }),
        )
        .await
        .expect("update preset");

        assert_eq!(summary.user.id, member.id);
        assert_eq!(summary.primary_preset_name, "Mixed access");
        assert!(summary
            .effective_role_names
            .iter()
            .any(|name| name == "Admin"));
        assert!(summary
            .effective_role_names
            .iter()
            .any(|name| name == "Member Helper"));
    }
}
