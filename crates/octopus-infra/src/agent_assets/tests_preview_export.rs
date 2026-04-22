    #[test]
    fn parse_frontmatter_fallback_merges_indented_continuation_lines() {
        let markdown = "---\nversion: \"2.0.0\"\nname: Performance Review\ndescription: \"第一段说明。\"\n  第二段说明。\nauthor: BytesAgain\n---\n\n# Body\n";

        let (frontmatter, body) = parse_frontmatter(markdown).expect("parse frontmatter");

        assert_eq!(
            yaml_string(&frontmatter, "description").as_deref(),
            Some("第一段说明。 第二段说明。")
        );
        assert_eq!(
            yaml_string(&frontmatter, "author").as_deref(),
            Some("BytesAgain")
        );
        assert!(body.contains("# Body"));
    }

    #[test]
    fn preview_supports_standalone_agent_root_and_yaml_arrays() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = paths.database().expect("database").acquire().expect("db");
        ensure_test_tables(&connection);

        let preview = crate::agent_bundle::preview_import(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Workspace,
            ImportWorkspaceAgentBundlePreviewInput {
                files: vec![
                    encoded_file(
                        "财务分析师/财务分析师.md",
                        "---\nname: 财务分析师\ndescription: 财务分析\ncharacter: 数字敏感\ntools: [\"ALL\"]\nskills: [\"shared-skill\"]\nmcps: [\"ops\"]\n---\n\n# 角色定义\n财务专家\n",
                    ),
                    encoded_file(
                        "财务分析师/skills/shared-skill/SKILL.md",
                        "---\nname: shared-skill\ndescription: shared\n---\n\n# Shared\n",
                    ),
                    encoded_json_file(
                        "财务分析师/mcps/ops.json",
                        r#"{"type":"http","url":"https://ops.example.test/mcp"}"#,
                    ),
                ],
            },
        )
        .expect("preview");

        assert_eq!(preview.detected_agent_count, 1);
        assert_eq!(preview.detected_team_count, 0);
        assert_eq!(preview.unique_skill_count, 1);
        assert_eq!(preview.unique_mcp_count, 1);
        assert_eq!(preview.agents[0].mcp_server_names, vec!["ops"]);
    }

    #[test]
    fn preview_supports_team_bundle_and_reference_only_mcp_warning() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = paths.database().expect("database").acquire().expect("db");
        ensure_test_tables(&connection);

        let preview = crate::agent_bundle::preview_import(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Workspace,
            ImportWorkspaceAgentBundlePreviewInput {
                files: vec![
                    encoded_file(
                        "财务部/财务部门说明.md",
                        "---\nname: 财务部\ndescription: 财务团队\nleader: 财务负责人\nmember: [\"财务负责人\", \"财务分析师\"]\n---\n\n# leader职责\n负责统筹\n",
                    ),
                    encoded_file(
                        "财务部/财务负责人/财务负责人.md",
                        "---\nname: 财务负责人\ndescription: 负责人\nskills: []\nmcps: []\n---\n\n# 角色定义\n负责人\n",
                    ),
                    encoded_file(
                        "财务部/财务分析师/财务分析师.md",
                        "---\nname: 财务分析师\ndescription: 分析师\nmcps: [\"shared-ops\"]\n---\n\n# 角色定义\n分析师\n",
                    ),
                ],
            },
        )
        .expect("preview");

        assert_eq!(preview.detected_team_count, 1);
        assert_eq!(preview.detected_agent_count, 2);
        assert_eq!(preview.teams.len(), 1);
        assert_eq!(preview.teams[0].member_names.len(), 2);
        assert!(preview.issues.iter().any(|item| item.scope == "mcp"));
    }

    #[test]
    fn export_single_agent_uses_agent_root_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = paths.database().expect("database").acquire().expect("db");
        ensure_test_tables(&connection);

        let agent_id = "agent-1";
        let record = test_agent_record(
            agent_id,
            "ws-local",
            None,
            "workspace",
            "财务分析师",
            "数字敏感",
            vec!["财务".into()],
            "# 角色定义\n财务专家\n",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            "负责财务分析",
        );
        write_agent_record(&connection, &record, false).expect("write agent");

        let exported = crate::agent_bundle::export_assets(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Workspace,
            ExportWorkspaceAgentBundleInput {
                mode: "single".into(),
                agent_ids: vec![agent_id.into()],
                team_ids: Vec::new(),
            },
        )
        .expect("export");

        assert_eq!(exported.root_dir_name, "财务分析师");
        assert!(exported
            .files
            .iter()
            .any(|file| file.relative_path == "财务分析师/财务分析师.md"));
    }

    #[test]
    fn export_single_team_keeps_members_directly_under_team_root_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = paths.database().expect("database").acquire().expect("db");
        ensure_test_tables(&connection);

        let leader = test_agent_record(
            "agent-leader",
            "ws-local",
            None,
            "workspace",
            "财务负责人",
            "负责人",
            vec!["财务".into()],
            "# 角色定义\n统筹\n",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            "负责人",
        );
        let member = test_agent_record(
            "agent-member",
            "ws-local",
            None,
            "workspace",
            "财务分析师",
            "分析",
            vec!["财务".into()],
            "# 角色定义\n分析\n",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            "分析师",
        );
        write_agent_record(&connection, &leader, false).expect("write leader");
        write_agent_record(&connection, &member, false).expect("write member");

        let team = test_team_record(
            "team-1",
            "ws-local",
            None,
            "workspace",
            "财务部",
            "团队",
            vec!["财务".into()],
            "# 团队职责\n统筹财务\n",
            Some(leader.id.clone()),
            vec![leader.id.clone(), member.id.clone()],
            "财务团队",
        );
        crate::infra_state::write_team_record(&connection, &team, false).expect("write team");

        let exported = crate::agent_bundle::export_assets(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Workspace,
            ExportWorkspaceAgentBundleInput {
                mode: "single".into(),
                agent_ids: Vec::new(),
                team_ids: vec![team.id.clone()],
            },
        )
        .expect("export");

        assert_eq!(exported.root_dir_name, "财务部");
        assert!(exported
            .files
            .iter()
            .any(|file| file.relative_path == "财务部/财务部门说明.md"));
        assert!(exported
            .files
            .iter()
            .any(|file| file.relative_path == "财务部/财务负责人/财务负责人.md"));
        assert!(exported
            .files
            .iter()
            .any(|file| file.relative_path == "财务部/财务分析师/财务分析师.md"));
        assert!(!exported.files.iter().any(|file| {
            file.relative_path == "财务部/财务部/财务负责人/财务负责人.md"
                || file.relative_path == "财务部/财务部/财务分析师/财务分析师.md"
        }));
    }

