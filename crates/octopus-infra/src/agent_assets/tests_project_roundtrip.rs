    #[test]
    fn export_project_bundle_includes_project_skill_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = paths.database().expect("database").acquire().expect("db");
        ensure_test_tables(&connection);

        let project_id = "project-finance";
        let skill_slug = "project-skill";
        let skill_root = paths.project_skills_root(project_id).join(skill_slug);
        fs::create_dir_all(&skill_root).expect("create skill root");
        fs::write(
            skill_root.join("SKILL.md"),
            "---\nname: project-skill\ndescription: Project scoped skill\n---\n\n# Overview\n\nproject only\n",
        )
        .expect("write skill");

        let project_agent_id = "agent-project-1";
        let project_agent = test_agent_record(
            project_agent_id,
            "ws-local",
            Some(project_id),
            "project",
            "项目财务分析师",
            "项目财务",
            vec!["项目".into()],
            "# 角色定义\n项目财务分析\n",
            Vec::new(),
            vec![managed_skill_id(
                &AssetTargetScope::Project(project_id),
                skill_slug,
            )],
            Vec::new(),
            "项目级技能",
        );
        write_agent_record(&connection, &project_agent, false).expect("write project agent");

        let exported = crate::agent_bundle::export_assets(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Project(project_id),
            ExportWorkspaceAgentBundleInput {
                mode: "batch".into(),
                agent_ids: Vec::new(),
                team_ids: Vec::new(),
            },
        )
        .expect("export");

        assert!(exported.files.iter().any(|file| {
            file.relative_path == "templates/项目财务分析师/skills/project-skill/SKILL.md"
        }));
    }

    #[test]
    fn export_project_bundle_materializes_assigned_workspace_builtin_dependencies() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = paths.database().expect("database").acquire().expect("db");
        ensure_test_tables(&connection);

        let builtin_skill = crate::agent_bundle::list_builtin_skill_assets()
            .expect("builtin skills")
            .into_iter()
            .next()
            .expect("builtin skill");
        let builtin_skill_id = catalog_hash_id("skill", &builtin_skill.display_path);

        let agent_id = "agent-linked-workspace";
        let record = test_agent_record(
            agent_id,
            "ws-local",
            None,
            "workspace",
            "财务联动员工",
            "处理项目联动财务任务",
            vec!["财务".into()],
            "# 角色定义\n处理项目财务联动\n",
            vec!["bash".into()],
            vec![builtin_skill_id],
            Vec::new(),
            "工作区级财务员工",
        );
        write_agent_record(&connection, &record, false).expect("write workspace agent");
        connection
            .execute(
                "INSERT INTO projects (id, workspace_id, name, status, description, resource_directory, assignments_json, owner_user_id, member_user_ids_json, permission_overrides_json, linked_workspace_assets_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    "proj-finance",
                    "ws-local",
                    "Finance Project",
                    "active",
                    "Finance export coverage",
                    "data/projects/proj-finance/resources",
                    serde_json::to_string(&json!({
                        "agents": {
                            "agentIds": [agent_id],
                            "teamIds": []
                        }
                    }))
                    .expect("serialize assignments"),
                    "user-owner",
                    serde_json::to_string(&vec!["user-owner"]).expect("serialize members"),
                    serde_json::to_string(&json!({
                        "agents": "inherit",
                        "resources": "inherit",
                        "tools": "inherit",
                        "knowledge": "inherit"
                    }))
                    .expect("serialize permission overrides"),
                    serde_json::to_string(&json!({
                        "agentIds": [agent_id],
                        "resourceIds": [],
                        "toolSourceKeys": [],
                        "knowledgeIds": []
                    }))
                    .expect("serialize linked assets"),
                ],
            )
            .expect("insert project");

        let exported = crate::agent_bundle::export_assets(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Project("proj-finance"),
            ExportWorkspaceAgentBundleInput {
                mode: "single".into(),
                agent_ids: vec![agent_id.into()],
                team_ids: Vec::new(),
            },
        )
        .expect("export");

        assert_eq!(exported.root_dir_name, "财务联动员工");
        assert!(exported.files.iter().any(|file| {
            file.relative_path.starts_with("财务联动员工/skills/")
                && file.relative_path.ends_with("/SKILL.md")
        }));
        assert!(!exported
            .files
            .iter()
            .any(|file| file.relative_path.contains("/.octopus/")));
    }

    #[test]
    fn export_project_bundle_roundtrips_via_project_import() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = paths.database().expect("database").acquire().expect("db");
        ensure_test_tables(&connection);

        let builtin_skill = crate::agent_bundle::list_builtin_skill_assets()
            .expect("builtin skills")
            .into_iter()
            .next()
            .expect("builtin skill");
        let builtin_skill_id = catalog_hash_id("skill", &builtin_skill.display_path);

        let agent_id = "agent-linked-roundtrip";
        let record = test_agent_record(
            agent_id,
            "ws-local",
            None,
            "workspace",
            "导出回导员工",
            "负责导出回导验证",
            vec!["回归".into()],
            "# 角色定义\n导出后重新导入\n",
            vec!["bash".into()],
            vec![builtin_skill_id],
            Vec::new(),
            "验证项目导出闭包",
        );
        write_agent_record(&connection, &record, false).expect("write workspace agent");
        connection
            .execute(
                "INSERT INTO projects (id, workspace_id, name, status, description, resource_directory, assignments_json, owner_user_id, member_user_ids_json, permission_overrides_json, linked_workspace_assets_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    "proj-export",
                    "ws-local",
                    "Export Project",
                    "active",
                    "Project export roundtrip",
                    "data/projects/proj-export/resources",
                    serde_json::to_string(&json!({
                        "agents": {
                            "agentIds": [agent_id],
                            "teamIds": []
                        }
                    }))
                    .expect("serialize assignments"),
                    "user-owner",
                    serde_json::to_string(&vec!["user-owner"]).expect("serialize members"),
                    serde_json::to_string(&json!({
                        "agents": "inherit",
                        "resources": "inherit",
                        "tools": "inherit",
                        "knowledge": "inherit"
                    }))
                    .expect("serialize permission overrides"),
                    serde_json::to_string(&json!({
                        "agentIds": [agent_id],
                        "resourceIds": [],
                        "toolSourceKeys": [],
                        "knowledgeIds": []
                    }))
                    .expect("serialize linked assets"),
                ],
            )
            .expect("insert project");

        let exported = crate::agent_bundle::export_assets(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Project("proj-export"),
            ExportWorkspaceAgentBundleInput {
                mode: "single".into(),
                agent_ids: vec![agent_id.into()],
                team_ids: Vec::new(),
            },
        )
        .expect("export");

        let imported = crate::agent_bundle::execute_import(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Project("proj-import"),
            ImportWorkspaceAgentBundleInput {
                files: exported.files,
            },
        )
        .expect("import");

        assert_eq!(imported.failure_count, 0);
        assert_eq!(imported.agent_count, 1);
        assert_eq!(imported.team_count, 0);
        assert_eq!(imported.skill_count, 1);
        assert_eq!(imported.mcp_count, 0);
        assert_eq!(imported.avatar_count, 1);
    }

    #[test]
    fn export_import_skips_legacy_asset_state_sidecar_files() {
        let source_temp = tempfile::tempdir().expect("source tempdir");
        let source_paths = WorkspacePaths::new(source_temp.path());
        source_paths.ensure_layout().expect("source layout");
        let source_connection = source_paths
            .database()
            .expect("database")
            .acquire()
            .expect("source db");
        ensure_test_tables(&source_connection);

        let skill_slug = "managed-roundtrip";
        let skill_dir = source_paths.managed_skills_dir.join(skill_slug);
        fs::create_dir_all(&skill_dir).expect("managed skill dir");
        fs::write(
            skill_dir.join(SKILL_FRONTMATTER_FILE),
            "---\nname: managed-roundtrip\ndescription: Managed roundtrip skill.\n---\n",
        )
        .expect("write managed skill");

        let skill_source_key = format!("skill:data/skills/{skill_slug}/{SKILL_FRONTMATTER_FILE}");
        let mcp_server_name = "roundtrip-mcp";
        let mcp_source_key = format!("mcp:{mcp_server_name}");

        let mut source_asset_state =
            crate::resources_skills::load_workspace_asset_state_document(&source_paths)
                .expect("load source asset state");
        crate::resources_skills::set_workspace_asset_enabled(
            &mut source_asset_state,
            &skill_source_key,
            false,
        );
        crate::resources_skills::set_workspace_asset_trusted(
            &mut source_asset_state,
            &skill_source_key,
            true,
        );
        crate::resources_skills::set_workspace_asset_enabled(
            &mut source_asset_state,
            &mcp_source_key,
            false,
        );
        crate::resources_skills::set_workspace_asset_trusted(
            &mut source_asset_state,
            &mcp_source_key,
            true,
        );
        crate::resources_skills::save_workspace_asset_state_document(
            &source_paths,
            &source_asset_state,
        )
        .expect("save source asset state");

        crate::resources_skills::write_workspace_runtime_document(
            &source_paths,
            &serde_json::Map::from_iter([(
                "mcpServers".to_string(),
                json!({
                    mcp_server_name: {
                        "transport": "stdio",
                        "command": "roundtrip-mcp",
                        "args": ["serve"]
                    }
                }),
            )]),
        )
        .expect("write workspace runtime document");

        let agent_id = "agent-managed-roundtrip";
        let record = test_agent_record(
            agent_id,
            "ws-local",
            None,
            "workspace",
            "Managed Roundtrip",
            "Verifies asset metadata export and import",
            vec!["roundtrip".into()],
            "# Role\nVerify asset metadata roundtrip.\n",
            vec!["bash".into()],
            vec![managed_skill_id(&AssetTargetScope::Workspace, skill_slug)],
            vec![mcp_server_name.into()],
            "Managed asset metadata roundtrip",
        );
        write_agent_record(&source_connection, &record, false).expect("write source agent");

        let exported = crate::agent_bundle::export_assets(
            &source_connection,
            &source_paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Workspace,
            ExportWorkspaceAgentBundleInput {
                mode: "single".into(),
                agent_ids: vec![agent_id.into()],
                team_ids: Vec::new(),
            },
        )
        .expect("export source assets");
        assert!(
            !exported
                .files
                .iter()
                .any(|file| file.relative_path.contains("/.octopus/")),
            "template export should not emit legacy .octopus metadata files"
        );

        let destination_temp = tempfile::tempdir().expect("destination tempdir");
        let destination_paths = WorkspacePaths::new(destination_temp.path());
        destination_paths
            .ensure_layout()
            .expect("destination layout");
        let destination_connection = destination_paths
            .database()
            .expect("database")
            .acquire()
            .expect("destination db");
        ensure_test_tables(&destination_connection);

        crate::agent_bundle::execute_import(
            &destination_connection,
            &destination_paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Workspace,
            ImportWorkspaceAgentBundleInput {
                files: exported.files,
            },
        )
        .expect("import destination assets");

        assert!(
            !destination_paths.workspace_asset_state_path.exists(),
            "template import should not materialize legacy asset state sidecar"
        );

        let destination_asset_state =
            crate::resources_skills::load_workspace_asset_state_document(&destination_paths)
                .expect("load destination asset state");
        let destination_asset_state = serde_json::to_value(destination_asset_state)
            .expect("serialize destination asset state");
        assert!(destination_asset_state["assets"]
            .get(skill_source_key.as_str())
            .is_none());
        assert!(destination_asset_state["assets"]
            .get(mcp_source_key.as_str())
            .is_none());
    }
