    #[test]
    fn copy_workspace_skill_to_managed_persists_asset_state_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let codex_skill_dir = bundle.paths.root.join(".codex/skills/external-help");
        std::fs::create_dir_all(&codex_skill_dir).expect("codex skill dir");
        std::fs::write(
            codex_skill_dir.join("SKILL.md"),
            "---\nname: external-help\ndescription: External help skill.\n---\n",
        )
        .expect("external skill");

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let projection = runtime
            .block_on(bundle.workspace.get_capability_management_projection())
            .expect("management projection");
        let source = projection
            .entries
            .iter()
            .find(|entry| {
                entry.kind == "skill"
                    && entry.display_path == ".codex/skills/external-help/SKILL.md"
            })
            .expect("source skill");

        let copied = runtime
            .block_on(bundle.workspace.copy_workspace_skill_to_managed(
                &source.id,
                CopyWorkspaceSkillToManagedInput {
                    slug: "copied-help".into(),
                },
            ))
            .expect("copied skill");

        let asset_state = read_json_file(&bundle.paths.workspace_asset_state_path);
        assert_eq!(
            asset_state["assets"][copied.source_key.as_str()]["trusted"],
            JsonValue::Bool(true)
        );
        assert!(
            asset_state["assets"][copied.source_key.as_str()]["enabled"].is_null(),
            "managed skill should default to enabled without an explicit override"
        );
    }

    #[test]
    fn create_workspace_mcp_server_persists_asset_state_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let created = runtime
            .block_on(bundle.workspace.create_workspace_mcp_server(
                octopus_core::UpsertWorkspaceMcpServerInput {
                    server_name: "ops".into(),
                    config: serde_json::json!({
                        "transport": "stdio",
                        "command": "ops-mcp",
                        "args": ["serve"]
                    }),
                },
            ))
            .expect("created mcp");

        let asset_state = read_json_file(&bundle.paths.workspace_asset_state_path);
        assert_eq!(
            asset_state["assets"][created.source_key.as_str()]["trusted"],
            JsonValue::Bool(true)
        );
        assert!(
            asset_state["assets"][created.source_key.as_str()]["enabled"].is_null(),
            "managed mcp should default to enabled without an explicit override"
        );
    }

    #[test]
    fn copy_workspace_skill_to_managed_rewrites_frontmatter_name_to_slug() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let codex_skill_dir = bundle.paths.root.join(".codex/skills/external-help");
        std::fs::create_dir_all(&codex_skill_dir).expect("codex skill dir");
        std::fs::write(
            codex_skill_dir.join("SKILL.md"),
            "---\nname: external-help\ndescription: External help skill.\n---\n",
        )
        .expect("external skill");

        let projection = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.get_capability_management_projection())
            .expect("management projection");
        let source = projection
            .entries
            .iter()
            .find(|entry| {
                entry.kind == "skill"
                    && entry.display_path == ".codex/skills/external-help/SKILL.md"
            })
            .expect("source skill");

        let copied = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.copy_workspace_skill_to_managed(
                &source.id,
                CopyWorkspaceSkillToManagedInput {
                    slug: "copied-help".into(),
                },
            ))
            .expect("copied skill");

        assert_eq!(copied.name, "copied-help");
        assert_eq!(copied.display_path, "data/skills/copied-help/SKILL.md");
        assert!(copied.content.contains("name: copied-help"));
    }

    #[test]
    fn inbox_service_preserves_actionable_navigation_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        bundle
            .inbox
            .state
            .inbox
            .lock()
            .expect("inbox lock")
            .push(octopus_core::InboxItemRecord {
                id: "inbox-approval".into(),
                workspace_id: "ws-local".into(),
                project_id: Some("proj-redesign".into()),
                target_user_id: "user-owner".into(),
                item_type: "approval".into(),
                title: "Runtime approval pending".into(),
                description: "Runtime command needs approval.".into(),
                status: "pending".into(),
                priority: "high".into(),
                actionable: true,
                route_to: Some("/workspaces/ws-local/projects/proj-redesign/settings".into()),
                action_label: Some("Review approval".into()),
                created_at: 42,
            });

        let items = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.inbox.list_inbox())
            .expect("list inbox");

        assert_eq!(items.len(), 1);
        assert!(items[0].actionable);
        assert_eq!(
            items[0].route_to.as_deref(),
            Some("/workspaces/ws-local/projects/proj-redesign/settings")
        );
        assert_eq!(items[0].action_label.as_deref(), Some("Review approval"));
    }

    #[test]
    fn project_delete_requests_fan_out_targeted_inbox_items_and_close_remaining_reviews() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
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
                name: "Inbox Delete Project".into(),
                description: "Project deletion inbox fanout.".into(),
                resource_directory: "data/projects/inbox-delete-project/resources".into(),
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
        runtime
            .block_on(bundle.workspace.update_project(
                &project.id,
                UpdateProjectRequest {
                    name: project.name.clone(),
                    description: project.description.clone(),
                    status: "archived".into(),
                    resource_directory: project.resource_directory.clone(),
                    owner_user_id: Some(project.owner_user_id.clone()),
                    member_user_ids: Some(project.member_user_ids.clone()),
                    permission_overrides: Some(project.permission_overrides.clone()),
                    linked_workspace_assets: Some(project.linked_workspace_assets.clone()),
                    leader_agent_id: project.leader_agent_id.clone(),
                    manager_user_id: project.manager_user_id.clone(),
                    preset_code: project.preset_code.clone(),
                    assignments: project.assignments.clone(),
                },
            ))
            .expect("archive project");

        let approver = runtime
            .block_on(bundle.access_control.create_user(AccessUserUpsertRequest {
                username: "delete-approver".into(),
                display_name: "Delete Approver".into(),
                status: "active".into(),
                password: Some("password123".into()),
                confirm_password: Some("password123".into()),
                reset_password: Some(false),
            }))
            .expect("create approver");
        let outsider = runtime
            .block_on(bundle.access_control.create_user(AccessUserUpsertRequest {
                username: "delete-outsider".into(),
                display_name: "Delete Outsider".into(),
                status: "active".into(),
                password: Some("password123".into()),
                confirm_password: Some("password123".into()),
                reset_password: Some(false),
            }))
            .expect("create outsider");
        let project_admin_role = runtime
            .block_on(bundle.access_control.create_role(RoleUpsertRequest {
                code: "custom.project-delete-inbox-admin".into(),
                name: "Project Delete Inbox Admin".into(),
                description: "Can review scoped project deletions.".into(),
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
                        subject_id: approver.id.clone(),
                        effect: "allow".into(),
                    }),
            )
            .expect("bind approver role");
        runtime
            .block_on(
                bundle
                    .access_control
                    .create_role_binding(RoleBindingUpsertRequest {
                        role_id: project_admin_role.id,
                        subject_type: "user".into(),
                        subject_id: approver.id.clone(),
                        effect: "allow".into(),
                    }),
            )
            .expect("bind approver role");
        runtime
            .block_on(
                bundle
                    .access_control
                    .create_data_policy(DataPolicyUpsertRequest {
                        name: "delete approver scope".into(),
                        subject_type: "user".into(),
                        subject_id: approver.id.clone(),
                        resource_type: "project".into(),
                        scope_type: "selected-projects".into(),
                        project_ids: vec![project.id.clone()],
                        tags: Vec::new(),
                        classifications: Vec::new(),
                        effect: "allow".into(),
                    }),
            )
            .expect("create approver policy");

        let request = runtime
            .block_on(bundle.workspace.create_project_deletion_request(
                &project.id,
                &owner_session.user_id,
                CreateProjectDeletionRequestInput {
                    reason: Some("Retire this workspace project".into()),
                },
            ))
            .expect("create deletion request");

        let pending_items = runtime
            .block_on(bundle.inbox.list_inbox())
            .expect("list inbox")
            .into_iter()
            .filter(|item| {
                item.project_id.as_deref() == Some(project.id.as_str())
                    && item.item_type == "project-deletion-request"
            })
            .collect::<Vec<_>>();

        assert_eq!(pending_items.len(), 2);
        assert!(pending_items
            .iter()
            .any(|item| item.target_user_id == owner_session.user_id && item.status == "pending"));
        assert!(pending_items
            .iter()
            .any(|item| item.target_user_id == approver.id && item.status == "pending"));
        assert!(
            pending_items
                .iter()
                .all(|item| item.target_user_id != outsider.id),
            "users without project manage permission should not receive delete approval inbox items"
        );
        assert!(pending_items.iter().all(|item| {
            item.route_to.as_deref()
                == Some(
                    format!(
                        "/workspaces/{}/projects/{}/settings",
                        request.workspace_id, request.project_id
                    )
                    .as_str(),
                )
        }));
        assert!(pending_items
            .iter()
            .all(|item| item.action_label.as_deref() == Some("Review approval")));

        runtime
            .block_on(bundle.workspace.review_project_deletion_request(
                &request.id,
                &approver.id,
                true,
                ReviewProjectDeletionRequestInput {
                    review_comment: Some("Approved for deletion".into()),
                },
            ))
            .expect("approve deletion request");

        let reviewed_items = runtime
            .block_on(bundle.inbox.list_inbox())
            .expect("list inbox after review")
            .into_iter()
            .filter(|item| {
                item.project_id.as_deref() == Some(project.id.as_str())
                    && item.item_type == "project-deletion-request"
            })
            .collect::<Vec<_>>();
        let approver_item = reviewed_items
            .iter()
            .find(|item| item.target_user_id == approver.id)
            .expect("approver item");
        let owner_item = reviewed_items
            .iter()
            .find(|item| item.target_user_id == owner_session.user_id)
            .expect("owner item");

        assert_eq!(approver_item.status, "approved");
        assert!(!approver_item.actionable);
        assert_eq!(owner_item.status, "closed");
        assert!(!owner_item.actionable);
    }
