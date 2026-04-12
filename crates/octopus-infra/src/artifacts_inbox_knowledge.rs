use super::*;

#[async_trait]
impl ArtifactService for InfraArtifactService {
    async fn list_artifacts(&self) -> Result<Vec<ArtifactRecord>, AppError> {
        Ok(self
            .state
            .artifacts
            .lock()
            .map_err(|_| AppError::runtime("artifacts mutex poisoned"))?
            .clone())
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
                scope: if record.project_id.is_some() {
                    "project".into()
                } else {
                    "workspace".into()
                },
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
        self.state
            .open_db()?
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
    use octopus_core::{CreateProjectRequest, UpdateProjectRequest};
    use octopus_platform::{InboxService, WorkspaceService};
    use rusqlite::Connection;

    #[test]
    fn workspace_initialization_creates_expected_layout_and_defaults() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = initialize_workspace(temp.path()).expect("workspace initialized");

        for path in [
            &paths.config_dir,
            &paths.data_dir,
            &paths.runtime_dir,
            &paths.logs_dir,
            &paths.tmp_dir,
            &paths.blobs_dir,
            &paths.artifacts_dir,
            &paths.knowledge_dir,
            &paths.inbox_dir,
            &paths.runtime_sessions_dir,
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

        assert_eq!(
            paths.runtime_sessions_dir,
            temp.path().join("runtime/sessions")
        );
        assert_eq!(paths.runtime_events_dir, temp.path().join("runtime/events"));
        assert_eq!(paths.audit_log_dir, temp.path().join("logs/audit"));
        assert_eq!(paths.db_path, temp.path().join("data/main.db"));
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
                assignments: Some(octopus_core::ProjectWorkspaceAssignments {
                    models: Some(octopus_core::ProjectModelAssignments {
                        configured_model_ids: vec!["anthropic-primary".into()],
                        default_configured_model_id: "anthropic-primary".into(),
                    }),
                    tools: Some(octopus_core::ProjectToolAssignments {
                        source_keys: vec!["builtin:bash".into()],
                    }),
                    agents: Some(octopus_core::ProjectAgentAssignments {
                        agent_ids: vec!["agent-architect".into()],
                        team_ids: vec!["team-studio".into()],
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
                    assignments: Some(octopus_core::ProjectWorkspaceAssignments {
                        models: Some(octopus_core::ProjectModelAssignments {
                            configured_model_ids: vec!["anthropic-alt".into()],
                            default_configured_model_id: "anthropic-alt".into(),
                        }),
                        tools: Some(octopus_core::ProjectToolAssignments {
                            source_keys: vec!["builtin:bash".into(), "mcp:ops".into()],
                        }),
                        agents: Some(octopus_core::ProjectAgentAssignments {
                            agent_ids: vec!["agent-architect".into()],
                            team_ids: vec![],
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

        let snapshot = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.get_tool_catalog())
            .expect("tool catalog");

        let help_entries = snapshot
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

        let snapshot = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.get_tool_catalog())
            .expect("tool catalog");

        let ops = snapshot
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

        let snapshot = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.get_tool_catalog())
            .expect("tool catalog");
        let source = snapshot
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
                item_type: "approval".into(),
                title: "Runtime approval pending".into(),
                description: "Runtime command needs approval.".into(),
                status: "pending".into(),
                priority: "high".into(),
                actionable: true,
                route_to: Some("/workspaces/ws-local/projects/proj-redesign/runtime".into()),
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
            Some("/workspaces/ws-local/projects/proj-redesign/runtime")
        );
        assert_eq!(items[0].action_label.as_deref(), Some("Review approval"));
    }
}
