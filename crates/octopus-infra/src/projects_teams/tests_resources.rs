    #[test]
    fn project_owned_agents_and_teams_can_be_promoted_to_workspace() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let project = runtime
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Promotion Agents".into(),
                description: "Agent/team promotion coverage.".into(),
                resource_directory: "data/projects/promotion-agents/resources".into(),
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

        let project_agent = runtime
            .block_on(bundle.workspace.create_agent(UpsertAgentInput {
                workspace_id: project.workspace_id.clone(),
                project_id: Some(project.id.clone()),
                scope: "project".into(),
                name: "Promotion Analyst".into(),
                avatar: None,
                remove_avatar: None,
                personality: "Project-only analyst".into(),
                tags: vec!["promotion".into()],
                prompt: "Promote this agent into the workspace.".into(),
                builtin_tool_keys: vec!["bash".into()],
                skill_ids: Vec::new(),
                mcp_server_names: Vec::new(),
                task_domains: vec!["promotion".into()],
                default_model_strategy: None,
                capability_policy: None,
                permission_envelope: None,
                memory_policy: None,
                delegation_policy: None,
                approval_preference: None,
                output_contract: None,
                shared_capability_policy: None,
                description: "Project-owned agent".into(),
                status: "active".into(),
            }))
            .expect("created project agent");

        let project_team = runtime
            .block_on(bundle.workspace.create_team(UpsertTeamInput {
                workspace_id: project.workspace_id.clone(),
                project_id: Some(project.id.clone()),
                scope: "project".into(),
                name: "Promotion Strike Team".into(),
                avatar: None,
                remove_avatar: None,
                personality: "Project-only team".into(),
                tags: vec!["promotion".into()],
                prompt: "Promote this team into the workspace.".into(),
                builtin_tool_keys: vec!["bash".into()],
                skill_ids: Vec::new(),
                mcp_server_names: Vec::new(),
                task_domains: vec!["promotion".into()],
                default_model_strategy: None,
                capability_policy: None,
                permission_envelope: None,
                memory_policy: None,
                delegation_policy: None,
                approval_preference: None,
                output_contract: None,
                shared_capability_policy: None,
                leader_ref: crate::canonical_agent_ref(&project_agent.id),
                member_refs: vec![crate::canonical_agent_ref(&project_agent.id)],
                team_topology: None,
                shared_memory_policy: None,
                mailbox_policy: None,
                artifact_handoff_policy: None,
                workflow_affordance: None,
                worker_concurrency_limit: None,
                description: "Project-owned team".into(),
                status: "active".into(),
            }))
            .expect("created project team");

        let promoted_agent = runtime
            .block_on(
                bundle
                    .workspace
                    .copy_workspace_agent_from_builtin(&project_agent.id),
            )
            .expect("promoted project agent");
        assert_eq!(promoted_agent.failure_count, 0);
        assert_eq!(promoted_agent.agent_count, 1);

        let promoted_team = runtime
            .block_on(
                bundle
                    .workspace
                    .copy_workspace_team_from_builtin(&project_team.id),
            )
            .expect("promoted project team");
        assert_eq!(promoted_team.failure_count, 0);
        assert_eq!(promoted_team.team_count, 1);
        assert_eq!(promoted_team.agent_count, 1);

        let agents = runtime
            .block_on(bundle.workspace.list_agents())
            .expect("list agents");
        assert!(agents.iter().any(|agent| agent.id == project_agent.id
            && agent.project_id.as_deref() == Some(project.id.as_str())));
        assert!(agents
            .iter()
            .any(|agent| agent.name == "Promotion Analyst" && agent.project_id.is_none()));

        let teams = runtime
            .block_on(bundle.workspace.list_teams())
            .expect("list teams");
        assert!(teams.iter().any(|team| team.id == project_team.id
            && team.project_id.as_deref() == Some(project.id.as_str())));
        assert!(teams
            .iter()
            .any(|team| team.name == "Promotion Strike Team" && team.project_id.is_none()));
    }

    #[test]
    fn import_folder_creates_single_record_and_delete_removes_managed_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let created = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Import Project".into(),
                description: "Resource import coverage.".into(),
                resource_directory: "data/projects/import-project/resources".into(),
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

        let imported = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.import_project_resource(
                &created.id,
                "user-owner",
                octopus_core::WorkspaceResourceImportInput {
                    name: "design-assets".into(),
                    root_dir_name: Some("design-assets".into()),
                    scope: "project".into(),
                    visibility: "public".into(),
                    tags: Some(vec!["assets".into()]),
                    files: vec![
                        encoded_file("brief.md", "text/markdown", "# Brief"),
                        encoded_file("nested/spec.json", "application/json", "{\"ok\":true}"),
                    ],
                },
            ))
            .expect("imported folder");

        assert_eq!(imported.kind, "folder");
        assert_eq!(imported.scope, "project");
        assert_eq!(imported.visibility, "public");

        let listed = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.list_project_resources(&created.id))
            .expect("listed resources");
        assert_eq!(listed.len(), 1);

        let children = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.list_resource_children(&imported.id))
            .expect("children");
        assert_eq!(children.len(), 2);
        assert!(children
            .iter()
            .any(|entry| entry.relative_path == "nested/spec.json"));

        let promoted = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.promote_resource(
                &imported.id,
                octopus_core::PromoteWorkspaceResourceInput {
                    scope: "workspace".into(),
                },
            ))
            .expect("promoted");
        assert_eq!(promoted.scope, "workspace");

        let storage_path = imported.storage_path.expect("storage path");
        let absolute_storage_path = bundle.paths.root.join(&storage_path);
        assert!(absolute_storage_path.exists());

        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(
                bundle
                    .workspace
                    .delete_project_resource(&created.id, &imported.id),
            )
            .expect("deleted");

        assert!(!absolute_storage_path.exists());
    }

    #[test]
    fn workspace_import_writes_into_workspace_resources_and_supports_content_and_directory_browsing(
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let workspace_id = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.workspace_summary())
            .expect("workspace summary")
            .id;

        let imported = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.import_workspace_resource(
                &workspace_id,
                "user-owner",
                octopus_core::WorkspaceResourceImportInput {
                    name: "workspace-handbook.md".into(),
                    root_dir_name: None,
                    scope: "workspace".into(),
                    visibility: "public".into(),
                    tags: Some(vec!["docs".into()]),
                    files: vec![encoded_file(
                        "workspace-handbook.md",
                        "text/markdown",
                        "# Workspace Handbook",
                    )],
                },
            ))
            .expect("imported workspace resource");

        let storage_path = imported.storage_path.clone().expect("storage path");
        assert!(storage_path.starts_with("data/resources/workspace/workspace-handbook"));
        assert!(storage_path.ends_with(".md"));
        assert!(bundle.paths.root.join(&storage_path).exists());

        let content = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.get_resource_content(&imported.id))
            .expect("resource content");
        assert_eq!(content.preview_kind, "markdown");
        assert_eq!(
            content.text_content.as_deref(),
            Some("# Workspace Handbook")
        );
        assert!(content.data_base64.is_none());

        let directories = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.list_directories(Some("data/resources")))
            .expect("directories");
        assert_eq!(directories.current_path, "data/resources");
        assert!(directories
            .entries
            .iter()
            .any(|entry| entry.path == "data/resources/workspace"));
    }

    #[test]
    fn project_personal_resources_follow_the_promotion_chain() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let created = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Promotion Project".into(),
                description: "Promotion coverage.".into(),
                resource_directory: "data/projects/promotion-project/resources".into(),
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

        let imported = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.import_project_resource(
                &created.id,
                "user-owner",
                octopus_core::WorkspaceResourceImportInput {
                    name: "private-notes.md".into(),
                    root_dir_name: None,
                    scope: "personal".into(),
                    visibility: "private".into(),
                    tags: Some(vec!["notes".into()]),
                    files: vec![encoded_file(
                        "private-notes.md",
                        "text/markdown",
                        "# Private Notes",
                    )],
                },
            ))
            .expect("imported personal resource");

        assert_eq!(imported.scope, "personal");
        assert_eq!(imported.visibility, "private");

        let invalid_direct_promotion = tokio::runtime::Runtime::new().expect("runtime").block_on(
            bundle.workspace.promote_resource(
                &imported.id,
                octopus_core::PromoteWorkspaceResourceInput {
                    scope: "workspace".into(),
                },
            ),
        );
        assert!(invalid_direct_promotion.is_err());

        let promoted_to_project = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.promote_resource(
                &imported.id,
                octopus_core::PromoteWorkspaceResourceInput {
                    scope: "project".into(),
                },
            ))
            .expect("promoted to project");
        assert_eq!(promoted_to_project.scope, "project");
        assert_eq!(promoted_to_project.visibility, "private");

        let promoted_to_workspace = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.promote_resource(
                &imported.id,
                octopus_core::PromoteWorkspaceResourceInput {
                    scope: "workspace".into(),
                },
            ))
            .expect("promoted to workspace");
        assert_eq!(promoted_to_workspace.scope, "workspace");
        assert_eq!(promoted_to_workspace.visibility, "private");
    }
