    fn encoded_file(relative_path: &str, content: &str) -> WorkspaceDirectoryUploadEntry {
        WorkspaceDirectoryUploadEntry {
            relative_path: relative_path.into(),
            file_name: Path::new(relative_path)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .into(),
            content_type: "text/markdown".into(),
            data_base64: BASE64_STANDARD.encode(content.as_bytes()),
            byte_size: content.len() as u64,
        }
    }

    fn encoded_json_file(relative_path: &str, content: &str) -> WorkspaceDirectoryUploadEntry {
        WorkspaceDirectoryUploadEntry {
            relative_path: relative_path.into(),
            file_name: Path::new(relative_path)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .into(),
            content_type: "application/json".into(),
            data_base64: BASE64_STANDARD.encode(content.as_bytes()),
            byte_size: content.len() as u64,
        }
    }

    fn ensure_test_tables(connection: &Connection) {
        connection
            .execute_batch(
                "CREATE TABLE agents (
                    id TEXT PRIMARY KEY,
                    workspace_id TEXT NOT NULL,
                    project_id TEXT,
                    scope TEXT NOT NULL,
                    name TEXT NOT NULL,
                    avatar_path TEXT,
                    personality TEXT NOT NULL,
                    tags TEXT NOT NULL,
                    prompt TEXT NOT NULL,
                    builtin_tool_keys TEXT NOT NULL,
                    skill_ids TEXT NOT NULL,
                    mcp_server_names TEXT NOT NULL,
                    description TEXT NOT NULL,
                    status TEXT NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                CREATE TABLE teams (
                    id TEXT PRIMARY KEY,
                    workspace_id TEXT NOT NULL,
                    project_id TEXT,
                    scope TEXT NOT NULL,
                    name TEXT NOT NULL,
                    avatar_path TEXT,
                    personality TEXT NOT NULL,
                    tags TEXT NOT NULL,
                    prompt TEXT NOT NULL,
                    builtin_tool_keys TEXT NOT NULL,
                    skill_ids TEXT NOT NULL,
                    mcp_server_names TEXT NOT NULL,
                    leader_ref TEXT NOT NULL DEFAULT '',
                    member_refs TEXT NOT NULL DEFAULT '[]',
                    description TEXT NOT NULL,
                    status TEXT NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                CREATE TABLE project_agent_links (
                    workspace_id TEXT NOT NULL,
                    project_id TEXT NOT NULL,
                    agent_id TEXT NOT NULL,
                    linked_at INTEGER NOT NULL
                );
                CREATE TABLE project_team_links (
                    workspace_id TEXT NOT NULL,
                    project_id TEXT NOT NULL,
                    team_id TEXT NOT NULL,
                    linked_at INTEGER NOT NULL
                );
                CREATE TABLE projects (
                    id TEXT PRIMARY KEY,
                    workspace_id TEXT NOT NULL,
                    name TEXT NOT NULL,
                    status TEXT NOT NULL,
                    description TEXT NOT NULL,
                    resource_directory TEXT NOT NULL,
                    leader_agent_id TEXT,
                    manager_user_id TEXT,
                    preset_code TEXT,
                    assignments_json TEXT,
                    owner_user_id TEXT,
                    member_user_ids_json TEXT,
                    permission_overrides_json TEXT,
                    linked_workspace_assets_json TEXT
                );",
            )
            .expect("tables");
        ensure_agent_record_columns(connection).expect("agent columns");
        ensure_team_record_columns(connection).expect("team columns");
        ensure_bundle_asset_descriptor_columns(connection).expect("descriptor columns");
        crate::agent_bundle::shared::ensure_import_source_tables(connection)
            .expect("import source tables");
        ensure_pet_agent_extension_columns(connection).expect("pet extension columns");
    }

    fn test_agent_record(
        id: &str,
        workspace_id: &str,
        project_id: Option<&str>,
        scope: &str,
        name: &str,
        personality: &str,
        tags: Vec<String>,
        prompt: &str,
        builtin_tool_keys: Vec<String>,
        skill_ids: Vec<String>,
        mcp_server_names: Vec<String>,
        description: &str,
    ) -> AgentRecord {
        let task_domains = normalize_task_domains(tags.clone());
        AgentRecord {
            id: id.into(),
            workspace_id: workspace_id.into(),
            project_id: project_id.map(str::to_string),
            scope: scope.into(),
            owner_user_id: None,
            asset_role: "default".into(),
            name: name.into(),
            avatar_path: None,
            avatar: None,
            personality: personality.into(),
            tags,
            prompt: prompt.into(),
            builtin_tool_keys: builtin_tool_keys.clone(),
            skill_ids: skill_ids.clone(),
            mcp_server_names: mcp_server_names.clone(),
            task_domains: task_domains.clone(),
            manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
            default_model_strategy: default_model_strategy(),
            capability_policy: capability_policy_from_sources(
                &builtin_tool_keys,
                &skill_ids,
                &mcp_server_names,
            ),
            permission_envelope: default_permission_envelope(),
            memory_policy: default_agent_memory_policy(),
            delegation_policy: default_agent_delegation_policy(),
            approval_preference: default_approval_preference(),
            output_contract: default_output_contract(),
            shared_capability_policy: default_agent_shared_capability_policy(),
            integration_source: None,
            trust_metadata: default_asset_trust_metadata(),
            dependency_resolution: Vec::new(),
            import_metadata: default_asset_import_metadata(),
            description: description.into(),
            status: "active".into(),
            updated_at: timestamp_now(),
        }
    }

    fn avatar_payload() -> octopus_core::AvatarUploadPayload {
        octopus_core::AvatarUploadPayload {
            content_type: "image/png".into(),
            data_base64: "iVBORw0KGgo=".into(),
            file_name: "avatar.png".into(),
            byte_size: 8,
        }
    }

    fn test_team_record(
        id: &str,
        workspace_id: &str,
        project_id: Option<&str>,
        scope: &str,
        name: &str,
        personality: &str,
        tags: Vec<String>,
        prompt: &str,
        leader_actor_ref: Option<String>,
        member_actor_refs: Vec<String>,
        description: &str,
    ) -> TeamRecord {
        let task_domains = normalize_task_domains(tags.clone());
        let delegation_policy = default_team_delegation_policy();
        let leader_ref = leader_actor_ref
            .as_deref()
            .map(crate::canonical_agent_ref)
            .unwrap_or_default();
        let member_refs = crate::canonical_agent_refs(&member_actor_refs);
        TeamRecord {
            id: id.into(),
            workspace_id: workspace_id.into(),
            project_id: project_id.map(str::to_string),
            scope: scope.into(),
            name: name.into(),
            avatar_path: None,
            avatar: None,
            personality: personality.into(),
            tags,
            prompt: prompt.into(),
            builtin_tool_keys: Vec::new(),
            skill_ids: Vec::new(),
            mcp_server_names: Vec::new(),
            task_domains: task_domains.clone(),
            manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
            default_model_strategy: default_model_strategy(),
            capability_policy: capability_policy_from_sources(&[], &[], &[]),
            permission_envelope: default_permission_envelope(),
            memory_policy: default_team_memory_policy(),
            delegation_policy: delegation_policy.clone(),
            approval_preference: default_approval_preference(),
            output_contract: default_output_contract(),
            shared_capability_policy: default_team_shared_capability_policy(),
            leader_ref: leader_ref.clone(),
            member_refs: member_refs.clone(),
            team_topology: team_topology_from_refs(Some(leader_ref), member_refs.clone()),
            shared_memory_policy: default_shared_memory_policy(),
            mailbox_policy: default_mailbox_policy(),
            artifact_handoff_policy: default_artifact_handoff_policy(),
            workflow_affordance: workflow_affordance_from_task_domains(&task_domains, true, true),
            worker_concurrency_limit: delegation_policy.max_worker_count,
            integration_source: None,
            trust_metadata: default_asset_trust_metadata(),
            dependency_resolution: Vec::new(),
            import_metadata: default_asset_import_metadata(),
            description: description.into(),
            status: "active".into(),
            updated_at: timestamp_now(),
        }
    }
