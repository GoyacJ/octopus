use super::*;

pub(super) const APPROVAL_AGENT_ID: &str = "agent-task-runtime-approval";
pub(super) const APPROVAL_AGENT_REF: &str = "agent:agent-task-runtime-approval";
pub(super) const CHAINED_APPROVAL_TEAM_REF: &str = "team:team-spawn-workflow-approval";

pub(super) fn open_workspace_db(root: &Path) -> Connection {
    Database::open(&root.join("data").join("main.db"))
        .expect("database")
        .acquire()
        .expect("db")
}

pub(super) fn sample_session() -> SessionRecord {
    SessionRecord {
        id: "sess-1".into(),
        workspace_id: "ws-local".into(),
        user_id: "user-owner".into(),
        client_app_id: "octopus-desktop".into(),
        token: "token".into(),
        status: "active".into(),
        created_at: 1,
        expires_at: None,
    }
}

pub(super) fn avatar_payload() -> octopus_core::AvatarUploadPayload {
    octopus_core::AvatarUploadPayload {
        file_name: "avatar.png".into(),
        content_type: "image/png".into(),
        data_base64: "iVBORw0KGgo=".into(),
        byte_size: 8,
    }
}

pub(super) fn update_request_from_project(project: ProjectRecord) -> UpdateProjectRequest {
    UpdateProjectRequest {
        name: project.name,
        description: project.description,
        status: project.status,
        resource_directory: project.resource_directory,
        owner_user_id: Some(project.owner_user_id),
        member_user_ids: Some(project.member_user_ids),
        permission_overrides: Some(project.permission_overrides),
        leader_agent_id: project.leader_agent_id,
        manager_user_id: project.manager_user_id,
        preset_code: project.preset_code,
        linked_workspace_assets: None,
        assignments: None,
    }
}

pub(super) fn project_scoped_agent_input(
    record: &octopus_core::AgentRecord,
    project_id: &str,
) -> octopus_core::UpsertAgentInput {
    octopus_core::UpsertAgentInput {
        workspace_id: record.workspace_id.clone(),
        project_id: Some(project_id.into()),
        scope: "project".into(),
        name: format!("{} Project Copy", record.name),
        avatar: None,
        remove_avatar: None,
        personality: record.personality.clone(),
        tags: record.tags.clone(),
        prompt: record.prompt.clone(),
        builtin_tool_keys: record.builtin_tool_keys.clone(),
        skill_ids: record.skill_ids.clone(),
        mcp_server_names: record.mcp_server_names.clone(),
        task_domains: record.task_domains.clone(),
        default_model_strategy: Some(record.default_model_strategy.clone()),
        capability_policy: Some(record.capability_policy.clone()),
        permission_envelope: Some(record.permission_envelope.clone()),
        memory_policy: Some(record.memory_policy.clone()),
        delegation_policy: Some(record.delegation_policy.clone()),
        approval_preference: Some(record.approval_preference.clone()),
        output_contract: Some(record.output_contract.clone()),
        shared_capability_policy: Some(record.shared_capability_policy.clone()),
        description: record.description.clone(),
        status: "active".into(),
    }
}

pub(super) fn auth_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).expect("bearer header"),
    );
    headers.insert(
        "x-workspace-id",
        HeaderValue::from_static(DEFAULT_WORKSPACE_ID),
    );
    headers
}

pub(super) async fn visible_workspace_agent_actor_ref(state: &ServerState) -> String {
    let agent = state
        .services
        .workspace
        .list_agents()
        .await
        .expect("list agents")
        .into_iter()
        .find(|record| {
            record.project_id.is_none()
                && record.status == "active"
                && agent_visible_in_generic_catalog(record)
        })
        .expect("workspace agent");
    format!("agent:{}", agent.id)
}

pub(super) async fn bootstrap_owner(state: &ServerState) -> SessionRecord {
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

pub(super) async fn create_user_session(
    state: &ServerState,
    username: &str,
    display_name: &str,
) -> SessionRecord {
    state
        .services
        .access_control
        .create_user(AccessUserUpsertRequest {
            username: username.into(),
            display_name: display_name.into(),
            status: "active".into(),
            password: Some("password123".into()),
            confirm_password: Some("password123".into()),
            reset_password: Some(false),
        })
        .await
        .expect("create user");
    state
        .services
        .auth
        .login(LoginRequest {
            client_app_id: "octopus-desktop".into(),
            username: username.into(),
            password: "password123".into(),
            workspace_id: Some(DEFAULT_WORKSPACE_ID.into()),
        })
        .await
        .expect("login user")
        .session
}

pub(super) async fn seed_task_run(
    state: &ServerState,
    task: &ProjectTaskRecord,
    user_id: &str,
    status: &str,
) -> ProjectTaskRunRecord {
    let started_at = timestamp_now();
    let attention_reasons = match status {
        "waiting_approval" => vec!["needs_approval".into()],
        "waiting_input" => vec!["waiting_input".into()],
        "failed" => vec!["failed".into()],
        _ => Vec::new(),
    };
    let run = ProjectTaskRunRecord {
        id: format!("task-run-{}-{status}", task.id),
        workspace_id: task.workspace_id.clone(),
        project_id: task.project_id.clone(),
        task_id: task.id.clone(),
        trigger_type: "manual".into(),
        status: status.into(),
        session_id: Some(format!("runtime-session-{}-{status}", task.id)),
        conversation_id: Some(format!("conversation-{}-{status}", task.id)),
        runtime_run_id: Some(format!("runtime-run-{}-{status}", task.id)),
        actor_ref: task.default_actor_ref.clone(),
        started_at,
        completed_at: None,
        result_summary: None,
        pending_approval_id: (status == "waiting_approval")
            .then_some(format!("approval-task-run-{}-{status}", task.id)),
        failure_category: None,
        failure_summary: None,
        view_status: if attention_reasons.is_empty() {
            "healthy".into()
        } else {
            "attention".into()
        },
        attention_updated_at: if attention_reasons.is_empty() {
            None
        } else {
            Some(started_at)
        },
        attention_reasons,
        deliverable_refs: Vec::new(),
        artifact_refs: Vec::new(),
        latest_transition: Some(TaskStateTransitionSummary {
            kind: status.into(),
            summary: match status {
                "waiting_approval" => "Task run is waiting for approval.".into(),
                "waiting_input" => "Task run is waiting for input.".into(),
                "failed" => "Task run failed in the runtime.".into(),
                "completed" => "Task run completed in the runtime.".into(),
                _ => "Task run started in the runtime.".into(),
            },
            at: started_at,
            run_id: Some(format!("runtime-run-{}-{status}", task.id)),
        }),
    };
    state
        .services
        .project_tasks
        .save_task_run(run.clone())
        .await
        .expect("save seeded task run");
    state
        .services
        .project_tasks
        .save_task(update_task_record_from_run(task, &run, user_id))
        .await
        .expect("save seeded task projection");
    run
}

pub(super) fn write_runtime_workspace_config(root: &Path) {
    std::env::set_var(
        "OCTOPUS_TEST_ANTHROPIC_API_KEY",
        "test-octopus-server-anthropic-key",
    );
    let path = root.join("config").join("runtime").join("workspace.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("runtime config dir");
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&json!({
            "configuredModels": {
                "opus": {
                    "configuredModelId": "opus",
                    "name": "Opus",
                    "providerId": "anthropic",
                    "modelId": "claude-opus-4-6",
                    "credentialRef": "env:OCTOPUS_TEST_ANTHROPIC_API_KEY",
                    "enabled": true,
                    "source": "workspace"
                },
                "quota-model": {
                    "configuredModelId": "quota-model",
                    "name": "Quota Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": "env:OCTOPUS_TEST_ANTHROPIC_API_KEY",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }))
        .expect("runtime config json"),
    )
    .expect("write runtime config");
}

pub(super) fn write_runtime_workspace_config_with_generation_model(root: &Path) {
    std::env::set_var(
        "OCTOPUS_TEST_GOOGLE_API_KEY",
        "test-octopus-server-google-key",
    );
    let path = root.join("config").join("runtime").join("workspace.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("runtime config dir");
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&json!({
            "configuredModels": {
                "generation-only-model": {
                    "configuredModelId": "generation-only-model",
                    "name": "Generation Only Model",
                    "providerId": "google",
                    "modelId": "gemini-2.5-flash",
                    "credentialRef": "env:OCTOPUS_TEST_GOOGLE_API_KEY",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }))
        .expect("runtime config json"),
    )
    .expect("write runtime config");
}

pub(super) fn insert_approval_required_agent(root: &Path) {
    let connection = open_workspace_db(root);
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, memory_policy_json, delegation_policy_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                APPROVAL_AGENT_ID,
                DEFAULT_WORKSPACE_ID,
                DEFAULT_PROJECT_ID,
                "project",
                "Task Runtime Approval Agent",
                Option::<String>::None,
                "Approver",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Require approval before model execution starts.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for task runtime approval route tests.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({})).expect("permission envelope"),
                serde_json::to_string(&json!({})).expect("memory policy"),
                serde_json::to_string(&json!({})).expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "require-approval",
                    "workflowEscalation": "require-approval"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert approval-required agent");
}

pub(super) fn insert_chained_approval_team(root: &Path) {
    let connection = open_workspace_db(root);
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-spawn-workflow-leader",
                DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Spawn Workflow Leader",
                Option::<String>::None,
                "Coordinator",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Lead the team.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Leader for chained workflow approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert chained leader");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-team-spawn-workflow-worker",
                DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Spawn Workflow Worker",
                Option::<String>::None,
                "Executor",
                serde_json::to_string(&vec!["delivery"]).expect("tags"),
                "Do the delegated work.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Worker for chained workflow approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert chained worker");
    connection
        .execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                "team-spawn-workflow-approval",
                DEFAULT_WORKSPACE_ID,
                Option::<String>::None,
                "workspace",
                "Spawn Workflow Approval Team",
                Option::<String>::None,
                "Approval aware team",
                serde_json::to_string(&vec!["coordination"]).expect("tags"),
                "Delegate after approval, then continue workflow after approval.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "require-approval",
                    "workflowEscalation": "require-approval"
                }))
                .expect("approval preference"),
                "agent:agent-team-spawn-workflow-leader",
                serde_json::to_string(&vec![
                    "agent:agent-team-spawn-workflow-leader",
                    "agent:agent-team-spawn-workflow-worker"
                ])
                .expect("member refs"),
                "Team for chained workflow approval tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert chained approval team");
}

pub(super) async fn seed_runtime_pending_approval_task_run(
    state: &ServerState,
    task: &ProjectTaskRecord,
    user_id: &str,
) -> ProjectTaskRunRecord {
    let runtime_session = state
        .services
        .runtime_session
        .create_session(
            CreateRuntimeSessionInput {
                conversation_id: format!("conversation-{}-approval", task.id),
                project_id: Some(task.project_id.clone()),
                title: format!("{} runtime approval", task.title),
                session_kind: Some("task".into()),
                selected_actor_ref: task.default_actor_ref.clone(),
                selected_configured_model_id: Some("quota-model".into()),
                execution_permission_mode: octopus_core::RUNTIME_PERMISSION_READ_ONLY.into(),
            },
            user_id,
        )
        .await
        .expect("create runtime session");
    let runtime_run = state
        .services
        .runtime_execution
        .submit_turn(
            &runtime_session.summary.id,
            SubmitRuntimeTurnInput {
                content: task_prompt_from_record(task, "manual", None),
                permission_mode: None,
                recall_mode: None,
                ignored_memory_ids: Vec::new(),
                memory_intent: None,
            },
        )
        .await
        .expect("submit task turn");
    assert_eq!(runtime_run.status, "waiting_approval");
    let run = state
        .services
        .project_tasks
        .save_task_run(build_task_run_record(
            task,
            &runtime_session,
            &runtime_run,
            "manual",
            &task.default_actor_ref,
        ))
        .await
        .expect("save runtime-backed task run");
    state
        .services
        .project_tasks
        .save_task(update_task_record_from_run(task, &run, user_id))
        .await
        .expect("save runtime-backed task projection");
    run
}

pub(super) fn sample_resource(visibility: &str, owner_user_id: &str) -> WorkspaceResourceRecord {
    WorkspaceResourceRecord {
        id: "res-1".into(),
        workspace_id: "ws-local".into(),
        project_id: Some("proj-redesign".into()),
        kind: "file".into(),
        name: "brief.md".into(),
        location: Some("data/projects/proj-redesign/resources/brief.md".into()),
        origin: "source".into(),
        scope: "project".into(),
        visibility: visibility.into(),
        owner_user_id: owner_user_id.into(),
        storage_path: Some("data/projects/proj-redesign/resources/brief.md".into()),
        content_type: Some("text/markdown".into()),
        byte_size: Some(12),
        preview_kind: "markdown".into(),
        status: "healthy".into(),
        updated_at: 1,
        tags: Vec::new(),
        source_artifact_id: None,
    }
}

pub(super) fn sample_knowledge(
    scope: &str,
    visibility: &str,
    owner_user_id: Option<&str>,
) -> KnowledgeRecord {
    KnowledgeRecord {
        id: "kn-1".into(),
        workspace_id: "ws-local".into(),
        project_id: if scope == "project" {
            Some("proj-redesign".into())
        } else {
            None
        },
        title: "Knowledge brief".into(),
        summary: "Knowledge summary".into(),
        kind: "shared".into(),
        status: "reviewed".into(),
        source_type: "artifact".into(),
        source_ref: "artifact-1".into(),
        updated_at: 1,
        scope: scope.into(),
        visibility: visibility.into(),
        owner_user_id: owner_user_id.map(str::to_string),
    }
}

pub(super) fn sample_agent(asset_role: &str, owner_user_id: Option<&str>) -> AgentRecord {
    AgentRecord {
        id: format!("agent-{asset_role}"),
        workspace_id: "ws-local".into(),
        project_id: None,
        scope: if asset_role == "pet" {
            "personal".into()
        } else {
            "workspace".into()
        },
        owner_user_id: owner_user_id.map(str::to_string),
        asset_role: asset_role.into(),
        name: format!("{asset_role} agent"),
        avatar_path: None,
        avatar: None,
        personality: "Helpful".into(),
        tags: Vec::new(),
        prompt: "Assist the workspace.".into(),
        builtin_tool_keys: Vec::new(),
        skill_ids: Vec::new(),
        mcp_server_names: Vec::new(),
        task_domains: Vec::new(),
        manifest_revision: "asset-manifest/v2".into(),
        default_model_strategy: octopus_core::default_model_strategy(),
        capability_policy: octopus_core::capability_policy_from_sources(&[], &[], &[]),
        permission_envelope: octopus_core::default_permission_envelope(),
        memory_policy: octopus_core::default_agent_memory_policy(),
        delegation_policy: octopus_core::default_agent_delegation_policy(),
        approval_preference: octopus_core::default_approval_preference(),
        output_contract: octopus_core::default_output_contract(),
        shared_capability_policy: octopus_core::default_agent_shared_capability_policy(),
        integration_source: None,
        trust_metadata: octopus_core::default_asset_trust_metadata(),
        dependency_resolution: Vec::new(),
        import_metadata: octopus_core::default_asset_import_metadata(),
        description: "Test agent".into(),
        status: "active".into(),
        updated_at: 1,
    }
}

pub(super) fn sample_runtime_run_snapshot() -> octopus_core::RuntimeRunSnapshot {
    octopus_core::RuntimeRunSnapshot {
        id: "run-1".into(),
        session_id: "session-1".into(),
        conversation_id: "conversation-1".into(),
        status: "running".into(),
        current_step: "workflow_step".into(),
        started_at: 10,
        updated_at: 20,
        selected_memory: Vec::new(),
        freshness_summary: None,
        pending_memory_proposal: None,
        memory_state_ref: "memory-state-1".into(),
        configured_model_id: Some("quota-model".into()),
        configured_model_name: Some("Quota Model".into()),
        model_id: Some("provider-model".into()),
        consumed_tokens: Some(42),
        next_action: Some("await_workflow".into()),
        config_snapshot_id: "config-1".into(),
        effective_config_hash: "hash-1".into(),
        started_from_scope_set: vec!["workspace".into()],
        run_kind: "primary".into(),
        parent_run_id: None,
        actor_ref: "team:workspace-core".into(),
        delegated_by_tool_call_id: Some("tool-call-1".into()),
        workflow_run: Some("workflow-1".into()),
        workflow_run_detail: Some(octopus_core::RuntimeWorkflowRunDetail {
            workflow_run_id: "workflow-1".into(),
            status: "background_running".into(),
            current_step_id: Some("step-1".into()),
            current_step_label: Some("Worker review".into()),
            total_steps: 3,
            completed_steps: 1,
            background_capable: true,
            steps: vec![octopus_core::RuntimeWorkflowStepSummary {
                step_id: "step-1".into(),
                node_kind: "worker".into(),
                label: "Worker review".into(),
                actor_ref: "agent:workspace-worker".into(),
                run_id: Some("subrun-1".into()),
                parent_run_id: Some("run-1".into()),
                delegated_by_tool_call_id: Some("tool-call-1".into()),
                mailbox_ref: Some("mailbox-1".into()),
                handoff_ref: Some("handoff-1".into()),
                status: "running".into(),
                started_at: 12,
                updated_at: 20,
            }],
            blocking: None,
        }),
        mailbox_ref: Some("mailbox-1".into()),
        handoff_ref: Some("handoff-1".into()),
        background_state: Some("background_running".into()),
        worker_dispatch: Some(octopus_core::RuntimeWorkerDispatchSummary {
            total_subruns: 1,
            active_subruns: 1,
            completed_subruns: 0,
            failed_subruns: 0,
        }),
        approval_state: "not-required".into(),
        approval_target: None,
        auth_target: None,
        usage_summary: octopus_core::RuntimeUsageSummary::default(),
        artifact_refs: vec!["runtime-artifact-run-1".into()],
        deliverable_refs: Vec::new(),
        trace_context: octopus_core::RuntimeTraceContext::default(),
        checkpoint: octopus_core::RuntimeRunCheckpoint::default(),
        capability_plan_summary: octopus_core::RuntimeCapabilityPlanSummary::default(),
        provider_state_summary: Vec::new(),
        pending_mediation: None,
        last_execution_outcome: None,
        last_mediation_outcome: None,
        resolved_target: None,
        requested_actor_kind: Some("team".into()),
        requested_actor_id: Some("team:workspace-core".into()),
        resolved_actor_kind: Some("team".into()),
        resolved_actor_id: Some("team:workspace-core".into()),
        resolved_actor_label: Some("Workspace Core".into()),
    }
}

pub(super) fn sample_runtime_session_detail() -> octopus_core::RuntimeSessionDetail {
    let run = sample_runtime_run_snapshot();
    let workflow = octopus_core::RuntimeWorkflowSummary {
        workflow_run_id: "workflow-1".into(),
        label: "Team workflow".into(),
        status: "background_running".into(),
        total_steps: 3,
        completed_steps: 1,
        current_step_id: Some("step-1".into()),
        current_step_label: Some("Worker review".into()),
        background_capable: true,
        updated_at: 20,
    };
    let mailbox = octopus_core::RuntimeMailboxSummary {
        mailbox_ref: "mailbox-1".into(),
        channel: "leader-hub".into(),
        status: "pending".into(),
        pending_count: 1,
        total_messages: 1,
        updated_at: 20,
    };
    let background = octopus_core::RuntimeBackgroundRunSummary {
        run_id: run.id.clone(),
        workflow_run_id: Some("workflow-1".into()),
        status: "background_running".into(),
        background_capable: true,
        continuation_state: "running".into(),
        blocking: None,
        updated_at: 20,
    };

    octopus_core::RuntimeSessionDetail {
        summary: octopus_core::RuntimeSessionSummary {
            id: "session-1".into(),
            conversation_id: "conversation-1".into(),
            project_id: "project-1".into(),
            title: "Phase 4".into(),
            session_kind: "project".into(),
            status: "running".into(),
            updated_at: 20,
            last_message_preview: Some("Workflow in progress".into()),
            config_snapshot_id: "config-1".into(),
            effective_config_hash: "hash-1".into(),
            started_from_scope_set: vec!["workspace".into()],
            selected_actor_ref: "team:workspace-core".into(),
            manifest_revision: "manifest-1".into(),
            session_policy: octopus_core::RuntimeSessionPolicySnapshot::default(),
            active_run_id: run.id.clone(),
            subrun_count: 1,
            workflow: Some(workflow.clone()),
            pending_mailbox: Some(mailbox.clone()),
            background_run: Some(background.clone()),
            memory_summary: octopus_core::RuntimeMemorySummary::default(),
            memory_selection_summary: octopus_core::RuntimeMemorySelectionSummary::default(),
            pending_memory_proposal_count: 0,
            memory_state_ref: "memory-state-1".into(),
            capability_summary: octopus_core::RuntimeCapabilityPlanSummary::default(),
            provider_state_summary: Vec::new(),
            auth_state_summary: octopus_core::RuntimeAuthStateSummary::default(),
            pending_mediation: None,
            policy_decision_summary: octopus_core::RuntimePolicyDecisionSummary::default(),
            last_execution_outcome: None,
        },
        selected_actor_ref: "team:workspace-core".into(),
        manifest_revision: "manifest-1".into(),
        session_policy: octopus_core::RuntimeSessionPolicySnapshot::default(),
        active_run_id: run.id.clone(),
        subrun_count: 1,
        workflow: Some(workflow),
        pending_mailbox: Some(mailbox),
        background_run: Some(background),
        memory_summary: octopus_core::RuntimeMemorySummary::default(),
        memory_selection_summary: octopus_core::RuntimeMemorySelectionSummary::default(),
        pending_memory_proposal_count: 0,
        memory_state_ref: "memory-state-1".into(),
        capability_summary: octopus_core::RuntimeCapabilityPlanSummary::default(),
        provider_state_summary: Vec::new(),
        auth_state_summary: octopus_core::RuntimeAuthStateSummary::default(),
        pending_mediation: None,
        policy_decision_summary: octopus_core::RuntimePolicyDecisionSummary::default(),
        last_execution_outcome: None,
        run,
        subruns: vec![octopus_core::RuntimeSubrunSummary {
            run_id: "subrun-1".into(),
            parent_run_id: Some("run-1".into()),
            actor_ref: "agent:worker".into(),
            label: "Worker".into(),
            status: "running".into(),
            run_kind: "subrun".into(),
            delegated_by_tool_call_id: Some("tool-call-1".into()),
            workflow_run_id: Some("workflow-1".into()),
            mailbox_ref: Some("mailbox-1".into()),
            handoff_ref: Some("handoff-1".into()),
            started_at: 11,
            updated_at: 20,
        }],
        handoffs: vec![octopus_core::RuntimeHandoffSummary {
            handoff_ref: "handoff-1".into(),
            mailbox_ref: "mailbox-1".into(),
            sender_actor_ref: "team:workspace-core".into(),
            receiver_actor_ref: "agent:worker".into(),
            state: "pending".into(),
            artifact_refs: vec!["runtime-artifact-run-1".into()],
            updated_at: 20,
        }],
        messages: Vec::new(),
        trace: Vec::new(),
        pending_approval: None,
    }
}
