use super::*;
use crate::project_tasks::{
    load_project_task_interventions, load_project_task_runs, load_project_task_scheduler_claims,
    load_project_tasks,
};
use octopus_core::{
    ArtifactVersionReference, BundleAssetDescriptorRecord, ProjectModelAssignments,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

mod config;
mod defaults;
mod loaders_assets;
mod loaders_core;
mod loaders_projects;
mod loaders_runtime;
mod pet;
mod schema_assets;
mod schema_bootstrap_access;
mod schema_bootstrap_core;
mod schema_bootstrap_runtime;
mod schema_projects;
mod schema_resources;
mod schema_runtime;
mod schema_support;
mod startup;

pub(crate) use self::config::{
    initialize_app_registry, initialize_workspace_config, AppRegistryFile, WorkspaceConfigFile,
};
pub(crate) use self::defaults::{
    avatar_data_url, content_hash, default_knowledge_records, default_model_catalog,
    default_project_assignments, default_project_default_permissions,
    default_project_model_assignments, default_project_permission_overrides,
    default_provider_credentials, default_tool_records, default_workspace_resources,
    empty_project_linked_workspace_assets, normalized_project_member_user_ids,
    stored_avatar_data_url,
};
pub(crate) use self::loaders_assets::{
    agent_avatar, load_agents, load_artifact_records, load_bundle_asset_descriptor_records,
    load_knowledge_records, load_project_agent_links, load_project_artifact_records,
    load_project_team_links, load_resources, load_teams,
};
pub(crate) use self::loaders_core::{load_state, load_users};
pub(crate) use self::loaders_projects::{
    load_project_deletion_requests, load_project_promotion_requests, load_projects,
};
pub(crate) use self::loaders_runtime::{
    load_audit_records, load_cost_entries, load_model_catalog, load_provider_credentials,
    load_sessions, load_tools, load_trace_events,
};
pub(crate) use self::pet::{
    default_pet_profile, default_workspace_pet_presence_for, ensure_personal_pet_for_user,
    load_pet_agent_extensions, load_pet_bindings, load_pet_presences,
    load_runtime_messages_for_conversation, pet_context_key,
};
pub(crate) use self::schema_assets::{
    ensure_agent_record_columns, ensure_bundle_asset_descriptor_columns,
    ensure_pet_agent_extension_columns, ensure_pet_projection_columns, ensure_team_record_columns,
    write_agent_record, write_bundle_asset_descriptor_record, write_team_record,
};
pub(crate) use self::schema_bootstrap_access::apply_access_schema_batch;
pub(crate) use self::schema_bootstrap_core::apply_core_schema_batch;
pub(crate) use self::schema_bootstrap_runtime::apply_runtime_schema_batch;
pub(crate) use self::schema_projects::{
    backfill_default_project_assignments, backfill_project_governance,
    backfill_project_resource_directories, ensure_project_agent_link_table,
    ensure_project_assignment_columns, ensure_project_deletion_request_table,
    ensure_project_promotion_request_table, ensure_project_team_link_table,
};
pub(crate) use self::schema_resources::{ensure_knowledge_columns, ensure_resource_columns};
pub(crate) use self::schema_runtime::{
    ensure_cost_entry_columns, ensure_runtime_config_snapshot_columns,
    ensure_runtime_memory_projection_tables, ensure_runtime_phase_four_projection_tables,
    ensure_runtime_run_projection_columns, ensure_runtime_session_projection_columns,
};
pub(crate) use self::schema_support::{
    drop_legacy_access_control_tables, ensure_columns, ensure_project_task_run_columns,
    ensure_user_avatar_columns, json_string, parse_json_or_default, reset_legacy_sessions_table,
    table_columns,
};
pub(crate) use self::startup::{apply_workspace_schema, initialize_database, seed_defaults};

const BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID: &str = "user-owner";
const PERSONAL_PET_ASSET_ROLE: &str = "pet";
const PET_CONTEXT_SCOPE_HOME: &str = "home";
const PET_CONTEXT_SCOPE_PROJECT: &str = "project";
const PERSONAL_PET_SPECIES_REGISTRY: &[&str] = &[
    "duck", "goose", "blob", "cat", "dragon", "octopus", "owl", "penguin", "turtle", "snail",
    "ghost", "axolotl", "capybara", "cactus", "robot", "rabbit", "mushroom", "chonk",
];

#[derive(Debug, Clone)]
pub(crate) struct StoredUser {
    pub(crate) record: UserRecord,
    pub(crate) password_hash: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PetAgentExtensionRecord {
    pub(crate) pet_id: String,
    pub(crate) workspace_id: String,
    pub(crate) owner_user_id: String,
    pub(crate) species: String,
    pub(crate) display_name: String,
    pub(crate) avatar_label: String,
    pub(crate) summary: String,
    pub(crate) greeting: String,
    pub(crate) mood: String,
    pub(crate) favorite_snack: String,
    pub(crate) prompt_hints: Vec<String>,
    pub(crate) fallback_asset: String,
    pub(crate) rive_asset: Option<String>,
    pub(crate) state_machine: Option<String>,
    pub(crate) updated_at: u64,
}

#[derive(Debug)]
pub(crate) struct InfraState {
    pub(crate) paths: WorkspacePaths,
    pub(crate) workspace: Mutex<WorkspaceSummary>,
    pub(crate) workspace_avatar_path: Mutex<Option<String>>,
    pub(crate) workspace_avatar_content_type: Mutex<Option<String>>,
    pub(crate) users: Mutex<Vec<StoredUser>>,
    pub(crate) apps: Mutex<Vec<ClientAppRecord>>,
    pub(crate) sessions: Mutex<Vec<SessionRecord>>,
    pub(crate) projects: Mutex<Vec<ProjectRecord>>,
    pub(crate) project_promotion_requests: Mutex<Vec<ProjectPromotionRequest>>,
    pub(crate) project_deletion_requests: Mutex<Vec<ProjectDeletionRequest>>,
    pub(crate) resources: Mutex<Vec<WorkspaceResourceRecord>>,
    pub(crate) knowledge_records: Mutex<Vec<KnowledgeRecord>>,
    #[allow(dead_code)]
    pub(crate) project_tasks: Mutex<Vec<ProjectTaskRecord>>,
    #[allow(dead_code)]
    pub(crate) project_task_runs: Mutex<Vec<ProjectTaskRunRecord>>,
    #[allow(dead_code)]
    pub(crate) project_task_interventions: Mutex<Vec<ProjectTaskInterventionRecord>>,
    #[allow(dead_code)]
    pub(crate) project_task_scheduler_claims: Mutex<Vec<ProjectTaskSchedulerClaimRecord>>,
    pub(crate) agents: Mutex<Vec<AgentRecord>>,
    pub(crate) project_agent_links: Mutex<Vec<ProjectAgentLinkRecord>>,
    pub(crate) teams: Mutex<Vec<TeamRecord>>,
    pub(crate) project_team_links: Mutex<Vec<ProjectTeamLinkRecord>>,
    pub(crate) model_catalog: Mutex<Vec<ModelCatalogRecord>>,
    pub(crate) provider_credentials: Mutex<Vec<ProviderCredentialRecord>>,
    pub(crate) tools: Mutex<Vec<ToolRecord>>,
    pub(crate) artifacts: Mutex<Vec<ArtifactRecord>>,
    pub(crate) inbox: Mutex<Vec<InboxItemRecord>>,
    pub(crate) trace_events: Mutex<Vec<TraceEventRecord>>,
    pub(crate) audit_records: Mutex<Vec<AuditRecord>>,
    pub(crate) cost_entries: Mutex<Vec<CostLedgerEntry>>,
    pub(crate) pet_extensions: Mutex<HashMap<String, PetAgentExtensionRecord>>,
    pub(crate) pet_presences: Mutex<HashMap<String, PetPresenceState>>,
    pub(crate) pet_bindings: Mutex<HashMap<String, PetConversationBinding>>,
}

impl InfraState {
    pub(crate) fn open_db(&self) -> Result<Connection, AppError> {
        crate::persistence::open_connection(&self.paths)
    }

    pub(crate) fn workspace_snapshot(&self) -> Result<WorkspaceSummary, AppError> {
        self.workspace
            .lock()
            .map_err(|_| AppError::runtime("workspace mutex poisoned"))
            .map(|workspace| workspace.clone())
    }

    pub(crate) fn workspace_id(&self) -> Result<String, AppError> {
        Ok(self.workspace_snapshot()?.id)
    }

    pub(crate) fn save_workspace_config(&self) -> Result<(), AppError> {
        let workspace = self.workspace_snapshot()?;
        let avatar_path = self
            .workspace_avatar_path
            .lock()
            .map_err(|_| AppError::runtime("workspace avatar mutex poisoned"))?
            .clone();
        let avatar_content_type = self
            .workspace_avatar_content_type
            .lock()
            .map_err(|_| AppError::runtime("workspace avatar mutex poisoned"))?
            .clone();
        bootstrap::save_workspace_config_file(
            &self.paths.workspace_config,
            &workspace,
            avatar_path.as_deref(),
            avatar_content_type.as_deref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use octopus_core::ApprovalPreference;
    use std::collections::BTreeMap;

    #[test]
    fn agent_avatar_returns_svg_data_url() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        let relative_path = "data/blobs/avatars/agent-svg.svg";
        let absolute_path = paths.root.join(relative_path);
        fs::create_dir_all(absolute_path.parent().expect("avatar parent")).expect("avatar dir");
        fs::write(
            &absolute_path,
            br#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#,
        )
        .expect("write avatar");

        let avatar = agent_avatar(&paths, Some(relative_path)).expect("avatar");

        assert!(avatar.starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn parse_json_or_default_merges_partial_approval_preference_with_defaults() {
        let parsed: ApprovalPreference = parse_json_or_default(
            r#"{"toolExecution":"require-approval"}"#,
            default_approval_preference,
        );
        let defaults = default_approval_preference();

        assert_eq!(parsed.tool_execution, "require-approval");
        assert_eq!(parsed.memory_write, defaults.memory_write);
        assert_eq!(parsed.mcp_auth, defaults.mcp_auth);
        assert_eq!(parsed.team_spawn, defaults.team_spawn);
        assert_eq!(parsed.workflow_escalation, defaults.workflow_escalation);
    }

    #[test]
    fn runtime_artifact_projection_table_includes_recovery_metadata_columns() {
        let connection = Connection::open_in_memory().expect("in-memory db");

        ensure_runtime_phase_four_projection_tables(&connection).expect("phase four tables");

        let mut statement = connection
            .prepare("PRAGMA table_info(runtime_artifact_projections)")
            .expect("table info statement");
        let columns = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .expect("table info rows")
            .collect::<Result<BTreeMap<_, _>, _>>()
            .expect("collect columns");

        assert_eq!(
            columns.get("artifact_ref").map(String::as_str),
            Some("TEXT")
        );
        assert_eq!(
            columns.get("storage_path").map(String::as_str),
            Some("TEXT")
        );
        assert_eq!(
            columns.get("content_hash").map(String::as_str),
            Some("TEXT")
        );
        assert_eq!(
            columns.get("byte_size").map(String::as_str),
            Some("INTEGER")
        );
        assert_eq!(
            columns.get("content_type").map(String::as_str),
            Some("TEXT")
        );
        assert_eq!(
            columns.get("summary_json").map(String::as_str),
            Some("TEXT")
        );
    }

    #[test]
    fn initialize_database_creates_runtime_secret_records_table() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        crate::initialize_database(&paths).expect("database");

        let connection = paths.database().expect("database").acquire().expect("db");
        let mut statement = connection
            .prepare("PRAGMA table_info(runtime_secret_records)")
            .expect("table info statement");
        let columns = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .expect("table info rows")
            .collect::<Result<BTreeMap<_, _>, _>>()
            .expect("collect columns");

        assert_eq!(columns.get("reference").map(String::as_str), Some("TEXT"));
        assert_eq!(columns.get("ciphertext").map(String::as_str), Some("BLOB"));
        assert_eq!(columns.get("nonce").map(String::as_str), Some("BLOB"));
        assert_eq!(
            columns.get("key_version").map(String::as_str),
            Some("INTEGER")
        );
    }

    #[test]
    fn load_state_hydrates_project_task_projections_from_sqlite() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        super::initialize_workspace_config(&paths).expect("workspace config");
        super::initialize_app_registry(&paths).expect("app registry");
        crate::initialize_database(&paths).expect("database");
        crate::seed_defaults(&paths).expect("seed defaults");

        let connection = paths.database().expect("database").acquire().expect("db");
        connection
            .execute(
                "INSERT INTO project_tasks (
                    id, workspace_id, project_id, title, goal, brief, default_actor_ref, status,
                    schedule_spec, next_run_at, last_run_at, active_task_run_id,
                    latest_result_summary, latest_failure_category, latest_transition_json,
                    view_status, attention_reasons_json, attention_updated_at,
                    analytics_summary_json, context_bundle_json,
                    latest_deliverable_refs_json, latest_artifact_refs_json,
                    created_by, updated_by, created_at, updated_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                    ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15,
                    ?16, ?17, ?18,
                    ?19, ?20,
                    ?21, ?22,
                    ?23, ?24, ?25, ?26
                )",
                rusqlite::params![
                    "task-1",
                    DEFAULT_WORKSPACE_ID,
                    DEFAULT_PROJECT_ID,
                    "Daily Review",
                    "Summarize project state",
                    "Review the latest outputs and prepare a crisp summary.",
                    "actor-ops",
                    "running",
                    Some("manual"),
                    Some(1_711_234_567_i64),
                    Some(1_711_200_000_i64),
                    Some("task-run-1"),
                    Some("Latest summary"),
                    Some("runtime_error"),
                    Some(
                        r#"{"kind":"progressed","summary":"Run is active","at":1711201234,"runId":"task-run-1"}"#
                    ),
                    "attention",
                    r#"["waiting_input"]"#,
                    Some(1_711_201_235_i64),
                    r#"{"runCount":3,"manualRunCount":2,"scheduledRunCount":1,"completionCount":1,"failureCount":1,"takeoverCount":0,"approvalRequiredCount":1,"averageRunDurationMs":1200,"lastSuccessfulRunAt":1711200000}"#,
                    r#"{"refs":[{"kind":"resource","refId":"res-handbook","title":"Workspace Handbook","pinMode":"snapshot"}],"pinnedInstructions":"Always cite the latest state.","resolutionMode":"explicit_only","lastResolvedAt":1711201000}"#,
                    r#"[{"artifactId":"artifact-deliverable","version":2,"title":"Weekly Summary","previewKind":"markdown","updatedAt":1711201100,"contentType":"text/markdown"}]"#,
                    r#"[{"artifactId":"artifact-trace","version":1,"title":"Execution Trace","previewKind":"trace","updatedAt":1711201120,"contentType":"application/json"}]"#,
                    "user-owner",
                    Some("user-editor"),
                    1_711_100_000_i64,
                    1_711_201_300_i64,
                ],
            )
            .expect("insert project task");
        connection
            .execute(
                "INSERT INTO project_task_runs (
                    id, workspace_id, project_id, task_id, trigger_type, status,
                    session_id, conversation_id, runtime_run_id, actor_ref,
                    started_at, completed_at, result_summary,
                    failure_category, failure_summary,
                    view_status, attention_reasons_json, attention_updated_at,
                    deliverable_refs_json, artifact_refs_json, latest_transition_json
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6,
                    ?7, ?8, ?9, ?10,
                    ?11, ?12, ?13,
                    ?14, ?15,
                    ?16, ?17, ?18,
                    ?19, ?20, ?21
                )",
                rusqlite::params![
                    "task-run-1",
                    DEFAULT_WORKSPACE_ID,
                    DEFAULT_PROJECT_ID,
                    "task-1",
                    "manual",
                    "running",
                    Some("session-1"),
                    Some("conversation-1"),
                    Some("runtime-run-1"),
                    "actor-ops",
                    1_711_200_100_i64,
                    Option::<i64>::None,
                    Some("Interim result"),
                    Option::<String>::None,
                    Option::<String>::None,
                    "attention",
                    r#"["waiting_input"]"#,
                    Some(1_711_200_900_i64),
                    r#"[{"artifactId":"artifact-deliverable","version":2,"title":"Weekly Summary","previewKind":"markdown","updatedAt":1711201100,"contentType":"text/markdown"}]"#,
                    r#"[{"artifactId":"artifact-trace","version":1,"title":"Execution Trace","previewKind":"trace","updatedAt":1711201120,"contentType":"application/json"}]"#,
                    Some(
                        r#"{"kind":"progressed","summary":"Waiting for user input","at":1711200900,"runId":"task-run-1"}"#
                    ),
                ],
            )
            .expect("insert project task run");
        connection
            .execute(
                "INSERT INTO project_task_interventions (
                    id, workspace_id, project_id, task_id, task_run_id, type,
                    payload_json, created_by, created_at, applied_to_session_id, status
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6,
                    ?7, ?8, ?9, ?10, ?11
                )",
                rusqlite::params![
                    "task-intervention-1",
                    DEFAULT_WORKSPACE_ID,
                    DEFAULT_PROJECT_ID,
                    "task-1",
                    Some("task-run-1"),
                    "edit_brief",
                    r#"{"brief":"Focus on blockers first."}"#,
                    "user-owner",
                    1_711_200_950_i64,
                    Some("session-1"),
                    "applied",
                ],
            )
            .expect("insert project task intervention");
        connection
            .execute(
                "INSERT INTO project_task_scheduler_claims (
                    task_id, workspace_id, project_id, claim_token, claimed_by,
                    claim_until, last_dispatched_at, last_evaluated_at, updated_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5,
                    ?6, ?7, ?8, ?9
                )",
                rusqlite::params![
                    "task-1",
                    DEFAULT_WORKSPACE_ID,
                    DEFAULT_PROJECT_ID,
                    Some("claim-token-1"),
                    Some("scheduler-worker-1"),
                    Some(1_711_201_500_i64),
                    Some(1_711_201_000_i64),
                    Some(1_711_201_200_i64),
                    1_711_201_300_i64,
                ],
            )
            .expect("insert project task scheduler claim");

        let state = load_state(paths).expect("load state");

        let project_tasks = state.project_tasks.lock().expect("project tasks lock");
        assert_eq!(project_tasks.len(), 1);
        let project_task = &project_tasks[0];
        assert_eq!(project_task.id, "task-1");
        assert_eq!(project_task.context_bundle.refs.len(), 1);
        assert_eq!(project_task.context_bundle.refs[0].ref_id, "res-handbook");
        assert_eq!(project_task.attention_reasons, vec!["waiting_input"]);
        assert_eq!(
            project_task
                .latest_transition
                .as_ref()
                .map(|transition| transition.run_id.as_deref()),
            Some(Some("task-run-1"))
        );
        assert_eq!(project_task.analytics_summary.run_count, 3);
        drop(project_tasks);

        let project_task_runs = state.project_task_runs.lock().expect("task runs lock");
        assert_eq!(project_task_runs.len(), 1);
        assert_eq!(project_task_runs[0].task_id, "task-1");
        assert_eq!(
            project_task_runs[0].session_id.as_deref(),
            Some("session-1")
        );
        assert_eq!(
            project_task_runs[0].deliverable_refs[0].artifact_id,
            "artifact-deliverable"
        );
        drop(project_task_runs);

        let project_task_interventions = state
            .project_task_interventions
            .lock()
            .expect("task interventions lock");
        assert_eq!(project_task_interventions.len(), 1);
        assert_eq!(project_task_interventions[0].task_id, "task-1");
        assert_eq!(
            project_task_interventions[0]
                .payload
                .get("brief")
                .and_then(serde_json::Value::as_str),
            Some("Focus on blockers first.")
        );

        let project_task_scheduler_claims = state
            .project_task_scheduler_claims
            .lock()
            .expect("task scheduler claims lock");
        assert_eq!(project_task_scheduler_claims.len(), 1);
        assert_eq!(project_task_scheduler_claims[0].task_id, "task-1");
        assert_eq!(
            project_task_scheduler_claims[0].claimed_by.as_deref(),
            Some("scheduler-worker-1")
        );
    }
}
