    #[test]
    fn default_avatar_payload_accepts_svg_header_assets() {
        let (file_name, content_type, bytes) =
            default_avatar_payload("agent", "workspace:agent-svg").expect("default avatar");

        assert!(file_name.ends_with(".svg"));
        assert_eq!(content_type, "image/svg+xml");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn persist_avatar_writes_svg_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        let avatar = ParsedAssetAvatar {
            source_id: "agent-svg".into(),
            owner_kind: "agent".into(),
            owner_name: "SVG Agent".into(),
            file_name: "portrait.svg".into(),
            content_type: "image/svg+xml".into(),
            bytes: br#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#.to_vec(),
            generated: false,
        };

        let relative_path = persist_avatar(&paths, "agent-svg", &avatar)
            .expect("persist avatar")
            .expect("avatar path");

        assert_eq!(relative_path, "data/blobs/avatars/agent-svg.svg");
        assert_eq!(
            fs::read(paths.root.join(&relative_path)).expect("read avatar"),
            avatar.bytes
        );
    }

    #[test]
    fn content_type_for_export_returns_svg_mime_type() {
        assert_eq!(content_type_for_export("avatar.svg"), "image/svg+xml");
    }

    #[test]
    fn agent_record_roundtrips_personal_owner_metadata_and_role() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = paths.database().expect("database").acquire().expect("db");
        ensure_test_tables(&connection);

        let mut record = test_agent_record(
            "pet-user-analyst",
            "ws-local",
            None,
            "personal",
            "Analyst Pet",
            "Curious companion",
            vec!["pet".into()],
            "Keep the owner company.",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            "Personal pet agent",
        );
        record.owner_user_id = Some("user-analyst".into());
        record.asset_role = "pet".into();

        write_agent_record(&connection, &record, false).expect("write agent");

        let reloaded = load_agents(&connection)
            .expect("load agents")
            .into_iter()
            .find(|agent| agent.id == record.id)
            .expect("reloaded pet");

        assert_eq!(reloaded.scope, "personal");
        assert_eq!(reloaded.owner_user_id.as_deref(), Some("user-analyst"));
        assert_eq!(reloaded.asset_role, "pet");
    }

    #[test]
    fn pet_extension_enforces_single_pet_per_workspace_owner() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = paths.database().expect("database").acquire().expect("db");
        ensure_test_tables(&connection);

        connection
            .execute(
                "INSERT INTO pet_agent_extensions (
                    pet_id, workspace_id, owner_user_id, species, display_name, avatar_label,
                    summary, greeting, mood, favorite_snack, prompt_hints_json, fallback_asset,
                    rive_asset, state_machine, updated_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6,
                    ?7, ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15
                )",
                params![
                    "pet-user-owner",
                    "ws-local",
                    "user-owner",
                    "octopus",
                    "小章",
                    "Octopus mascot",
                    "First pet",
                    "Hello",
                    "happy",
                    "虾",
                    serde_json::to_string(&vec!["打个招呼"]).expect("prompt hints"),
                    "octopus",
                    Option::<String>::None,
                    Option::<String>::None,
                    1_i64,
                ],
            )
            .expect("insert first pet extension");

        let duplicate = connection.execute(
            "INSERT INTO pet_agent_extensions (
                pet_id, workspace_id, owner_user_id, species, display_name, avatar_label,
                summary, greeting, mood, favorite_snack, prompt_hints_json, fallback_asset,
                rive_asset, state_machine, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15
            )",
            params![
                "pet-user-owner-duplicate",
                "ws-local",
                "user-owner",
                "duck",
                "小鸭",
                "Duck mascot",
                "Duplicate pet",
                "Hi",
                "happy",
                "玉米",
                serde_json::to_string(&vec!["去散步"]).expect("prompt hints"),
                "duck",
                Option::<String>::None,
                Option::<String>::None,
                2_i64,
            ],
        );

        assert!(duplicate.is_err(), "duplicate pet extension should fail");
    }

    #[test]
    fn reload_backfills_missing_personal_pet_for_existing_user_once() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime
            .block_on(bundle.auth.register_bootstrap_admin(
                octopus_core::RegisterBootstrapAdminRequest {
                    client_app_id: "octopus-desktop".into(),
                    username: "owner".into(),
                    display_name: "Owner".into(),
                    password: "password123".into(),
                    confirm_password: "password123".into(),
                    avatar: avatar_payload(),
                    workspace_id: Some("ws-local".into()),
                    mapped_directory: None,
                },
            ))
            .expect("bootstrap owner");

        let connection = bundle
            .paths
            .database()
            .expect("database")
            .acquire()
            .expect("db");
        connection
            .execute(
                "INSERT INTO users (
                    id, username, display_name, avatar_path, avatar_content_type, avatar_byte_size,
                    avatar_content_hash, status, password_hash, password_state, created_at, updated_at
                ) VALUES (
                    ?1, ?2, ?3, NULL, NULL, NULL,
                    NULL, 'active', ?4, 'set', ?5, ?6
                )",
                params![
                    "user-existing",
                    "existing",
                    "Existing User",
                    "plain::password123",
                    10_i64,
                    10_i64,
                ],
            )
            .expect("insert existing user");

        drop(bundle);

        let reloaded = build_infra_bundle(temp.path()).expect("reloaded infra bundle");
        let first_pet = reloaded
            .workspace
            .state
            .agents
            .lock()
            .expect("agents")
            .iter()
            .find(|record| {
                record.asset_role == "pet"
                    && record.owner_user_id.as_deref() == Some("user-existing")
            })
            .cloned()
            .expect("backfilled pet");

        drop(reloaded);

        let reloaded_again = build_infra_bundle(temp.path()).expect("reloaded again");
        let matching_pets = reloaded_again
            .workspace
            .state
            .agents
            .lock()
            .expect("agents")
            .iter()
            .filter(|record| {
                record.asset_role == "pet"
                    && record.owner_user_id.as_deref() == Some("user-existing")
            })
            .cloned()
            .collect::<Vec<_>>();

        assert_eq!(matching_pets.len(), 1);
        assert_eq!(matching_pets[0].id, first_pet.id);
    }

    #[test]
    fn parse_frontmatter_tolerates_inline_closing_delimiter_suffix() {
        let markdown = "---\nname: 简历筛选师\ndescription: 负责候选人简历初筛输出---\ncharacter: 活泼可爱\n---\n\n# 角色定义\n说明\n";

        let (frontmatter, body) = parse_frontmatter(markdown).expect("parse frontmatter");

        assert_eq!(
            yaml_string(&frontmatter, "name").as_deref(),
            Some("简历筛选师")
        );
        assert_eq!(
            yaml_string(&frontmatter, "description").as_deref(),
            Some("负责候选人简历初筛输出")
        );
        assert_eq!(
            yaml_string(&frontmatter, "character").as_deref(),
            Some("活泼可爱")
        );
        assert!(body.contains("# 角色定义"));
    }

