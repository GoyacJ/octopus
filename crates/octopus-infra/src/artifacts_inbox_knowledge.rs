use super::*;
use octopus_core::ProjectTokenUsageProjection;

#[async_trait]
impl ArtifactService for InfraArtifactService {
    async fn list_artifacts(&self) -> Result<Vec<ArtifactRecord>, AppError> {
        let artifacts = load_artifact_records(&self.state.open_db()?)?;
        *self
            .state
            .artifacts
            .lock()
            .map_err(|_| AppError::runtime("artifacts mutex poisoned"))? = artifacts.clone();
        Ok(artifacts)
    }
}

#[async_trait]
impl InboxService for InfraInboxService {
    async fn list_inbox(&self) -> Result<Vec<InboxItemRecord>, AppError> {
        Ok(self
            .state
            .inbox
            .lock()
            .map_err(|_| AppError::runtime("inbox mutex poisoned"))?
            .clone())
    }
}

#[async_trait]
impl KnowledgeService for InfraKnowledgeService {
    async fn list_knowledge(&self) -> Result<Vec<KnowledgeEntryRecord>, AppError> {
        Ok(self
            .state
            .knowledge_records
            .lock()
            .map_err(|_| AppError::runtime("knowledge mutex poisoned"))?
            .iter()
            .map(|record| KnowledgeEntryRecord {
                id: record.id.clone(),
                workspace_id: record.workspace_id.clone(),
                project_id: record.project_id.clone(),
                title: record.title.clone(),
                scope: record.scope.clone(),
                status: record.status.clone(),
                source_type: record.source_type.clone(),
                source_ref: record.source_ref.clone(),
                updated_at: record.updated_at,
            })
            .collect())
    }
}

#[async_trait]
impl ObservationService for InfraObservationService {
    async fn list_trace_events(&self) -> Result<Vec<TraceEventRecord>, AppError> {
        Ok(self
            .state
            .trace_events
            .lock()
            .map_err(|_| AppError::runtime("trace mutex poisoned"))?
            .clone())
    }

    async fn list_audit_records(&self) -> Result<Vec<AuditRecord>, AppError> {
        Ok(self
            .state
            .audit_records
            .lock()
            .map_err(|_| AppError::runtime("audit mutex poisoned"))?
            .clone())
    }

    async fn list_cost_entries(&self) -> Result<Vec<CostLedgerEntry>, AppError> {
        Ok(self
            .state
            .cost_entries
            .lock()
            .map_err(|_| AppError::runtime("cost mutex poisoned"))?
            .clone())
    }

    async fn list_project_token_usage(&self) -> Result<Vec<ProjectTokenUsageProjection>, AppError> {
        let connection = self.state.open_db()?;
        let mut statement = connection
            .prepare(
                "SELECT project_id, used_tokens, updated_at
                 FROM project_token_usage_projections
                 ORDER BY used_tokens DESC, project_id ASC",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let rows = statement
            .query_map([], |row| {
                Ok(ProjectTokenUsageProjection {
                    project_id: row.get(0)?,
                    used_tokens: row.get::<_, i64>(1)?.max(0) as u64,
                    updated_at: row.get::<_, i64>(2)? as u64,
                })
            })
            .map_err(|error| AppError::database(error.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::database(error.to_string()))
    }

    async fn project_used_tokens(&self, project_id: &str) -> Result<u64, AppError> {
        let connection = self.state.open_db()?;
        let used_tokens = connection
            .query_row(
                "SELECT used_tokens
                 FROM project_token_usage_projections
                 WHERE project_id = ?1",
                [project_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))?
            .unwrap_or(0);
        Ok(used_tokens.max(0) as u64)
    }

    async fn append_trace(&self, record: TraceEventRecord) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "INSERT INTO trace_events (id, workspace_id, project_id, run_id, session_id, event_kind, title, detail, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.workspace_id,
                    record.project_id,
                    record.run_id,
                    record.session_id,
                    record.event_kind,
                    record.title,
                    record.detail,
                    record.created_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        append_json_line(
            &self
                .state
                .paths
                .runtime_traces_dir
                .join("trace-events.jsonl"),
            &record,
        )?;
        self.state
            .trace_events
            .lock()
            .map_err(|_| AppError::runtime("trace mutex poisoned"))?
            .push(record);
        Ok(())
    }

    async fn append_audit(&self, record: AuditRecord) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "INSERT INTO audit_records (id, workspace_id, project_id, actor_type, actor_id, action, resource, outcome, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.workspace_id,
                    record.project_id,
                    record.actor_type,
                    record.actor_id,
                    record.action,
                    record.resource,
                    record.outcome,
                    record.created_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        append_json_line(
            &self.state.paths.audit_log_dir.join("audit-records.jsonl"),
            &record,
        )?;
        self.state
            .audit_records
            .lock()
            .map_err(|_| AppError::runtime("audit mutex poisoned"))?
            .push(record);
        Ok(())
    }

    async fn append_cost(&self, record: CostLedgerEntry) -> Result<(), AppError> {
        let connection = self.state.open_db()?;
        connection
            .execute(
                "INSERT INTO cost_entries (id, workspace_id, project_id, run_id, configured_model_id, metric, amount, unit, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.workspace_id,
                    record.project_id,
                    record.run_id,
                    record.configured_model_id,
                    record.metric,
                    record.amount,
                    record.unit,
                    record.created_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        if record.metric == "tokens" {
            if let (Some(project_id), amount) = (record.project_id.as_deref(), record.amount) {
                if amount > 0 {
                    connection
                        .execute(
                            "INSERT INTO project_token_usage_projections (project_id, used_tokens, updated_at)
                             VALUES (?1, ?2, ?3)
                             ON CONFLICT(project_id)
                             DO UPDATE SET
                               used_tokens = project_token_usage_projections.used_tokens + excluded.used_tokens,
                               updated_at = excluded.updated_at",
                            params![project_id, amount, record.created_at as i64],
                        )
                        .map_err(|error| AppError::database(error.to_string()))?;
                }
            }
        }
        append_json_line(
            &self.state.paths.server_log_dir.join("cost-ledger.jsonl"),
            &record,
        )?;
        self.state
            .cost_entries
            .lock()
            .map_err(|_| AppError::runtime("cost mutex poisoned"))?
            .push(record);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_infra_bundle, initialize_workspace, CopyWorkspaceSkillToManagedInput, WorkspacePaths,
    };
    use octopus_core::{
        AccessUserUpsertRequest, AvatarUploadPayload, CapabilityAssetDisablePatch, CostLedgerEntry,
        CreateProjectDeletionRequestInput, CreateProjectRequest, DataPolicyUpsertRequest,
        RegisterBootstrapAdminRequest, ReviewProjectDeletionRequestInput, RoleBindingUpsertRequest,
        RoleUpsertRequest, UpdateProjectRequest,
    };
    use octopus_platform::{
        AccessControlService, AuthService, InboxService, ObservationService, WorkspaceService,
    };
    use rusqlite::Connection;
    use serde_json::Value as JsonValue;

    fn read_json_file(path: &std::path::Path) -> JsonValue {
        let raw = std::fs::read_to_string(path).expect("json file");
        serde_json::from_str(&raw).expect("json document")
    }

    fn avatar_payload() -> AvatarUploadPayload {
        AvatarUploadPayload {
            content_type: "image/png".into(),
            data_base64: "iVBORw0KGgo=".into(),
            file_name: "avatar.png".into(),
            byte_size: 8,
        }
    }

    #[test]
    fn workspace_initialization_creates_expected_layout_and_defaults() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = initialize_workspace(temp.path()).expect("workspace initialized");

        for path in [
            &paths.config_dir,
            &paths.asset_config_dir,
            &paths.data_dir,
            &paths.runtime_dir,
            &paths.logs_dir,
            &paths.tmp_dir,
            &paths.blobs_dir,
            &paths.artifacts_dir,
            &paths.knowledge_dir,
            &paths.inbox_dir,
            &paths.runtime_state_dir,
            &paths.runtime_events_dir,
            &paths.runtime_traces_dir,
            &paths.runtime_approvals_dir,
            &paths.runtime_cache_dir,
            &paths.audit_log_dir,
            &paths.server_log_dir,
        ] {
            assert!(path.exists(), "missing {}", path.display());
        }
        assert!(paths.workspace_config.exists());
        assert!(paths.app_registry_config.exists());
        assert!(paths.db_path.exists());

        let workspace_toml =
            std::fs::read_to_string(&paths.workspace_config).expect("workspace toml");
        assert!(workspace_toml.contains("listen_address = \"127.0.0.1\""));
        assert!(workspace_toml.contains("bootstrap_status = \"setup_required\""));
    }

    #[test]
    fn bundle_exposes_bootstrap_setup_required_state_and_registered_apps() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let bootstrap = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.system_bootstrap())
            .expect("bootstrap");

        assert!(bootstrap.setup_required);
        assert!(!bootstrap.owner_ready);
        assert!(bootstrap
            .registered_apps
            .iter()
            .any(|app| app.id == "octopus-desktop"));
    }

    #[test]
    fn workspace_paths_follow_unified_workspace_layout() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());

        assert_eq!(paths.runtime_state_dir, temp.path().join("runtime/state"));
        assert_eq!(paths.runtime_events_dir, temp.path().join("runtime/events"));
        assert_eq!(paths.audit_log_dir, temp.path().join("logs/audit"));
        assert_eq!(paths.db_path, temp.path().join("data/main.db"));
    }

    #[test]
    fn observation_service_tracks_project_token_usage_projection() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        for record in [
            CostLedgerEntry {
                id: "cost-1".into(),
                workspace_id: "ws-local".into(),
                project_id: Some("proj-redesign".into()),
                run_id: Some("run-1".into()),
                configured_model_id: Some("anthropic-primary".into()),
                metric: "tokens".into(),
                amount: 120,
                unit: "tokens".into(),
                created_at: 1,
            },
            CostLedgerEntry {
                id: "cost-2".into(),
                workspace_id: "ws-local".into(),
                project_id: Some("proj-redesign".into()),
                run_id: Some("run-2".into()),
                configured_model_id: Some("anthropic-primary".into()),
                metric: "turns".into(),
                amount: 1,
                unit: "count".into(),
                created_at: 2,
            },
            CostLedgerEntry {
                id: "cost-3".into(),
                workspace_id: "ws-local".into(),
                project_id: Some("proj-redesign".into()),
                run_id: Some("run-3".into()),
                configured_model_id: Some("anthropic-primary".into()),
                metric: "tokens".into(),
                amount: 5,
                unit: "tokens".into(),
                created_at: 3,
            },
            CostLedgerEntry {
                id: "cost-4".into(),
                workspace_id: "ws-local".into(),
                project_id: Some("proj-other".into()),
                run_id: Some("run-4".into()),
                configured_model_id: Some("anthropic-primary".into()),
                metric: "tokens".into(),
                amount: 999,
                unit: "tokens".into(),
                created_at: 4,
            },
            CostLedgerEntry {
                id: "cost-5".into(),
                workspace_id: "ws-local".into(),
                project_id: Some("proj-redesign".into()),
                run_id: Some("run-5".into()),
                configured_model_id: Some("anthropic-primary".into()),
                metric: "tokens".into(),
                amount: -50,
                unit: "tokens".into(),
                created_at: 5,
            },
        ] {
            runtime
                .block_on(bundle.observation.append_cost(record))
                .expect("append cost");
        }

        let used_tokens = runtime
            .block_on(bundle.observation.project_used_tokens("proj-redesign"))
            .expect("project used tokens");
        assert_eq!(used_tokens, 125);
        let usage_rows = runtime
            .block_on(bundle.observation.list_project_token_usage())
            .expect("project token usage rows");
        assert_eq!(usage_rows[0].project_id, "proj-other");
        assert_eq!(usage_rows[0].used_tokens, 999);
        assert_eq!(usage_rows[1].project_id, "proj-redesign");
        assert_eq!(usage_rows[1].used_tokens, 125);

        let connection = Connection::open(&bundle.paths.db_path).expect("open sqlite");
        let stored_used_tokens: i64 = connection
            .query_row(
                "SELECT used_tokens FROM project_token_usage_projections WHERE project_id = ?1",
                ["proj-redesign"],
                |row| row.get(0),
            )
            .expect("stored project used tokens");
        assert_eq!(stored_used_tokens, 125);

        let reloaded_bundle = build_infra_bundle(temp.path()).expect("reloaded bundle");
        let reloaded_used_tokens = runtime
            .block_on(
                reloaded_bundle
                    .observation
                    .project_used_tokens("proj-redesign"),
            )
            .expect("reloaded project used tokens");
        assert_eq!(reloaded_used_tokens, 125);
    }

    #[test]
    fn bundle_normalizes_legacy_setup_required_state_when_owner_already_exists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = initialize_workspace(temp.path()).expect("workspace initialized");

        let connection = Connection::open(&paths.db_path).expect("open sqlite");
        connection
            .execute(
                "INSERT INTO users (id, username, display_name, status, password_hash, password_state, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    "user-owner",
                    "owner",
                    "Workspace Owner",
                    "active",
                    "hash",
                    "set",
                    1_i64,
                    1_i64,
                ],
            )
            .expect("insert owner user");
        connection
            .execute(
                "INSERT INTO role_bindings (id, role_id, subject_type, subject_id, effect)
                 VALUES (?1, ?2, 'user', ?3, 'allow')",
                rusqlite::params!["binding-user-owner", "owner", "user-owner",],
            )
            .expect("insert owner role binding");
        std::fs::write(
            &paths.workspace_config,
            r#"id = "ws-local"
name = "Octopus Local Workspace"
slug = "local-workspace"
deployment = "local"
bootstrap_status = "setup_required"
owner_user_id = "user-owner"
host = "127.0.0.1"
listen_address = "127.0.0.1"
default_project_id = "proj-redesign"
"#,
        )
        .expect("write legacy workspace config");

        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let bootstrap = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.system_bootstrap())
            .expect("bootstrap");

        assert!(!bootstrap.setup_required);
        assert!(bootstrap.owner_ready);

        let workspace_toml = std::fs::read_to_string(&paths.workspace_config)
            .expect("workspace toml after normalize");
        assert!(workspace_toml.contains("bootstrap_status = \"ready\""));
    }

    #[test]
    fn project_assignments_persist_through_create_and_update() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let created = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Assigned Project".into(),
                description: "Project assignment persistence coverage.".into(),
                resource_directory: "data/projects/assigned-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: Some(octopus_core::ProjectWorkspaceAssignments {
                    models: Some(octopus_core::ProjectModelAssignments {
                        configured_model_ids: vec!["anthropic-primary".into()],
                        default_configured_model_id: "anthropic-primary".into(),
                    }),
                    tools: Some(octopus_core::ProjectToolAssignments {
                        source_keys: vec!["builtin:bash".into()],
                        excluded_source_keys: Vec::new(),
                    }),
                    agents: Some(octopus_core::ProjectAgentAssignments {
                        agent_ids: vec!["agent-architect".into()],
                        team_ids: vec!["team-studio".into()],
                        excluded_agent_ids: Vec::new(),
                        excluded_team_ids: Vec::new(),
                    }),
                }),
            }))
            .expect("created project");
        assert_eq!(
            created
                .assignments
                .as_ref()
                .and_then(|assignments| assignments.models.as_ref())
                .map(|models| models.configured_model_ids.clone()),
            Some(vec!["anthropic-primary".to_string()])
        );

        let updated = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.update_project(
                &created.id,
                UpdateProjectRequest {
                    name: "Assigned Project".into(),
                    description: "Updated assignment persistence coverage.".into(),
                    status: "active".into(),
                    resource_directory: created.resource_directory.clone(),
                    owner_user_id: None,
                    member_user_ids: None,
                    permission_overrides: None,
                    linked_workspace_assets: None,
                    leader_agent_id: None,
                    manager_user_id: None,
                    preset_code: None,
                    assignments: Some(octopus_core::ProjectWorkspaceAssignments {
                        models: Some(octopus_core::ProjectModelAssignments {
                            configured_model_ids: vec!["anthropic-alt".into()],
                            default_configured_model_id: "anthropic-alt".into(),
                        }),
                        tools: Some(octopus_core::ProjectToolAssignments {
                            source_keys: vec!["builtin:bash".into(), "mcp:ops".into()],
                            excluded_source_keys: Vec::new(),
                        }),
                        agents: Some(octopus_core::ProjectAgentAssignments {
                            agent_ids: vec!["agent-architect".into()],
                            team_ids: vec![],
                            excluded_agent_ids: Vec::new(),
                            excluded_team_ids: Vec::new(),
                        }),
                    }),
                },
            ))
            .expect("updated project");
        assert_eq!(
            updated
                .assignments
                .as_ref()
                .and_then(|assignments| assignments.models.as_ref())
                .map(|models| models.configured_model_ids.clone()),
            Some(vec!["anthropic-alt".to_string()])
        );

        let listed = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.list_projects())
            .expect("listed projects");
        let persisted = listed
            .iter()
            .find(|project| project.id == created.id)
            .expect("persisted project");
        assert_eq!(
            persisted
                .assignments
                .as_ref()
                .and_then(|assignments| assignments.tools.as_ref())
                .map(|tools| tools.source_keys.clone()),
            Some(vec!["builtin:bash".to_string(), "mcp:ops".to_string()])
        );
    }

    #[test]
    fn tool_catalog_prefers_higher_priority_skill_roots_and_marks_shadowed_entries() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let codex_skill_dir = bundle.paths.root.join(".codex/skills/help");
        let claude_skill_dir = bundle.paths.root.join(".claude/skills/help");
        std::fs::create_dir_all(&codex_skill_dir).expect("codex skill dir");
        std::fs::create_dir_all(&claude_skill_dir).expect("claude skill dir");
        std::fs::write(
            codex_skill_dir.join("SKILL.md"),
            "---\nname: help\ndescription: Preferred help skill.\n---\n",
        )
        .expect("codex skill");
        std::fs::write(
            claude_skill_dir.join("SKILL.md"),
            "---\nname: help\ndescription: Shadowed help skill.\n---\n",
        )
        .expect("claude skill");

        let projection = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.get_capability_management_projection())
            .expect("management projection");

        let help_entries = projection
            .entries
            .iter()
            .filter(|entry| entry.kind == "skill" && entry.name == "help")
            .collect::<Vec<_>>();
        assert_eq!(help_entries.len(), 2);
        assert!(help_entries.iter().any(|entry| {
            entry.display_path == ".codex/skills/help/SKILL.md"
                && entry.active == Some(true)
                && entry.shadowed_by.is_none()
        }));
        assert!(help_entries.iter().any(|entry| {
            entry.display_path == ".claude/skills/help/SKILL.md"
                && entry.active == Some(false)
                && entry.shadowed_by.as_deref() == Some("project-codex")
        }));
    }

    #[test]
    fn tool_catalog_marks_unsupported_mcp_servers_as_attention() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        std::fs::write(
            bundle.paths.runtime_config_dir.join("workspace.json"),
            r#"{"mcpServers":{"ops":{"type":"http","url":"https://ops.example.test/mcp"}}}"#,
        )
        .expect("workspace runtime config");

        let projection = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.get_capability_management_projection())
            .expect("management projection");

        let ops = projection
            .entries
            .iter()
            .find(|entry| entry.kind == "mcp" && entry.server_name.as_deref() == Some("ops"))
            .expect("ops entry");
        assert_eq!(ops.availability, "attention");
        assert_eq!(ops.scope.as_deref(), Some("workspace"));
        assert!(ops
            .status_detail
            .as_deref()
            .is_some_and(|detail| detail.contains("not supported")));
    }

    #[test]
    fn capability_management_projection_matches_tool_catalog_and_tracks_disabled_assets() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let projection = runtime
            .block_on(bundle.workspace.get_capability_management_projection())
            .expect("management projection");

        let projection_keys = projection
            .entries
            .iter()
            .map(|entry| entry.source_key.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(projection_keys.contains("builtin:bash"));

        let builtin_bash = projection
            .entries
            .iter()
            .find(|entry| entry.source_key == "builtin:bash")
            .expect("builtin bash entry");
        assert!(builtin_bash.enabled);
        assert_eq!(builtin_bash.state, "builtin");

        let updated_projection =
            runtime
                .block_on(bundle.workspace.set_capability_asset_disabled(
                    CapabilityAssetDisablePatch {
                        source_key: "builtin:bash".into(),
                        disabled: true,
                    },
                ))
                .expect("disabled tool");
        assert!(updated_projection
            .entries
            .iter()
            .any(|entry| entry.source_key == "builtin:bash" && entry.disabled));

        let projection_after_disable = runtime
            .block_on(bundle.workspace.get_capability_management_projection())
            .expect("management projection after disable");
        let disabled_bash = projection_after_disable
            .entries
            .iter()
            .find(|entry| entry.source_key == "builtin:bash")
            .expect("disabled bash entry");
        assert!(!disabled_bash.enabled);
        assert_eq!(disabled_bash.state, "disabled");

        let runtime_config_path = bundle.paths.runtime_config_dir.join("workspace.json");
        if runtime_config_path.exists() {
            let runtime_document = read_json_file(&runtime_config_path);
            assert!(
                runtime_document
                    .get("toolCatalog")
                    .and_then(|value| value.get("disabledSourceKeys"))
                    .is_none(),
                "runtime config should no longer persist disabledSourceKeys: {runtime_document}"
            );
        }

        let asset_state = read_json_file(&bundle.paths.workspace_asset_state_path);
        assert_eq!(
            asset_state["assets"]["builtin:bash"]["enabled"],
            JsonValue::Bool(false)
        );
    }

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
}
