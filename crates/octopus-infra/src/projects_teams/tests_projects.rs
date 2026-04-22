    #[test]
    fn create_team_rejects_missing_leader_ref() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let error = runtime()
            .block_on(bundle.workspace.create_team(UpsertTeamInput {
                workspace_id: DEFAULT_WORKSPACE_ID.into(),
                project_id: None,
                scope: "workspace".into(),
                name: "Invalid Team".into(),
                avatar: None,
                remove_avatar: None,
                personality: "Missing leader ref".into(),
                tags: vec!["research".into()],
                prompt: "This should fail closed.".into(),
                builtin_tool_keys: vec!["bash".into()],
                skill_ids: Vec::new(),
                mcp_server_names: Vec::new(),
                task_domains: vec!["research".into()],
                default_model_strategy: None,
                capability_policy: None,
                permission_envelope: None,
                memory_policy: None,
                delegation_policy: None,
                approval_preference: None,
                output_contract: None,
                shared_capability_policy: None,
                leader_ref: String::new(),
                member_refs: Vec::new(),
                team_topology: None,
                shared_memory_policy: None,
                mailbox_policy: None,
                artifact_handoff_policy: None,
                workflow_affordance: None,
                worker_concurrency_limit: None,
                description: "Legacy-only team input".into(),
                status: "active".into(),
            }))
            .expect_err("missing leader_ref should fail");

        assert!(error.to_string().contains("leader_ref"));
    }

    #[test]
    fn project_resource_directory_persists_on_create() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let created = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Resource Project".into(),
                description: "Resource directory persistence.".into(),
                resource_directory: "data/projects/resource-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            }))
            .expect("created project");

        assert_eq!(
            created.resource_directory,
            "data/projects/resource-project/resources"
        );
        assert!(bundle
            .paths
            .root
            .join("data/projects/resource-project/resources")
            .exists());
    }

    #[test]
    fn create_project_persists_manager_and_preset_fields_without_legacy_grant_snapshots() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let created = runtime
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Governed Project".into(),
                description: "Project metadata persistence coverage.".into(),
                resource_directory: "data/projects/governed-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: Some("agent-leader".into()),
                manager_user_id: Some("user-manager".into()),
                preset_code: Some("preset-governed".into()),
                assignments: None,
            }))
            .expect("created project");

        assert_eq!(created.leader_agent_id.as_deref(), Some("agent-leader"));
        assert_eq!(created.manager_user_id.as_deref(), Some("user-manager"));
        assert_eq!(created.preset_code.as_deref(), Some("preset-governed"));
        assert_eq!(
            created.linked_workspace_assets,
            empty_project_linked_workspace_assets()
        );
        assert!(created.assignments.is_none());

        let listed = runtime
            .block_on(bundle.workspace.list_projects())
            .expect("listed projects");
        let persisted = listed
            .iter()
            .find(|project| project.id == created.id)
            .expect("persisted project");

        assert_eq!(persisted.leader_agent_id.as_deref(), Some("agent-leader"));
        assert_eq!(persisted.manager_user_id.as_deref(), Some("user-manager"));
        assert_eq!(persisted.preset_code.as_deref(), Some("preset-governed"));
        assert_eq!(
            persisted.linked_workspace_assets,
            empty_project_linked_workspace_assets()
        );
        assert!(persisted.assignments.is_none());
    }

    #[test]
    fn update_project_rewrites_manager_and_preset_fields_without_legacy_grant_snapshots() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let created = runtime
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Rewrite Governed Project".into(),
                description: "Rewrite project metadata persistence coverage.".into(),
                resource_directory: "data/projects/rewrite-governed-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: Some("agent-alpha".into()),
                manager_user_id: Some("user-manager-alpha".into()),
                preset_code: Some("preset-alpha".into()),
                assignments: None,
            }))
            .expect("created project");

        let updated = runtime
            .block_on(bundle.workspace.update_project(
                &created.id,
                UpdateProjectRequest {
                    name: "Rewrite Governed Project".into(),
                    description: "Rewrite project metadata persistence updated.".into(),
                    status: "active".into(),
                    resource_directory: created.resource_directory.clone(),
                    owner_user_id: None,
                    member_user_ids: None,
                    permission_overrides: None,
                    linked_workspace_assets: None,
                    leader_agent_id: Some("agent-beta".into()),
                    manager_user_id: Some("user-manager-beta".into()),
                    preset_code: Some("preset-beta".into()),
                    assignments: None,
                },
            ))
            .expect("updated project");

        assert_eq!(updated.leader_agent_id.as_deref(), Some("agent-beta"));
        assert_eq!(
            updated.manager_user_id.as_deref(),
            Some("user-manager-beta")
        );
        assert_eq!(updated.preset_code.as_deref(), Some("preset-beta"));
        assert_eq!(
            updated.linked_workspace_assets,
            empty_project_linked_workspace_assets()
        );
        assert!(updated.assignments.is_none());

        let persisted = runtime
            .block_on(bundle.workspace.list_projects())
            .expect("listed projects")
            .into_iter()
            .find(|project| project.id == created.id)
            .expect("persisted project");
        assert_eq!(persisted.leader_agent_id.as_deref(), Some("agent-beta"));
        assert_eq!(
            persisted.manager_user_id.as_deref(),
            Some("user-manager-beta")
        );
        assert_eq!(persisted.preset_code.as_deref(), Some("preset-beta"));
        assert_eq!(
            persisted.linked_workspace_assets,
            empty_project_linked_workspace_assets()
        );
        assert!(persisted.assignments.is_none());

        let connection = bundle.workspace.state.open_db().expect("open db");
        let (
            project_count,
            stored_leader_agent_id,
            stored_manager_user_id,
            stored_preset_code,
            assignments_json,
            linked_workspace_assets_json,
        ): (
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = connection
            .query_row(
                "SELECT COUNT(*), leader_agent_id, manager_user_id, preset_code, assignments_json, linked_workspace_assets_json
                 FROM projects
                 WHERE id = ?1
                 GROUP BY leader_agent_id, manager_user_id, preset_code, assignments_json, linked_workspace_assets_json",
                params![created.id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("load persisted project row");
        assert_eq!(project_count, 1);
        assert_eq!(stored_leader_agent_id.as_deref(), Some("agent-beta"));
        assert_eq!(stored_manager_user_id.as_deref(), Some("user-manager-beta"));
        assert_eq!(stored_preset_code.as_deref(), Some("preset-beta"));
        assert!(assignments_json.is_none());
        assert!(linked_workspace_assets_json.is_none());
    }

    #[test]
    fn project_deletion_requests_round_trip_and_gate_delete_until_approved() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let owner_session = bootstrap_admin_session(&bundle);

        let created = runtime
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Delete Gate Project".into(),
                description: "Project deletion request coverage.".into(),
                resource_directory: "data/projects/delete-gate-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            }))
            .expect("created project");

        let error = runtime
            .block_on(bundle.workspace.delete_project(&created.id))
            .expect_err("active project delete should fail");
        assert!(error.to_string().contains("archived"));

        let archived = runtime
            .block_on(bundle.workspace.update_project(
                &created.id,
                UpdateProjectRequest {
                    name: created.name.clone(),
                    description: created.description.clone(),
                    status: "archived".into(),
                    resource_directory: created.resource_directory.clone(),
                    owner_user_id: Some(created.owner_user_id.clone()),
                    member_user_ids: Some(created.member_user_ids.clone()),
                    permission_overrides: Some(created.permission_overrides.clone()),
                    linked_workspace_assets: Some(created.linked_workspace_assets.clone()),
                    leader_agent_id: created.leader_agent_id.clone(),
                    manager_user_id: created.manager_user_id.clone(),
                    preset_code: created.preset_code.clone(),
                    assignments: created.assignments.clone(),
                },
            ))
            .expect("archived project");
        assert_eq!(archived.status, "archived");

        let error = runtime
            .block_on(bundle.workspace.delete_project(&created.id))
            .expect_err("archived project without request should fail");
        assert!(error.to_string().contains("approved"));

        let pending = runtime
            .block_on(bundle.workspace.create_project_deletion_request(
                &created.id,
                &owner_session.user_id,
                CreateProjectDeletionRequestInput {
                    reason: Some("Archive complete".into()),
                },
            ))
            .expect("created deletion request");
        assert_eq!(pending.status, "pending");
        assert_eq!(pending.reason.as_deref(), Some("Archive complete"));

        let listed = runtime
            .block_on(bundle.workspace.list_project_deletion_requests(&created.id))
            .expect("listed deletion requests");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, pending.id);

        let error = runtime
            .block_on(bundle.workspace.delete_project(&created.id))
            .expect_err("pending request should not unlock delete");
        assert!(error.to_string().contains("approved"));

        let approved = runtime
            .block_on(bundle.workspace.review_project_deletion_request(
                &pending.id,
                &owner_session.user_id,
                true,
                ReviewProjectDeletionRequestInput {
                    review_comment: Some("Approved for deletion".into()),
                },
            ))
            .expect("approved deletion request");
        assert_eq!(approved.status, "approved");
        assert_eq!(
            approved.reviewed_by_user_id.as_deref(),
            Some(owner_session.user_id.as_str())
        );
        assert_eq!(
            approved.review_comment.as_deref(),
            Some("Approved for deletion")
        );
        assert!(approved.reviewed_at.is_some());

        runtime
            .block_on(bundle.workspace.delete_project(&created.id))
            .expect("deleted project");

        let projects = runtime
            .block_on(bundle.workspace.list_projects())
            .expect("listed projects");
        assert!(!projects.iter().any(|project| project.id == created.id));

        let connection = bundle.workspace.state.open_db().expect("open db");
        let remaining_requests: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM project_deletion_requests WHERE project_id = ?1",
                params![created.id],
                |row| row.get(0),
            )
            .expect("count project deletion requests");
        assert_eq!(remaining_requests, 0);
    }

    #[test]
    fn project_delete_removes_managed_resource_storage_and_project_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let owner_session = bootstrap_admin_session(&bundle);

        let created = runtime
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Delete Storage Project".into(),
                description: "Project delete cleanup coverage.".into(),
                resource_directory: "data/projects/delete-storage-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            }))
            .expect("created project");

        let imported = runtime
            .block_on(bundle.workspace.import_project_resource(
                &created.id,
                "user-owner",
                octopus_core::WorkspaceResourceImportInput {
                    name: "cleanup-folder".into(),
                    root_dir_name: Some("cleanup-folder".into()),
                    scope: "project".into(),
                    visibility: "private".into(),
                    tags: Some(vec!["cleanup".into()]),
                    files: vec![
                        encoded_file("notes/todo.md", "text/markdown", "# Cleanup"),
                        encoded_file("payload.json", "application/json", "{\"ok\":true}"),
                    ],
                },
            ))
            .expect("imported project resource");

        let project_root = bundle.paths.root.join(&created.resource_directory);
        let storage_path = imported.storage_path.clone().expect("storage path");
        let absolute_storage_path = bundle.paths.root.join(&storage_path);
        assert!(project_root.exists());
        assert!(absolute_storage_path.exists());

        runtime
            .block_on(bundle.workspace.update_project(
                &created.id,
                UpdateProjectRequest {
                    name: created.name.clone(),
                    description: created.description.clone(),
                    status: "archived".into(),
                    resource_directory: created.resource_directory.clone(),
                    owner_user_id: Some(created.owner_user_id.clone()),
                    member_user_ids: Some(created.member_user_ids.clone()),
                    permission_overrides: Some(created.permission_overrides.clone()),
                    linked_workspace_assets: Some(created.linked_workspace_assets.clone()),
                    leader_agent_id: created.leader_agent_id.clone(),
                    manager_user_id: created.manager_user_id.clone(),
                    preset_code: created.preset_code.clone(),
                    assignments: created.assignments.clone(),
                },
            ))
            .expect("archived project");
        let deletion_request = runtime
            .block_on(bundle.workspace.create_project_deletion_request(
                &created.id,
                &owner_session.user_id,
                CreateProjectDeletionRequestInput {
                    reason: Some("Cleanup all files".into()),
                },
            ))
            .expect("created deletion request");
        runtime
            .block_on(bundle.workspace.review_project_deletion_request(
                &deletion_request.id,
                &owner_session.user_id,
                true,
                ReviewProjectDeletionRequestInput {
                    review_comment: Some("Approved cleanup".into()),
                },
            ))
            .expect("approved deletion request");

        runtime
            .block_on(bundle.workspace.delete_project(&created.id))
            .expect("deleted project");

        assert!(!project_root.exists());
        assert!(!absolute_storage_path.exists());

        let connection = bundle.workspace.state.open_db().expect("open db");
        let remaining_resources: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM resources WHERE project_id = ?1",
                params![created.id],
                |row| row.get(0),
            )
            .expect("count project resources");
        let remaining_projects: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE id = ?1",
                params![created.id],
                |row| row.get(0),
            )
            .expect("count projects");
        assert_eq!(remaining_resources, 0);
        assert_eq!(remaining_projects, 0);
    }
