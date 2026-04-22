    #[test]
    fn update_workspace_moves_the_real_workspace_root_and_preserves_shell_root_pointer() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        bootstrap_admin_session(&bundle);
        let mapped_root = temp
            .path()
            .parent()
            .expect("temp parent")
            .join(format!("octopus-mapped-root-{}", uuid::Uuid::new_v4()));

        let updated = runtime()
            .block_on(bundle.workspace.update_workspace(UpdateWorkspaceRequest {
                name: Some("Workspace Moved".into()),
                avatar: None,
                remove_avatar: Some(false),
                mapped_directory: Some(mapped_root.to_string_lossy().to_string()),
            }))
            .expect("updated workspace");

        assert_eq!(updated.name, "Workspace Moved");
        assert_eq!(
            updated.mapped_directory.as_deref(),
            Some(mapped_root.to_string_lossy().as_ref())
        );
        assert_eq!(
            updated.mapped_directory_default.as_deref(),
            Some(temp.path().to_string_lossy().as_ref())
        );
        assert!(mapped_root.join("data").join("main.db").exists());
        assert!(mapped_root.join("config").join("workspace.toml").exists());
        assert!(!temp.path().join("data").join("main.db").exists());

        let shell_pointer = fs::read_to_string(temp.path().join("config").join("workspace.toml"))
            .expect("shell root workspace config");
        assert!(shell_pointer.contains(mapped_root.to_string_lossy().as_ref()));

        let reloaded = build_infra_bundle(&mapped_root).expect("reloaded bundle");
        let workspace = runtime()
            .block_on(reloaded.workspace.workspace_summary())
            .expect("reloaded workspace summary");
        assert_eq!(workspace.name, "Workspace Moved");
        assert_eq!(
            workspace.mapped_directory.as_deref(),
            Some(mapped_root.to_string_lossy().as_ref())
        );
        assert_eq!(
            workspace.mapped_directory_default.as_deref(),
            Some(temp.path().to_string_lossy().as_ref())
        );

        let login = runtime()
            .block_on(reloaded.auth.login(LoginRequest {
                client_app_id: "octopus-desktop".into(),
                username: "owner".into(),
                password: "password123".into(),
                workspace_id: Some("ws-local".into()),
            }))
            .expect("login after move");
        assert_eq!(
            login.session.user_id,
            workspace.owner_user_id.expect("owner user id")
        );
    }

    fn insert_artifact_record(
        connection: &Connection,
        id: &str,
        project_id: &str,
        title: &str,
        updated_at: u64,
    ) {
        connection
            .execute(
                "INSERT INTO artifact_records (
                    id, workspace_id, project_id, conversation_id, session_id, run_id,
                    source_message_id, parent_artifact_id, title, status, preview_kind,
                    latest_version, promotion_state, promotion_knowledge_id, updated_at,
                    storage_path, content_hash, byte_size, content_type
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6,
                    NULL, NULL, ?7, 'ready', 'markdown',
                    1, 'not-promoted', NULL, ?8,
                    NULL, NULL, NULL, 'text/markdown'
                )",
                rusqlite::params![
                    id,
                    DEFAULT_WORKSPACE_ID,
                    project_id,
                    format!("conv-{id}"),
                    format!("session-{id}"),
                    format!("run-{id}"),
                    title,
                    updated_at as i64,
                ],
            )
            .expect("insert artifact record");
    }

    #[test]
    fn create_agent_persists_runtime_policy_fields_across_db_reload() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let created = runtime()
            .block_on(bundle.workspace.create_agent(UpsertAgentInput {
                workspace_id: DEFAULT_WORKSPACE_ID.into(),
                project_id: None,
                scope: "workspace".into(),
                name: "Research Analyst".into(),
                avatar: None,
                remove_avatar: None,
                personality: "Structured and evidence-driven".into(),
                tags: vec!["research".into(), "docs".into()],
                prompt: "Investigate sources and produce a concise brief.".into(),
                builtin_tool_keys: vec!["bash".into(), "read_file".into()],
                skill_ids: vec!["skill-research".into()],
                mcp_server_names: vec!["browser".into()],
                description: "Produces research briefs and source syntheses.".into(),
                status: "active".into(),
                task_domains: vec!["research".into(), "docs".into()],
                default_model_strategy: Some(DefaultModelStrategy {
                    selection_mode: "actor-default".into(),
                    preferred_model_ref: Some("claude-sonnet-4-5".into()),
                    fallback_model_refs: vec!["gpt-4o".into()],
                    allow_turn_override: false,
                }),
                capability_policy: Some(CapabilityPolicy {
                    mode: "allow-list".into(),
                    deny_by_default: true,
                    builtin_tool_keys: vec!["bash".into(), "read_file".into()],
                    skill_ids: vec!["skill-research".into()],
                    mcp_server_names: vec!["browser".into()],
                    plugin_capability_refs: vec!["plugin.browser.capture".into()],
                }),
                permission_envelope: Some(PermissionEnvelope {
                    default_mode: "readonly".into(),
                    max_mode: "workspace-write".into(),
                    escalation_allowed: true,
                    allowed_resource_scopes: vec!["project-shared".into(), "team-shared".into()],
                }),
                memory_policy: Some(MemoryPolicy {
                    durable_scopes: vec!["user-private".into(), "project-shared".into()],
                    write_requires_approval: true,
                    allow_workspace_shared_write: false,
                    max_selections: 4,
                    freshness_required: true,
                }),
                delegation_policy: Some(DelegationPolicy {
                    mode: "single-worker".into(),
                    allow_background_runs: true,
                    allow_parallel_workers: false,
                    max_worker_count: 1,
                }),
                approval_preference: Some(ApprovalPreference {
                    tool_execution: "require-approval".into(),
                    memory_write: "require-approval".into(),
                    mcp_auth: "require-approval".into(),
                    team_spawn: "deny".into(),
                    workflow_escalation: "require-approval".into(),
                }),
                output_contract: Some(OutputContract {
                    primary_format: "markdown".into(),
                    artifact_kinds: vec!["report".into(), "trace".into()],
                    require_structured_summary: true,
                    preserve_lineage: true,
                }),
                shared_capability_policy: Some(SharedCapabilityPolicy {
                    allow_team_inherited_capabilities: false,
                    deny_direct_member_escalation: true,
                    shared_capability_refs: vec!["skill://docs/review".into()],
                }),
            }))
            .expect("create agent");

        let connection = bundle.workspace.state.open_db().expect("open db");
        let reloaded = load_agents(&connection)
            .expect("load agents")
            .into_iter()
            .find(|agent| agent.id == created.id)
            .expect("reloaded agent");

        assert_eq!(reloaded.task_domains, vec!["research", "docs"]);
        assert_eq!(
            reloaded.default_model_strategy,
            DefaultModelStrategy {
                selection_mode: "actor-default".into(),
                preferred_model_ref: Some("claude-sonnet-4-5".into()),
                fallback_model_refs: vec!["gpt-4o".into()],
                allow_turn_override: false,
            }
        );
        assert_eq!(
            reloaded.capability_policy.plugin_capability_refs,
            vec!["plugin.browser.capture"]
        );
        assert_eq!(reloaded.permission_envelope.default_mode, "readonly");
        assert_eq!(reloaded.memory_policy.max_selections, 4);
        assert_eq!(reloaded.delegation_policy.mode, "single-worker");
        assert_eq!(reloaded.approval_preference.team_spawn, "deny");
        assert_eq!(
            reloaded.output_contract.artifact_kinds,
            vec!["report", "trace"]
        );
        assert_eq!(
            reloaded.shared_capability_policy.shared_capability_refs,
            vec!["skill://docs/review"]
        );
        assert_eq!(reloaded.manifest_revision, "asset-manifest/v2");
        assert_eq!(reloaded.import_metadata.origin_kind, "native");
        assert_eq!(reloaded.import_metadata.translation_status, "native");
        assert_eq!(reloaded.trust_metadata.trust_level, "trusted");
        assert!(reloaded.dependency_resolution.is_empty());
    }

    #[test]
    fn list_project_deliverables_returns_only_requested_project_in_updated_order() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let connection = bundle.workspace.state.open_db().expect("open db");

        insert_artifact_record(&connection, "artifact-a-older", "proj-a", "A older", 100);
        insert_artifact_record(&connection, "artifact-b-newest", "proj-b", "B newest", 500);
        insert_artifact_record(&connection, "artifact-a-newest", "proj-a", "A newest", 400);
        insert_artifact_record(&connection, "artifact-a-middle", "proj-a", "A middle", 200);

        let records = runtime()
            .block_on(bundle.workspace.list_project_deliverables("proj-a"))
            .expect("list project deliverables");

        let ids = records
            .iter()
            .map(|record| record.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec!["artifact-a-newest", "artifact-a-middle", "artifact-a-older"]
        );
        assert!(records.iter().all(|record| record.project_id == "proj-a"));
    }

    #[test]
    fn personal_pet_snapshots_and_bindings_are_scoped_by_owner_user() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let owner_session = bootstrap_admin_session(&bundle);
        let analyst_session = create_user_session(&bundle, "analyst", "Analyst");

        let owner_snapshot = runtime()
            .block_on(
                bundle
                    .workspace
                    .get_workspace_pet_snapshot(&owner_session.user_id),
            )
            .expect("owner pet snapshot");
        let analyst_snapshot = runtime()
            .block_on(
                bundle
                    .workspace
                    .get_workspace_pet_snapshot(&analyst_session.user_id),
            )
            .expect("analyst pet snapshot");

        assert_eq!(owner_snapshot.owner_user_id, owner_session.user_id);
        assert_eq!(owner_snapshot.context_scope, "home");
        assert_eq!(analyst_snapshot.owner_user_id, analyst_session.user_id);
        assert_ne!(owner_snapshot.profile.id, analyst_snapshot.profile.id);

        let owner_binding = runtime()
            .block_on(bundle.workspace.bind_workspace_pet_conversation(
                &owner_session.user_id,
                BindPetConversationInput {
                    pet_id: owner_snapshot.profile.id.clone(),
                    conversation_id: "conversation-owner".into(),
                    session_id: Some("session-owner".into()),
                },
            ))
            .expect("bind owner pet");

        let refreshed_owner = runtime()
            .block_on(
                bundle
                    .workspace
                    .get_workspace_pet_snapshot(&owner_session.user_id),
            )
            .expect("refreshed owner pet");
        let refreshed_analyst = runtime()
            .block_on(
                bundle
                    .workspace
                    .get_workspace_pet_snapshot(&analyst_session.user_id),
            )
            .expect("refreshed analyst pet");

        assert_eq!(
            refreshed_owner
                .binding
                .as_ref()
                .map(|binding| binding.conversation_id.as_str()),
            Some("conversation-owner")
        );
        assert_eq!(
            refreshed_owner
                .binding
                .as_ref()
                .map(|binding| binding.owner_user_id.as_str()),
            Some(owner_session.user_id.as_str())
        );
        assert_eq!(owner_binding.context_scope, "home");
        assert!(refreshed_analyst.binding.is_none());
    }

    #[test]
    fn generic_agent_listing_excludes_personal_pet_agents() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let owner_session = bootstrap_admin_session(&bundle);
        let analyst_session = create_user_session(&bundle, "analyst", "Analyst");

        let agents = runtime()
            .block_on(bundle.workspace.list_agents())
            .expect("list agents");

        assert!(
            agents.iter().all(|record| record.asset_role != "pet"),
            "pet agents must be hidden from the generic catalog"
        );
        assert!(bundle
            .workspace
            .state
            .agents
            .lock()
            .expect("agents")
            .iter()
            .any(|record| {
                record.asset_role == "pet"
                    && record.owner_user_id.as_deref() == Some(owner_session.user_id.as_str())
            }));
        assert!(bundle
            .workspace
            .state
            .agents
            .lock()
            .expect("agents")
            .iter()
            .any(|record| {
                record.asset_role == "pet"
                    && record.owner_user_id.as_deref() == Some(analyst_session.user_id.as_str())
            }));
    }

    #[test]
    fn create_team_persists_topology_and_workflow_policy_fields_across_db_reload() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let created = runtime()
            .block_on(bundle.workspace.create_team(UpsertTeamInput {
                workspace_id: DEFAULT_WORKSPACE_ID.into(),
                project_id: None,
                scope: "workspace".into(),
                name: "Research Ops Team".into(),
                avatar: None,
                remove_avatar: None,
                personality: "Leader-coordinated specialists".into(),
                tags: vec!["research".into(), "browser".into()],
                prompt: "Break research work into specialist subruns.".into(),
                builtin_tool_keys: vec!["bash".into()],
                skill_ids: vec!["skill-research".into(), "skill-synthesis".into()],
                mcp_server_names: vec!["browser".into(), "notion".into()],
                description: "Coordinates research and browsing specialists.".into(),
                status: "active".into(),
                task_domains: vec!["research".into(), "browser".into()],
                default_model_strategy: Some(DefaultModelStrategy {
                    selection_mode: "session-selected".into(),
                    preferred_model_ref: Some("claude-sonnet-4-5".into()),
                    fallback_model_refs: vec!["gpt-4o".into()],
                    allow_turn_override: true,
                }),
                capability_policy: Some(CapabilityPolicy {
                    mode: "allow-list".into(),
                    deny_by_default: true,
                    builtin_tool_keys: vec!["bash".into()],
                    skill_ids: vec!["skill-research".into(), "skill-synthesis".into()],
                    mcp_server_names: vec!["browser".into(), "notion".into()],
                    plugin_capability_refs: vec!["plugin.browser.capture".into()],
                }),
                permission_envelope: Some(PermissionEnvelope {
                    default_mode: "workspace-write".into(),
                    max_mode: "danger-full-access".into(),
                    escalation_allowed: true,
                    allowed_resource_scopes: vec!["team-shared".into(), "project-shared".into()],
                }),
                memory_policy: Some(MemoryPolicy {
                    durable_scopes: vec!["team-shared".into(), "project-shared".into()],
                    write_requires_approval: true,
                    allow_workspace_shared_write: false,
                    max_selections: 6,
                    freshness_required: true,
                }),
                delegation_policy: Some(DelegationPolicy {
                    mode: "leader-orchestrated".into(),
                    allow_background_runs: true,
                    allow_parallel_workers: true,
                    max_worker_count: 3,
                }),
                approval_preference: Some(ApprovalPreference {
                    tool_execution: "require-approval".into(),
                    memory_write: "require-approval".into(),
                    mcp_auth: "require-approval".into(),
                    team_spawn: "require-approval".into(),
                    workflow_escalation: "require-approval".into(),
                }),
                output_contract: Some(OutputContract {
                    primary_format: "markdown".into(),
                    artifact_kinds: vec!["brief".into(), "artifact".into()],
                    require_structured_summary: true,
                    preserve_lineage: true,
                }),
                shared_capability_policy: Some(SharedCapabilityPolicy {
                    allow_team_inherited_capabilities: true,
                    deny_direct_member_escalation: true,
                    shared_capability_refs: vec!["skill://research/common".into()],
                }),
                leader_ref: "agent://workspace/lead".into(),
                member_refs: vec![
                    "agent://workspace/research".into(),
                    "agent://workspace/browser".into(),
                ],
                team_topology: Some(TeamTopology {
                    mode: "leader-orchestrated".into(),
                    leader_ref: "agent://workspace/lead".into(),
                    member_refs: vec![
                        "agent://workspace/research".into(),
                        "agent://workspace/browser".into(),
                    ],
                }),
                shared_memory_policy: Some(SharedMemoryPolicy {
                    share_mode: "team-shared".into(),
                    writable_by_workers: true,
                    require_review_before_persist: true,
                }),
                mailbox_policy: Some(MailboxPolicy {
                    mode: "leader-hub".into(),
                    allow_worker_to_worker: false,
                    retain_messages: true,
                }),
                artifact_handoff_policy: Some(ArtifactHandoffPolicy {
                    mode: "leader-reviewed".into(),
                    require_lineage: true,
                    retain_artifacts: true,
                }),
                workflow_affordance: Some(WorkflowAffordance {
                    supported_task_kinds: vec!["research".into(), "browser".into()],
                    background_capable: true,
                    automation_capable: true,
                }),
                worker_concurrency_limit: Some(3),
            }))
            .expect("create team");

        let connection = bundle.workspace.state.open_db().expect("open db");
        let reloaded = load_teams(&connection)
            .expect("load teams")
            .into_iter()
            .find(|team| team.id == created.id)
            .expect("reloaded team");

        assert_eq!(reloaded.leader_ref, "agent://workspace/lead");
        assert_eq!(
            reloaded.member_refs,
            vec!["agent://workspace/research", "agent://workspace/browser"]
        );
        assert_eq!(reloaded.team_topology.mode, "leader-orchestrated");
        assert_eq!(reloaded.shared_memory_policy.share_mode, "team-shared");
        assert_eq!(reloaded.mailbox_policy.mode, "leader-hub");
        assert_eq!(reloaded.artifact_handoff_policy.mode, "leader-reviewed");
        assert_eq!(
            reloaded.workflow_affordance.supported_task_kinds,
            vec!["research", "browser"]
        );
        assert_eq!(reloaded.worker_concurrency_limit, 3);
        assert_eq!(reloaded.delegation_policy.max_worker_count, 3);
        assert_eq!(reloaded.trust_metadata.trust_level, "trusted");
        assert_eq!(reloaded.import_metadata.origin_kind, "native");
    }

