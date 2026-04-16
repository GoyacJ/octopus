use super::*;
use octopus_core::ArtifactVersionReference;
use octopus_core::{
    default_agent_asset_role, BundleAssetDescriptorRecord, ProjectModelAssignments,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

const BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID: &str = "user-owner";
const PERSONAL_PET_ASSET_ROLE: &str = "pet";
const PET_CONTEXT_SCOPE_HOME: &str = "home";
const PET_CONTEXT_SCOPE_PROJECT: &str = "project";
const PERSONAL_PET_SPECIES_REGISTRY: &[&str] = &[
    "duck", "goose", "blob", "cat", "dragon", "octopus", "owl", "penguin", "turtle", "snail",
    "ghost", "axolotl", "capybara", "cactus", "robot", "rabbit", "mushroom", "chonk",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct WorkspaceConfigFile {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) slug: String,
    pub(super) deployment: String,
    pub(super) bootstrap_status: String,
    pub(super) owner_user_id: Option<String>,
    pub(super) host: String,
    pub(super) listen_address: String,
    pub(super) default_project_id: String,
    #[serde(default = "default_project_default_permissions")]
    pub(super) project_default_permissions: ProjectDefaultPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct AppRegistryFile {
    pub(super) apps: Vec<ClientAppRecord>,
}

pub(super) fn default_project_default_permissions() -> ProjectDefaultPermissions {
    ProjectDefaultPermissions {
        agents: "allow".into(),
        resources: "allow".into(),
        tools: "allow".into(),
        knowledge: "allow".into(),
    }
}

pub(super) fn default_project_permission_overrides() -> ProjectPermissionOverrides {
    ProjectPermissionOverrides {
        agents: "inherit".into(),
        resources: "inherit".into(),
        tools: "inherit".into(),
        knowledge: "inherit".into(),
    }
}

pub(super) fn empty_project_linked_workspace_assets() -> ProjectLinkedWorkspaceAssets {
    ProjectLinkedWorkspaceAssets {
        agent_ids: Vec::new(),
        resource_ids: Vec::new(),
        tool_source_keys: Vec::new(),
        knowledge_ids: Vec::new(),
    }
}

pub(super) fn default_project_model_assignments() -> ProjectModelAssignments {
    ProjectModelAssignments {
        configured_model_ids: vec!["claude-sonnet-4-5".into()],
        default_configured_model_id: "claude-sonnet-4-5".into(),
    }
}

pub(super) fn default_project_assignments() -> ProjectWorkspaceAssignments {
    ProjectWorkspaceAssignments {
        models: Some(default_project_model_assignments()),
        tools: None,
        agents: None,
    }
}

pub(super) fn normalized_project_member_user_ids(
    owner_user_id: &str,
    member_user_ids: Vec<String>,
) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut normalized = Vec::new();

    if !owner_user_id.trim().is_empty() && seen.insert(owner_user_id.to_string()) {
        normalized.push(owner_user_id.to_string());
    }

    for user_id in member_user_ids
        .into_iter()
        .map(|value| value.trim().to_string())
    {
        if user_id.is_empty() || !seen.insert(user_id.clone()) {
            continue;
        }
        normalized.push(user_id);
    }

    normalized
}

#[derive(Debug, Clone)]
pub(super) struct StoredUser {
    pub(super) record: UserRecord,
    pub(super) password_hash: String,
}

#[derive(Debug, Clone)]
pub(super) struct PetAgentExtensionRecord {
    pub(super) pet_id: String,
    pub(super) workspace_id: String,
    pub(super) owner_user_id: String,
    pub(super) species: String,
    pub(super) display_name: String,
    pub(super) avatar_label: String,
    pub(super) summary: String,
    pub(super) greeting: String,
    pub(super) mood: String,
    pub(super) favorite_snack: String,
    pub(super) prompt_hints: Vec<String>,
    pub(super) fallback_asset: String,
    pub(super) rive_asset: Option<String>,
    pub(super) state_machine: Option<String>,
    pub(super) updated_at: u64,
}

#[derive(Debug)]
pub(super) struct InfraState {
    pub(super) paths: WorkspacePaths,
    pub(super) workspace: Mutex<WorkspaceSummary>,
    pub(super) users: Mutex<Vec<StoredUser>>,
    pub(super) apps: Mutex<Vec<ClientAppRecord>>,
    pub(super) sessions: Mutex<Vec<SessionRecord>>,
    pub(super) projects: Mutex<Vec<ProjectRecord>>,
    pub(super) project_promotion_requests: Mutex<Vec<ProjectPromotionRequest>>,
    pub(super) resources: Mutex<Vec<WorkspaceResourceRecord>>,
    pub(super) knowledge_records: Mutex<Vec<KnowledgeRecord>>,
    pub(super) agents: Mutex<Vec<AgentRecord>>,
    pub(super) project_agent_links: Mutex<Vec<ProjectAgentLinkRecord>>,
    pub(super) teams: Mutex<Vec<TeamRecord>>,
    pub(super) project_team_links: Mutex<Vec<ProjectTeamLinkRecord>>,
    pub(super) model_catalog: Mutex<Vec<ModelCatalogRecord>>,
    pub(super) provider_credentials: Mutex<Vec<ProviderCredentialRecord>>,
    pub(super) tools: Mutex<Vec<ToolRecord>>,
    pub(super) automations: Mutex<Vec<AutomationRecord>>,
    pub(super) artifacts: Mutex<Vec<ArtifactRecord>>,
    pub(super) inbox: Mutex<Vec<InboxItemRecord>>,
    pub(super) trace_events: Mutex<Vec<TraceEventRecord>>,
    pub(super) audit_records: Mutex<Vec<AuditRecord>>,
    pub(super) cost_entries: Mutex<Vec<CostLedgerEntry>>,
    pub(super) pet_extensions: Mutex<HashMap<String, PetAgentExtensionRecord>>,
    pub(super) pet_presences: Mutex<HashMap<String, PetPresenceState>>,
    pub(super) pet_bindings: Mutex<HashMap<String, PetConversationBinding>>,
}

impl InfraState {
    pub(super) fn open_db(&self) -> Result<Connection, AppError> {
        Connection::open(&self.paths.db_path).map_err(|error| AppError::database(error.to_string()))
    }

    pub(super) fn workspace_snapshot(&self) -> Result<WorkspaceSummary, AppError> {
        self.workspace
            .lock()
            .map_err(|_| AppError::runtime("workspace mutex poisoned"))
            .map(|workspace| workspace.clone())
    }

    pub(super) fn workspace_id(&self) -> Result<String, AppError> {
        Ok(self.workspace_snapshot()?.id)
    }

    pub(super) fn save_workspace_config(&self) -> Result<(), AppError> {
        let workspace = self.workspace_snapshot()?;
        bootstrap::save_workspace_config_file(&self.paths.workspace_config, &workspace)
    }
}

pub(super) fn initialize_workspace_config(paths: &WorkspacePaths) -> Result<(), AppError> {
    if paths.workspace_config.exists() {
        return Ok(());
    }

    let config = WorkspaceConfigFile {
        id: DEFAULT_WORKSPACE_ID.into(),
        name: "Octopus Local Workspace".into(),
        slug: "local-workspace".into(),
        deployment: "local".into(),
        bootstrap_status: "setup_required".into(),
        owner_user_id: None,
        host: "127.0.0.1".into(),
        listen_address: "127.0.0.1".into(),
        default_project_id: DEFAULT_PROJECT_ID.into(),
        project_default_permissions: ProjectDefaultPermissions {
            agents: "allow".into(),
            resources: "allow".into(),
            tools: "allow".into(),
            knowledge: "allow".into(),
        },
    };
    fs::write(&paths.workspace_config, toml::to_string_pretty(&config)?)?;
    Ok(())
}

pub(super) fn initialize_app_registry(paths: &WorkspacePaths) -> Result<(), AppError> {
    if paths.app_registry_config.exists() {
        return Ok(());
    }

    let registry = AppRegistryFile {
        apps: default_client_apps(),
    };
    fs::write(
        &paths.app_registry_config,
        toml::to_string_pretty(&registry)?,
    )?;
    Ok(())
}

pub(super) fn initialize_database(paths: &WorkspacePaths) -> Result<(), AppError> {
    let connection =
        Connection::open(&paths.db_path).map_err(|error| AppError::database(error.to_string()))?;
    drop_legacy_access_control_tables(&connection)?;
    reset_legacy_sessions_table(&connection)?;

    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS users (
              id TEXT PRIMARY KEY,
              username TEXT NOT NULL UNIQUE,
              display_name TEXT NOT NULL,
              avatar_path TEXT,
              avatar_content_type TEXT,
              avatar_byte_size INTEGER,
              avatar_content_hash TEXT,
              status TEXT NOT NULL,
              password_hash TEXT NOT NULL,
              password_state TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS client_apps (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              platform TEXT NOT NULL,
              status TEXT NOT NULL,
              first_party INTEGER NOT NULL,
              allowed_origins TEXT NOT NULL,
              allowed_hosts TEXT NOT NULL,
              session_policy TEXT NOT NULL,
              default_scopes TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              user_id TEXT NOT NULL,
              client_app_id TEXT NOT NULL,
              token TEXT NOT NULL UNIQUE,
              status TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              expires_at INTEGER
            );
            CREATE TABLE IF NOT EXISTS projects (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              name TEXT NOT NULL,
              status TEXT NOT NULL,
              description TEXT NOT NULL,
              resource_directory TEXT NOT NULL,
              assignments_json TEXT,
              owner_user_id TEXT,
              member_user_ids_json TEXT,
              permission_overrides_json TEXT,
              linked_workspace_assets_json TEXT
            );
            CREATE TABLE IF NOT EXISTS project_promotion_requests (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              asset_type TEXT NOT NULL,
              asset_id TEXT NOT NULL,
              requested_by_user_id TEXT NOT NULL,
              submitted_by_owner_user_id TEXT NOT NULL,
              required_workspace_capability TEXT NOT NULL,
              status TEXT NOT NULL,
              reviewed_by_user_id TEXT,
              review_comment TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              reviewed_at INTEGER
            );
            CREATE TABLE IF NOT EXISTS resources (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              kind TEXT NOT NULL,
              name TEXT NOT NULL,
              location TEXT,
              origin TEXT NOT NULL,
              scope TEXT NOT NULL,
              visibility TEXT NOT NULL,
              owner_user_id TEXT NOT NULL,
              storage_path TEXT,
              content_type TEXT,
              byte_size INTEGER,
              preview_kind TEXT NOT NULL,
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL,
              tags TEXT NOT NULL,
              source_artifact_id TEXT
            );
            CREATE TABLE IF NOT EXISTS knowledge_records (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              title TEXT NOT NULL,
              summary TEXT NOT NULL,
              kind TEXT NOT NULL,
              scope TEXT,
              status TEXT NOT NULL,
              visibility TEXT,
              owner_user_id TEXT,
              source_type TEXT NOT NULL,
              source_ref TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS artifact_records (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              session_id TEXT NOT NULL,
              run_id TEXT NOT NULL,
              source_message_id TEXT,
              parent_artifact_id TEXT,
              title TEXT NOT NULL,
              status TEXT NOT NULL,
              preview_kind TEXT NOT NULL,
              latest_version INTEGER NOT NULL,
              promotion_state TEXT NOT NULL DEFAULT 'not-promoted',
              promotion_knowledge_id TEXT,
              updated_at INTEGER NOT NULL,
              storage_path TEXT,
              content_hash TEXT,
              byte_size INTEGER,
              content_type TEXT
            );
            CREATE TABLE IF NOT EXISTS artifact_versions (
              artifact_id TEXT NOT NULL,
              version INTEGER NOT NULL,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              session_id TEXT,
              run_id TEXT,
              source_message_id TEXT,
              parent_version INTEGER,
              title TEXT NOT NULL,
              preview_kind TEXT NOT NULL,
              updated_at INTEGER NOT NULL,
              storage_path TEXT NOT NULL,
              content_hash TEXT NOT NULL,
              byte_size INTEGER NOT NULL DEFAULT 0,
              content_type TEXT,
              PRIMARY KEY (artifact_id, version)
            );
            CREATE INDEX IF NOT EXISTS artifact_records_project_updated_idx
              ON artifact_records (project_id, updated_at DESC, id ASC);
            CREATE INDEX IF NOT EXISTS artifact_versions_artifact_updated_idx
              ON artifact_versions (artifact_id, version DESC);
            CREATE TABLE IF NOT EXISTS agents (
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
              task_domains TEXT NOT NULL DEFAULT '[]',
              manifest_revision TEXT NOT NULL DEFAULT 'asset-manifest/v2',
              default_model_strategy_json TEXT NOT NULL DEFAULT '{}',
              capability_policy_json TEXT NOT NULL DEFAULT '{}',
              permission_envelope_json TEXT NOT NULL DEFAULT '{}',
              memory_policy_json TEXT NOT NULL DEFAULT '{}',
              delegation_policy_json TEXT NOT NULL DEFAULT '{}',
              approval_preference_json TEXT NOT NULL DEFAULT '{}',
              output_contract_json TEXT NOT NULL DEFAULT '{}',
              shared_capability_policy_json TEXT NOT NULL DEFAULT '{}',
              integration_source_json TEXT,
              trust_metadata_json TEXT NOT NULL DEFAULT '{}',
              dependency_resolution_json TEXT NOT NULL DEFAULT '[]',
              import_metadata_json TEXT NOT NULL DEFAULT '{}',
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS project_agent_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              agent_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, agent_id)
            );
            CREATE TABLE IF NOT EXISTS teams (
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
              task_domains TEXT NOT NULL DEFAULT '[]',
              manifest_revision TEXT NOT NULL DEFAULT 'asset-manifest/v2',
              default_model_strategy_json TEXT NOT NULL DEFAULT '{}',
              capability_policy_json TEXT NOT NULL DEFAULT '{}',
              permission_envelope_json TEXT NOT NULL DEFAULT '{}',
              memory_policy_json TEXT NOT NULL DEFAULT '{}',
              delegation_policy_json TEXT NOT NULL DEFAULT '{}',
              approval_preference_json TEXT NOT NULL DEFAULT '{}',
              output_contract_json TEXT NOT NULL DEFAULT '{}',
              shared_capability_policy_json TEXT NOT NULL DEFAULT '{}',
              leader_agent_id TEXT,
              member_agent_ids TEXT NOT NULL,
              leader_ref TEXT NOT NULL DEFAULT '',
              member_refs TEXT NOT NULL DEFAULT '[]',
              team_topology_json TEXT NOT NULL DEFAULT '{}',
              shared_memory_policy_json TEXT NOT NULL DEFAULT '{}',
              mailbox_policy_json TEXT NOT NULL DEFAULT '{}',
              artifact_handoff_policy_json TEXT NOT NULL DEFAULT '{}',
              workflow_affordance_json TEXT NOT NULL DEFAULT '{}',
              worker_concurrency_limit INTEGER NOT NULL DEFAULT 1,
              integration_source_json TEXT,
              trust_metadata_json TEXT NOT NULL DEFAULT '{}',
              dependency_resolution_json TEXT NOT NULL DEFAULT '[]',
              import_metadata_json TEXT NOT NULL DEFAULT '{}',
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS bundle_asset_descriptors (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              scope TEXT NOT NULL,
              asset_kind TEXT NOT NULL,
              source_id TEXT NOT NULL,
              display_name TEXT NOT NULL,
              source_path TEXT NOT NULL,
              storage_path TEXT NOT NULL,
              content_hash TEXT NOT NULL,
              byte_size INTEGER NOT NULL,
              manifest_revision TEXT NOT NULL DEFAULT 'asset-manifest/v2',
              task_domains_json TEXT NOT NULL DEFAULT '[]',
              translation_mode TEXT NOT NULL DEFAULT 'native',
              trust_metadata_json TEXT NOT NULL DEFAULT '{}',
              dependency_resolution_json TEXT NOT NULL DEFAULT '[]',
              import_metadata_json TEXT NOT NULL DEFAULT '{}',
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS project_team_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              team_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, team_id)
            );
            CREATE TABLE IF NOT EXISTS model_catalog (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              label TEXT NOT NULL,
              provider TEXT NOT NULL,
              description TEXT NOT NULL,
              recommended_for TEXT NOT NULL,
              availability TEXT NOT NULL,
              default_permission TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS provider_credentials (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              provider TEXT NOT NULL,
              name TEXT NOT NULL,
              base_url TEXT,
              status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tools (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              kind TEXT NOT NULL,
              name TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              permission_mode TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS automations (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              title TEXT NOT NULL,
              description TEXT NOT NULL,
              cadence TEXT NOT NULL,
              owner_type TEXT NOT NULL,
              owner_id TEXT NOT NULL,
              status TEXT NOT NULL,
              next_run_at INTEGER,
              last_run_at INTEGER,
              output TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS org_units (
              id TEXT PRIMARY KEY,
              parent_id TEXT,
              code TEXT NOT NULL UNIQUE,
              name TEXT NOT NULL,
              status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS positions (
              id TEXT PRIMARY KEY,
              code TEXT NOT NULL UNIQUE,
              name TEXT NOT NULL,
              status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS user_groups (
              id TEXT PRIMARY KEY,
              code TEXT NOT NULL UNIQUE,
              name TEXT NOT NULL,
              status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS user_org_assignments (
              user_id TEXT NOT NULL,
              org_unit_id TEXT NOT NULL,
              is_primary INTEGER NOT NULL,
              position_ids TEXT NOT NULL,
              user_group_ids TEXT NOT NULL,
              PRIMARY KEY (user_id, org_unit_id)
            );
            CREATE TABLE IF NOT EXISTS access_roles (
              id TEXT PRIMARY KEY,
              code TEXT NOT NULL UNIQUE,
              name TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              permission_codes TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS role_bindings (
              id TEXT PRIMARY KEY,
              role_id TEXT NOT NULL,
              subject_type TEXT NOT NULL,
              subject_id TEXT NOT NULL,
              effect TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS data_policies (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              subject_type TEXT NOT NULL,
              subject_id TEXT NOT NULL,
              resource_type TEXT NOT NULL,
              scope_type TEXT NOT NULL,
              project_ids TEXT NOT NULL,
              tags TEXT NOT NULL,
              classifications TEXT NOT NULL,
              effect TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS resource_policies (
              id TEXT PRIMARY KEY,
              subject_type TEXT NOT NULL,
              subject_id TEXT NOT NULL,
              resource_type TEXT NOT NULL,
              resource_id TEXT NOT NULL,
              action_name TEXT NOT NULL,
              effect TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS menu_policies (
              menu_id TEXT PRIMARY KEY,
              enabled INTEGER NOT NULL,
              order_value INTEGER NOT NULL,
              group_key TEXT,
              visibility TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS protected_resources (
              resource_type TEXT NOT NULL,
              resource_id TEXT NOT NULL,
              resource_subtype TEXT,
              project_id TEXT,
              tags TEXT NOT NULL,
              classification TEXT NOT NULL,
              owner_subject_type TEXT,
              owner_subject_id TEXT,
              PRIMARY KEY (resource_type, resource_id)
            );
            CREATE TABLE IF NOT EXISTS audit_records (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              actor_type TEXT NOT NULL,
              actor_id TEXT NOT NULL,
              action TEXT NOT NULL,
              resource TEXT NOT NULL,
              outcome TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS trace_events (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              run_id TEXT,
              session_id TEXT,
              event_kind TEXT NOT NULL,
              title TEXT NOT NULL,
              detail TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS cost_entries (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              run_id TEXT,
              configured_model_id TEXT,
              metric TEXT NOT NULL,
              amount INTEGER NOT NULL,
              unit TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS configured_model_usage_projections (
              configured_model_id TEXT PRIMARY KEY,
              used_tokens INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS project_token_usage_projections (
              project_id TEXT PRIMARY KEY,
              used_tokens INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_config_snapshots (
              id TEXT PRIMARY KEY,
              effective_config_hash TEXT NOT NULL,
              started_from_scope_set TEXT NOT NULL,
              source_refs TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              effective_config_json TEXT
            );
            CREATE TABLE IF NOT EXISTS runtime_session_projections (
              id TEXT PRIMARY KEY,
              conversation_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              title TEXT NOT NULL,
              session_kind TEXT NOT NULL DEFAULT 'project',
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL,
              last_message_preview TEXT,
              config_snapshot_id TEXT NOT NULL,
              effective_config_hash TEXT NOT NULL,
              started_from_scope_set TEXT NOT NULL,
              selected_actor_ref TEXT NOT NULL DEFAULT '',
              manifest_revision TEXT NOT NULL DEFAULT '',
              active_run_id TEXT NOT NULL DEFAULT '',
              subrun_count INTEGER NOT NULL DEFAULT 0,
              workflow_run_id TEXT,
              workflow_status TEXT,
              workflow_total_steps INTEGER NOT NULL DEFAULT 0,
              workflow_completed_steps INTEGER NOT NULL DEFAULT 0,
              workflow_current_step_id TEXT,
              workflow_current_step_label TEXT,
              workflow_background_capable INTEGER NOT NULL DEFAULT 0,
              pending_mailbox_ref TEXT,
              pending_mailbox_count INTEGER NOT NULL DEFAULT 0,
              handoff_count INTEGER NOT NULL DEFAULT 0,
              background_run_id TEXT,
              background_workflow_run_id TEXT,
              background_status TEXT,
              manifest_snapshot_ref TEXT NOT NULL DEFAULT '',
              session_policy_snapshot_ref TEXT NOT NULL DEFAULT '',
              capability_plan_summary_json TEXT NOT NULL DEFAULT '{}',
              provider_state_summary_json TEXT NOT NULL DEFAULT '[]',
              pending_mediation_json TEXT,
              capability_state_ref TEXT,
              last_execution_outcome_json TEXT,
              granted_tool_count INTEGER NOT NULL DEFAULT 0,
              injected_skill_message_count INTEGER NOT NULL DEFAULT 0,
              deferred_capability_count INTEGER NOT NULL DEFAULT 0,
              hidden_capability_count INTEGER NOT NULL DEFAULT 0,
              degraded_provider_count INTEGER NOT NULL DEFAULT 0,
              detail_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pet_presence (
              scope_key TEXT PRIMARY KEY,
              owner_user_id TEXT,
              context_scope TEXT NOT NULL DEFAULT 'home',
              project_id TEXT,
              pet_id TEXT NOT NULL,
              is_visible INTEGER NOT NULL,
              chat_open INTEGER NOT NULL,
              motion_state TEXT NOT NULL,
              unread_count INTEGER NOT NULL,
              last_interaction_at INTEGER NOT NULL,
              position_x INTEGER NOT NULL,
              position_y INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pet_conversation_bindings (
              scope_key TEXT PRIMARY KEY,
              owner_user_id TEXT,
              context_scope TEXT NOT NULL DEFAULT 'home',
              project_id TEXT,
              pet_id TEXT NOT NULL,
              workspace_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              session_id TEXT,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pet_agent_extensions (
              pet_id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              owner_user_id TEXT NOT NULL,
              species TEXT NOT NULL,
              display_name TEXT NOT NULL,
              avatar_label TEXT NOT NULL,
              summary TEXT NOT NULL,
              greeting TEXT NOT NULL,
              mood TEXT NOT NULL,
              favorite_snack TEXT NOT NULL,
              prompt_hints_json TEXT NOT NULL DEFAULT '[]',
              fallback_asset TEXT NOT NULL,
              rive_asset TEXT,
              state_machine TEXT,
              updated_at INTEGER NOT NULL,
              UNIQUE(workspace_id, owner_user_id)
            );
            CREATE TABLE IF NOT EXISTS runtime_run_projections (
              id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              status TEXT NOT NULL,
              current_step TEXT NOT NULL,
              started_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              model_id TEXT,
              next_action TEXT,
              config_snapshot_id TEXT NOT NULL,
              effective_config_hash TEXT NOT NULL,
              started_from_scope_set TEXT NOT NULL,
              run_kind TEXT NOT NULL DEFAULT 'primary',
              parent_run_id TEXT,
              actor_ref TEXT NOT NULL DEFAULT '',
              delegated_by_tool_call_id TEXT,
              workflow_run_id TEXT,
              workflow_step_id TEXT,
              workflow_status TEXT,
              mailbox_ref TEXT,
              handoff_ref TEXT,
              background_state TEXT,
              worker_total_subruns INTEGER NOT NULL DEFAULT 0,
              worker_active_subruns INTEGER NOT NULL DEFAULT 0,
              worker_completed_subruns INTEGER NOT NULL DEFAULT 0,
              worker_failed_subruns INTEGER NOT NULL DEFAULT 0,
              worker_dispatch_json TEXT,
              workflow_run_detail_json TEXT,
              approval_state TEXT NOT NULL DEFAULT 'not-required',
              trace_id TEXT NOT NULL DEFAULT '',
              turn_id TEXT NOT NULL DEFAULT '',
              capability_plan_summary_json TEXT NOT NULL DEFAULT '{}',
              provider_state_summary_json TEXT NOT NULL DEFAULT '[]',
              pending_mediation_json TEXT,
              capability_state_ref TEXT,
              last_execution_outcome_json TEXT,
              granted_tool_count INTEGER NOT NULL DEFAULT 0,
              injected_skill_message_count INTEGER NOT NULL DEFAULT 0,
              deferred_capability_count INTEGER NOT NULL DEFAULT 0,
              hidden_capability_count INTEGER NOT NULL DEFAULT 0,
              degraded_provider_count INTEGER NOT NULL DEFAULT 0,
              run_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_subrun_projections (
              run_id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              parent_run_id TEXT,
              actor_ref TEXT NOT NULL DEFAULT '',
              label TEXT NOT NULL DEFAULT '',
              status TEXT NOT NULL,
              run_kind TEXT NOT NULL DEFAULT 'subrun',
              delegated_by_tool_call_id TEXT,
              workflow_run_id TEXT,
              mailbox_ref TEXT,
              handoff_ref TEXT,
              started_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              summary_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_mailbox_projections (
              mailbox_ref TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              run_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              channel TEXT NOT NULL DEFAULT '',
              status TEXT NOT NULL,
              pending_count INTEGER NOT NULL DEFAULT 0,
              total_messages INTEGER NOT NULL DEFAULT 0,
              latest_handoff_ref TEXT,
              body_storage_path TEXT,
              body_content_hash TEXT,
              updated_at INTEGER NOT NULL,
              summary_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_handoff_projections (
              handoff_ref TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              run_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              parent_run_id TEXT,
              delegated_by_tool_call_id TEXT,
              sender_actor_ref TEXT NOT NULL DEFAULT '',
              receiver_actor_ref TEXT NOT NULL DEFAULT '',
              mailbox_ref TEXT NOT NULL DEFAULT '',
              state TEXT NOT NULL,
              artifact_refs_json TEXT NOT NULL DEFAULT '[]',
              envelope_storage_path TEXT,
              envelope_content_hash TEXT,
              updated_at INTEGER NOT NULL,
              summary_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_workflow_projections (
              workflow_run_id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              run_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              label TEXT NOT NULL DEFAULT '',
              status TEXT NOT NULL,
              total_steps INTEGER NOT NULL DEFAULT 0,
              completed_steps INTEGER NOT NULL DEFAULT 0,
              current_step_id TEXT,
              current_step_label TEXT,
              background_capable INTEGER NOT NULL DEFAULT 0,
              detail_storage_path TEXT,
              detail_content_hash TEXT,
              updated_at INTEGER NOT NULL,
              summary_json TEXT NOT NULL,
              detail_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_background_projections (
              run_id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              workflow_run_id TEXT,
              status TEXT NOT NULL,
              background_capable INTEGER NOT NULL DEFAULT 0,
              state_storage_path TEXT,
              state_content_hash TEXT,
              updated_at INTEGER NOT NULL,
              summary_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_approval_projections (
              id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              run_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              tool_name TEXT NOT NULL,
              summary TEXT NOT NULL,
              detail TEXT NOT NULL,
              risk_level TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              status TEXT NOT NULL,
              approval_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_memory_records (
              memory_id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              owner_ref TEXT,
              source_run_id TEXT,
              kind TEXT NOT NULL,
              scope TEXT NOT NULL,
              title TEXT NOT NULL,
              summary TEXT NOT NULL,
              freshness_state TEXT NOT NULL,
              last_validated_at INTEGER,
              proposal_state TEXT NOT NULL,
              storage_path TEXT,
              content_hash TEXT,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_memory_proposals (
              proposal_id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              run_id TEXT NOT NULL,
              memory_id TEXT NOT NULL,
              kind TEXT NOT NULL,
              scope TEXT NOT NULL,
              title TEXT NOT NULL,
              summary TEXT NOT NULL,
              proposal_state TEXT NOT NULL,
              proposal_reason TEXT NOT NULL,
              review_json TEXT,
              artifact_storage_path TEXT,
              artifact_content_hash TEXT,
              updated_at INTEGER NOT NULL,
              proposal_json TEXT NOT NULL
            );
            ",
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    ensure_user_avatar_columns(&connection)?;
    ensure_agent_record_columns(&connection)?;
    ensure_pet_agent_extension_columns(&connection)?;
    ensure_pet_projection_columns(&connection)?;
    ensure_team_record_columns(&connection)?;
    ensure_bundle_asset_descriptor_columns(&connection)?;
    ensure_project_assignment_columns(&connection)?;
    ensure_project_promotion_request_table(&connection)?;
    ensure_project_agent_link_table(&connection)?;
    ensure_project_team_link_table(&connection)?;
    ensure_runtime_config_snapshot_columns(&connection)?;
    ensure_runtime_session_projection_columns(&connection)?;
    ensure_runtime_run_projection_columns(&connection)?;
    ensure_runtime_phase_four_projection_tables(&connection)?;
    ensure_runtime_memory_projection_tables(&connection)?;
    ensure_cost_entry_columns(&connection)?;
    ensure_resource_columns(&connection)?;
    ensure_knowledge_columns(&connection)?;
    agent_seed::ensure_import_source_tables(&connection)?;

    Ok(())
}

pub(super) fn seed_defaults(paths: &WorkspacePaths) -> Result<(), AppError> {
    let connection =
        Connection::open(&paths.db_path).map_err(|error| AppError::database(error.to_string()))?;

    let project_exists: Option<String> = connection
        .query_row(
            "SELECT id FROM projects WHERE id = ?1",
            params![DEFAULT_PROJECT_ID],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if project_exists.is_none() {
        let default_project_resource_directory =
            paths.default_project_resource_directory(DEFAULT_PROJECT_ID);
        let default_project_assignments = serde_json::to_string(&default_project_assignments())?;
        let default_permission_overrides = serde_json::to_string(&ProjectPermissionOverrides {
            agents: "inherit".into(),
            resources: "inherit".into(),
            tools: "inherit".into(),
            knowledge: "inherit".into(),
        })?;
        let default_linked_assets = serde_json::to_string(&ProjectLinkedWorkspaceAssets {
            agent_ids: Vec::new(),
            resource_ids: Vec::new(),
            tool_source_keys: Vec::new(),
            knowledge_ids: Vec::new(),
        })?;
        let default_member_user_ids = serde_json::to_string(&vec!["user-owner".to_string()])?;
        connection
            .execute(
                "INSERT INTO projects
                 (id, workspace_id, name, status, description, resource_directory, assignments_json, owner_user_id, member_user_ids_json, permission_overrides_json, linked_workspace_assets_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    DEFAULT_PROJECT_ID,
                    DEFAULT_WORKSPACE_ID,
                    "Default Project",
                    "active",
                    "Bootstrap project for the local workspace.",
                    default_project_resource_directory,
                    Some(default_project_assignments),
                    "user-owner",
                    default_member_user_ids,
                    default_permission_overrides,
                    default_linked_assets,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    let resources_exist: Option<String> = connection
        .query_row("SELECT id FROM resources LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if resources_exist.is_none() {
        for record in default_workspace_resources() {
            connection
                .execute(
                    "INSERT INTO resources (id, workspace_id, project_id, kind, name, location, origin, scope, visibility, owner_user_id, storage_path, content_type, byte_size, preview_kind, status, updated_at, tags, source_artifact_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.kind,
                        record.name,
                        record.location,
                        record.origin,
                        record.scope,
                        record.visibility,
                        record.owner_user_id,
                        record.storage_path,
                        record.content_type,
                        record.byte_size.map(|value| value as i64),
                        record.preview_kind,
                        record.status,
                        record.updated_at as i64,
                        serde_json::to_string(&record.tags)?,
                        record.source_artifact_id,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }

        fs::create_dir_all(&paths.workspace_resources_dir)?;
        fs::write(
            paths.workspace_resources_dir.join("workspace-handbook.md"),
            "# Workspace Handbook\n\nShared operating rules for this workspace.\n",
        )?;
        fs::create_dir_all(
            paths
                .project_resources_dir(DEFAULT_PROJECT_ID)
                .join("delivery-board"),
        )?;
    }

    let knowledge_exists: Option<String> = connection
        .query_row("SELECT id FROM knowledge_records LIMIT 1", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if knowledge_exists.is_none() {
        for record in default_knowledge_records() {
            connection
                .execute(
                    "INSERT INTO knowledge_records (id, workspace_id, project_id, title, summary, kind, scope, status, visibility, owner_user_id, source_type, source_ref, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.title,
                        record.summary,
                        record.kind,
                        record.scope,
                        record.status,
                        record.visibility,
                        record.owner_user_id,
                        record.source_type,
                        record.source_ref,
                        record.updated_at as i64,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    // Builtin agent/team assets are now exposed as readonly templates and are no longer
    // materialized into editable workspace records during bootstrap.

    let models_exist: Option<String> = connection
        .query_row("SELECT id FROM model_catalog LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if models_exist.is_none() {
        for record in default_model_catalog() {
            connection
                .execute(
                    "INSERT INTO model_catalog (id, workspace_id, label, provider, description, recommended_for, availability, default_permission)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.label,
                        record.provider,
                        record.description,
                        record.recommended_for,
                        record.availability,
                        record.default_permission,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    let provider_credentials_exist: Option<String> = connection
        .query_row("SELECT id FROM provider_credentials LIMIT 1", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if provider_credentials_exist.is_none() {
        for record in default_provider_credentials() {
            connection
                .execute(
                    "INSERT INTO provider_credentials (id, workspace_id, provider, name, base_url, status)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.provider,
                        record.name,
                        record.base_url,
                        record.status,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    let tools_exist: Option<String> = connection
        .query_row("SELECT id FROM tools LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if tools_exist.is_none() {
        for record in default_tool_records() {
            connection
                .execute(
                    "INSERT INTO tools (id, workspace_id, kind, name, description, status, permission_mode, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.kind,
                        record.name,
                        record.description,
                        record.status,
                        record.permission_mode,
                        record.updated_at as i64,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    // Default automations are not seeded because builtin actors are no longer written into
    // workspace/project tables at initialization time.
    connection
        .execute(
            "INSERT OR IGNORE INTO org_units (id, parent_id, code, name, status)
             VALUES ('org-root', NULL, 'root', 'Root Organization', 'active')",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    for app in default_client_apps() {
        connection
            .execute(
                "INSERT OR REPLACE INTO client_apps
                 (id, name, platform, status, first_party, allowed_origins, allowed_hosts, session_policy, default_scopes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    app.id,
                    app.name,
                    app.platform,
                    app.status,
                    if app.first_party { 1 } else { 0 },
                    serde_json::to_string(&app.allowed_origins)?,
                    serde_json::to_string(&app.allowed_hosts)?,
                    app.session_policy,
                    serde_json::to_string(&app.default_scopes)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}

pub(super) fn table_columns(
    connection: &Connection,
    table_name: &str,
) -> Result<Vec<String>, AppError> {
    let mut stmt = connection
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .map_err(|error| AppError::database(error.to_string()))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(columns)
}

pub(super) fn ensure_columns(
    connection: &Connection,
    table_name: &str,
    definitions: &[(&str, &str)],
) -> Result<(), AppError> {
    let columns = table_columns(connection, table_name)?;

    for (name, definition) in definitions {
        if columns.iter().any(|column| column == name) {
            continue;
        }

        connection
            .execute(
                &format!("ALTER TABLE {table_name} ADD COLUMN {name} {definition}"),
                [],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}

pub(super) fn ensure_user_avatar_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "users",
        &[
            ("avatar_path", "TEXT"),
            ("avatar_content_type", "TEXT"),
            ("avatar_byte_size", "INTEGER"),
            ("avatar_content_hash", "TEXT"),
        ],
    )
}

fn json_string<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(AppError::from)
}

fn merge_json_with_defaults(
    base: serde_json::Value,
    patch: serde_json::Value,
) -> serde_json::Value {
    match (base, patch) {
        (serde_json::Value::Object(mut base_map), serde_json::Value::Object(patch_map)) => {
            for (key, patch_value) in patch_map {
                let merged = merge_json_with_defaults(
                    base_map.remove(&key).unwrap_or(serde_json::Value::Null),
                    patch_value,
                );
                base_map.insert(key, merged);
            }
            serde_json::Value::Object(base_map)
        }
        (base, serde_json::Value::Null) => base,
        (_, patch) => patch,
    }
}

fn parse_json_or_default<T, F>(raw: &str, default: F) -> T
where
    T: serde::de::DeserializeOwned + Serialize,
    F: FnOnce() -> T,
{
    let default_value = default();
    let merged = serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|patch| {
            serde_json::to_value(&default_value)
                .ok()
                .map(|base| merge_json_with_defaults(base, patch))
        })
        .unwrap_or(serde_json::Value::Null);
    serde_json::from_value(merged).unwrap_or(default_value)
}

pub(super) fn ensure_agent_record_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "agents",
        &[
            ("owner_user_id", "TEXT"),
            ("asset_role", "TEXT NOT NULL DEFAULT 'default'"),
            ("avatar_path", "TEXT"),
            ("personality", "TEXT NOT NULL DEFAULT ''"),
            ("tags", "TEXT NOT NULL DEFAULT '[]'"),
            ("prompt", "TEXT NOT NULL DEFAULT ''"),
            ("builtin_tool_keys", "TEXT NOT NULL DEFAULT '[]'"),
            ("skill_ids", "TEXT NOT NULL DEFAULT '[]'"),
            ("mcp_server_names", "TEXT NOT NULL DEFAULT '[]'"),
            ("task_domains", "TEXT NOT NULL DEFAULT '[]'"),
            (
                "manifest_revision",
                "TEXT NOT NULL DEFAULT 'asset-manifest/v2'",
            ),
            ("default_model_strategy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("capability_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("permission_envelope_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("memory_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("delegation_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("approval_preference_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("output_contract_json", "TEXT NOT NULL DEFAULT '{}'"),
            (
                "shared_capability_policy_json",
                "TEXT NOT NULL DEFAULT '{}'",
            ),
            ("integration_source_json", "TEXT"),
            ("trust_metadata_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("dependency_resolution_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("import_metadata_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )
}

pub(super) fn ensure_pet_agent_extension_columns(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS pet_agent_extensions (
                pet_id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                owner_user_id TEXT NOT NULL,
                species TEXT NOT NULL,
                display_name TEXT NOT NULL,
                avatar_label TEXT NOT NULL,
                summary TEXT NOT NULL,
                greeting TEXT NOT NULL,
                mood TEXT NOT NULL,
                favorite_snack TEXT NOT NULL,
                prompt_hints_json TEXT NOT NULL DEFAULT '[]',
                fallback_asset TEXT NOT NULL,
                rive_asset TEXT,
                state_machine TEXT,
                updated_at INTEGER NOT NULL,
                UNIQUE(workspace_id, owner_user_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "pet_agent_extensions",
        &[
            ("workspace_id", "TEXT NOT NULL DEFAULT ''"),
            ("owner_user_id", "TEXT NOT NULL DEFAULT 'user-owner'"),
            ("species", "TEXT NOT NULL DEFAULT 'octopus'"),
            ("display_name", "TEXT NOT NULL DEFAULT '小章'"),
            ("avatar_label", "TEXT NOT NULL DEFAULT 'Octopus mascot'"),
            (
                "summary",
                "TEXT NOT NULL DEFAULT 'Octopus 首席吉祥物，负责卖萌和加油。'",
            ),
            (
                "greeting",
                "TEXT NOT NULL DEFAULT '嗨！我是小章，今天也要加油哦！'",
            ),
            ("mood", "TEXT NOT NULL DEFAULT 'happy'"),
            ("favorite_snack", "TEXT NOT NULL DEFAULT '新鲜小虾'"),
            ("prompt_hints_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("fallback_asset", "TEXT NOT NULL DEFAULT 'octopus'"),
            ("rive_asset", "TEXT"),
            ("state_machine", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
        ],
    )?;
    connection
        .execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_pet_agent_extensions_workspace_owner
             ON pet_agent_extensions (workspace_id, owner_user_id)",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

fn ensure_pet_projection_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "pet_presence",
        &[
            ("owner_user_id", "TEXT"),
            ("context_scope", "TEXT NOT NULL DEFAULT 'home'"),
            ("project_id", "TEXT"),
        ],
    )?;
    ensure_columns(
        connection,
        "pet_conversation_bindings",
        &[
            ("owner_user_id", "TEXT"),
            ("context_scope", "TEXT NOT NULL DEFAULT 'home'"),
            ("project_id", "TEXT"),
        ],
    )
}

pub(super) fn ensure_team_record_columns(connection: &Connection) -> Result<(), AppError> {
    let columns = table_columns(connection, "teams")?;

    ensure_columns(
        connection,
        "teams",
        &[
            ("avatar_path", "TEXT"),
            ("personality", "TEXT NOT NULL DEFAULT ''"),
            ("tags", "TEXT NOT NULL DEFAULT '[]'"),
            ("prompt", "TEXT NOT NULL DEFAULT ''"),
            ("builtin_tool_keys", "TEXT NOT NULL DEFAULT '[]'"),
            ("skill_ids", "TEXT NOT NULL DEFAULT '[]'"),
            ("mcp_server_names", "TEXT NOT NULL DEFAULT '[]'"),
            ("task_domains", "TEXT NOT NULL DEFAULT '[]'"),
            (
                "manifest_revision",
                "TEXT NOT NULL DEFAULT 'asset-manifest/v2'",
            ),
            ("default_model_strategy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("capability_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("permission_envelope_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("memory_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("delegation_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("approval_preference_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("output_contract_json", "TEXT NOT NULL DEFAULT '{}'"),
            (
                "shared_capability_policy_json",
                "TEXT NOT NULL DEFAULT '{}'",
            ),
            ("leader_agent_id", "TEXT"),
            ("member_agent_ids", "TEXT NOT NULL DEFAULT '[]'"),
            ("leader_ref", "TEXT NOT NULL DEFAULT ''"),
            ("member_refs", "TEXT NOT NULL DEFAULT '[]'"),
            ("team_topology_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("shared_memory_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("mailbox_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("artifact_handoff_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("workflow_affordance_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("worker_concurrency_limit", "INTEGER NOT NULL DEFAULT 1"),
            ("integration_source_json", "TEXT"),
            ("trust_metadata_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("dependency_resolution_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("import_metadata_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    if columns.iter().any(|column| column == "member_ids") {
        connection
            .execute(
                "UPDATE teams SET member_agent_ids = member_ids WHERE member_agent_ids = '[]' AND member_ids IS NOT NULL",
                [],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}

pub(super) fn ensure_bundle_asset_descriptor_columns(
    connection: &Connection,
) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS bundle_asset_descriptors (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                project_id TEXT,
                scope TEXT NOT NULL,
                asset_kind TEXT NOT NULL,
                source_id TEXT NOT NULL,
                display_name TEXT NOT NULL,
                source_path TEXT NOT NULL,
                storage_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                byte_size INTEGER NOT NULL,
                manifest_revision TEXT NOT NULL DEFAULT 'asset-manifest/v2',
                task_domains_json TEXT NOT NULL DEFAULT '[]',
                translation_mode TEXT NOT NULL DEFAULT 'native',
                trust_metadata_json TEXT NOT NULL DEFAULT '{}',
                dependency_resolution_json TEXT NOT NULL DEFAULT '[]',
                import_metadata_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "bundle_asset_descriptors",
        &[
            ("project_id", "TEXT"),
            ("scope", "TEXT NOT NULL DEFAULT 'workspace'"),
            ("asset_kind", "TEXT NOT NULL DEFAULT 'plugin'"),
            ("source_id", "TEXT NOT NULL DEFAULT ''"),
            ("display_name", "TEXT NOT NULL DEFAULT ''"),
            ("source_path", "TEXT NOT NULL DEFAULT ''"),
            ("storage_path", "TEXT NOT NULL DEFAULT ''"),
            ("content_hash", "TEXT NOT NULL DEFAULT ''"),
            ("byte_size", "INTEGER NOT NULL DEFAULT 0"),
            (
                "manifest_revision",
                "TEXT NOT NULL DEFAULT 'asset-manifest/v2'",
            ),
            ("task_domains_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("translation_mode", "TEXT NOT NULL DEFAULT 'native'"),
            ("trust_metadata_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("dependency_resolution_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("import_metadata_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
        ],
    )
}

pub(super) fn write_agent_record(
    connection: &Connection,
    record: &AgentRecord,
    replace: bool,
) -> Result<(), AppError> {
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };

    let sql = format!(
        "{verb} INTO agents (
            id, workspace_id, project_id, scope, owner_user_id, asset_role, name, avatar_path, personality, tags, prompt,
            builtin_tool_keys, skill_ids, mcp_server_names, task_domains, manifest_revision,
            default_model_strategy_json, capability_policy_json, permission_envelope_json,
            memory_policy_json, delegation_policy_json, approval_preference_json,
            output_contract_json, shared_capability_policy_json, integration_source_json,
            trust_metadata_json, dependency_resolution_json, import_metadata_json,
            description, status, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
            ?12, ?13, ?14, ?15, ?16,
            ?17, ?18, ?19,
            ?20, ?21, ?22,
            ?23, ?24, ?25,
            ?26, ?27, ?28,
            ?29, ?30, ?31
        )"
    );

    connection
        .execute(
            &sql,
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.owner_user_id,
                record.asset_role,
                record.name,
                record.avatar_path,
                record.personality,
                json_string(&record.tags)?,
                record.prompt,
                json_string(&record.builtin_tool_keys)?,
                json_string(&record.skill_ids)?,
                json_string(&record.mcp_server_names)?,
                json_string(&record.task_domains)?,
                record.manifest_revision,
                json_string(&record.default_model_strategy)?,
                json_string(&record.capability_policy)?,
                json_string(&record.permission_envelope)?,
                json_string(&record.memory_policy)?,
                json_string(&record.delegation_policy)?,
                json_string(&record.approval_preference)?,
                json_string(&record.output_contract)?,
                json_string(&record.shared_capability_policy)?,
                record
                    .integration_source
                    .as_ref()
                    .map(json_string)
                    .transpose()?,
                json_string(&record.trust_metadata)?,
                json_string(&record.dependency_resolution)?,
                json_string(&record.import_metadata)?,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(super) fn write_team_record(
    connection: &Connection,
    record: &TeamRecord,
    replace: bool,
) -> Result<(), AppError> {
    let member_agent_ids_json = serde_json::to_string(&record.member_agent_ids)?;
    let member_refs_json = json_string(&record.member_refs)?;
    let has_legacy_member_ids = table_columns(connection, "teams")?
        .iter()
        .any(|column| column == "member_ids");
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };

    let sql = if has_legacy_member_ids {
        format!(
            "{verb} INTO teams (
                id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt,
                builtin_tool_keys, skill_ids, mcp_server_names, task_domains, manifest_revision,
                default_model_strategy_json, capability_policy_json, permission_envelope_json,
                memory_policy_json, delegation_policy_json, approval_preference_json,
                output_contract_json, shared_capability_policy_json, leader_agent_id, member_ids,
                member_agent_ids, leader_ref, member_refs, team_topology_json,
                shared_memory_policy_json, mailbox_policy_json, artifact_handoff_policy_json,
                workflow_affordance_json, worker_concurrency_limit, integration_source_json,
                trust_metadata_json, dependency_resolution_json, import_metadata_json,
                description, status, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13, ?14,
                ?15, ?16, ?17,
                ?18, ?19, ?20,
                ?21, ?22, ?23, ?24,
                ?25, ?26, ?27, ?28,
                ?29, ?30, ?31, ?32,
                ?33, ?34, ?35,
                ?36, ?37, ?38,
                ?39, ?40, ?41
            )"
        )
    } else {
        format!(
            "{verb} INTO teams (
                id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt,
                builtin_tool_keys, skill_ids, mcp_server_names, task_domains, manifest_revision,
                default_model_strategy_json, capability_policy_json, permission_envelope_json,
                memory_policy_json, delegation_policy_json, approval_preference_json,
                output_contract_json, shared_capability_policy_json, leader_agent_id,
                member_agent_ids, leader_ref, member_refs, team_topology_json,
                shared_memory_policy_json, mailbox_policy_json, artifact_handoff_policy_json,
                workflow_affordance_json, worker_concurrency_limit, integration_source_json,
                trust_metadata_json, dependency_resolution_json, import_metadata_json,
                description, status, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13, ?14,
                ?15, ?16, ?17,
                ?18, ?19, ?20,
                ?21, ?22, ?23,
                ?24, ?25, ?26, ?27,
                ?28, ?29, ?30, ?31,
                ?32, ?33, ?34,
                ?35, ?36, ?37,
                ?38, ?39
            )"
        )
    };

    if has_legacy_member_ids {
        connection.execute(
            &sql,
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.name,
                record.avatar_path,
                record.personality,
                serde_json::to_string(&record.tags)?,
                record.prompt,
                serde_json::to_string(&record.builtin_tool_keys)?,
                serde_json::to_string(&record.skill_ids)?,
                serde_json::to_string(&record.mcp_server_names)?,
                json_string(&record.task_domains)?,
                record.manifest_revision,
                json_string(&record.default_model_strategy)?,
                json_string(&record.capability_policy)?,
                json_string(&record.permission_envelope)?,
                json_string(&record.memory_policy)?,
                json_string(&record.delegation_policy)?,
                json_string(&record.approval_preference)?,
                json_string(&record.output_contract)?,
                json_string(&record.shared_capability_policy)?,
                record.leader_agent_id,
                member_agent_ids_json,
                member_agent_ids_json,
                record.leader_ref,
                member_refs_json,
                json_string(&record.team_topology)?,
                json_string(&record.shared_memory_policy)?,
                json_string(&record.mailbox_policy)?,
                json_string(&record.artifact_handoff_policy)?,
                json_string(&record.workflow_affordance)?,
                record.worker_concurrency_limit as i64,
                record
                    .integration_source
                    .as_ref()
                    .map(json_string)
                    .transpose()?,
                json_string(&record.trust_metadata)?,
                json_string(&record.dependency_resolution)?,
                json_string(&record.import_metadata)?,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        )
    } else {
        connection.execute(
            &sql,
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.name,
                record.avatar_path,
                record.personality,
                serde_json::to_string(&record.tags)?,
                record.prompt,
                serde_json::to_string(&record.builtin_tool_keys)?,
                serde_json::to_string(&record.skill_ids)?,
                serde_json::to_string(&record.mcp_server_names)?,
                json_string(&record.task_domains)?,
                record.manifest_revision,
                json_string(&record.default_model_strategy)?,
                json_string(&record.capability_policy)?,
                json_string(&record.permission_envelope)?,
                json_string(&record.memory_policy)?,
                json_string(&record.delegation_policy)?,
                json_string(&record.approval_preference)?,
                json_string(&record.output_contract)?,
                json_string(&record.shared_capability_policy)?,
                record.leader_agent_id,
                member_agent_ids_json,
                record.leader_ref,
                member_refs_json,
                json_string(&record.team_topology)?,
                json_string(&record.shared_memory_policy)?,
                json_string(&record.mailbox_policy)?,
                json_string(&record.artifact_handoff_policy)?,
                json_string(&record.workflow_affordance)?,
                record.worker_concurrency_limit as i64,
                record
                    .integration_source
                    .as_ref()
                    .map(json_string)
                    .transpose()?,
                json_string(&record.trust_metadata)?,
                json_string(&record.dependency_resolution)?,
                json_string(&record.import_metadata)?,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        )
    }
    .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(super) fn write_bundle_asset_descriptor_record(
    connection: &Connection,
    record: &BundleAssetDescriptorRecord,
    replace: bool,
) -> Result<(), AppError> {
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };
    let sql = format!(
        "{verb} INTO bundle_asset_descriptors (
            id, workspace_id, project_id, scope, asset_kind, source_id, display_name, source_path,
            storage_path, content_hash, byte_size, manifest_revision, task_domains_json,
            translation_mode, trust_metadata_json, dependency_resolution_json,
            import_metadata_json, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
            ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16,
            ?17, ?18
        )"
    );

    connection
        .execute(
            &sql,
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.asset_kind,
                record.source_id,
                record.display_name,
                record.source_path,
                record.storage_path,
                record.content_hash,
                record.byte_size as i64,
                record.manifest_revision,
                json_string(&record.task_domains)?,
                record.translation_mode,
                json_string(&record.trust_metadata)?,
                json_string(&record.dependency_resolution)?,
                json_string(&record.import_metadata)?,
                record.updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(super) fn ensure_project_assignment_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "projects",
        &[
            ("assignments_json", "TEXT"),
            ("resource_directory", "TEXT"),
            ("owner_user_id", "TEXT"),
            ("member_user_ids_json", "TEXT"),
            ("permission_overrides_json", "TEXT"),
            ("linked_workspace_assets_json", "TEXT"),
        ],
    )
}

pub(super) fn ensure_project_promotion_request_table(
    connection: &Connection,
) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_promotion_requests (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              asset_type TEXT NOT NULL,
              asset_id TEXT NOT NULL,
              requested_by_user_id TEXT NOT NULL,
              submitted_by_owner_user_id TEXT NOT NULL,
              required_workspace_capability TEXT NOT NULL,
              status TEXT NOT NULL,
              reviewed_by_user_id TEXT,
              review_comment TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              reviewed_at INTEGER
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(super) fn ensure_project_agent_link_table(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_agent_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              agent_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, agent_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(super) fn ensure_project_team_link_table(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_team_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              team_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, team_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(super) fn ensure_runtime_config_snapshot_columns(
    connection: &Connection,
) -> Result<(), AppError> {
    let mut stmt = connection
        .prepare("PRAGMA table_info(runtime_config_snapshots)")
        .map_err(|error| AppError::database(error.to_string()))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

    if columns
        .iter()
        .any(|column| column == "effective_config_json")
    {
        return Ok(());
    }

    connection
        .execute(
            "ALTER TABLE runtime_config_snapshots ADD COLUMN effective_config_json TEXT",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(super) fn ensure_runtime_session_projection_columns(
    connection: &Connection,
) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "runtime_session_projections",
        &[
            ("session_kind", "TEXT NOT NULL DEFAULT 'project'"),
            ("last_message_preview", "TEXT"),
            ("config_snapshot_id", "TEXT NOT NULL DEFAULT ''"),
            ("effective_config_hash", "TEXT NOT NULL DEFAULT ''"),
            ("started_from_scope_set", "TEXT NOT NULL DEFAULT '[]'"),
            ("selected_actor_ref", "TEXT NOT NULL DEFAULT ''"),
            ("manifest_revision", "TEXT NOT NULL DEFAULT ''"),
            ("active_run_id", "TEXT NOT NULL DEFAULT ''"),
            ("subrun_count", "INTEGER NOT NULL DEFAULT 0"),
            ("workflow_run_id", "TEXT"),
            ("workflow_status", "TEXT"),
            ("workflow_total_steps", "INTEGER NOT NULL DEFAULT 0"),
            ("workflow_completed_steps", "INTEGER NOT NULL DEFAULT 0"),
            ("workflow_current_step_id", "TEXT"),
            ("workflow_current_step_label", "TEXT"),
            ("workflow_background_capable", "INTEGER NOT NULL DEFAULT 0"),
            ("pending_mailbox_ref", "TEXT"),
            ("pending_mailbox_count", "INTEGER NOT NULL DEFAULT 0"),
            ("handoff_count", "INTEGER NOT NULL DEFAULT 0"),
            ("background_run_id", "TEXT"),
            ("background_workflow_run_id", "TEXT"),
            ("background_status", "TEXT"),
            ("manifest_snapshot_ref", "TEXT NOT NULL DEFAULT ''"),
            ("session_policy_snapshot_ref", "TEXT NOT NULL DEFAULT ''"),
            ("capability_plan_summary_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("provider_state_summary_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("pending_mediation_json", "TEXT"),
            ("pending_mediation_kind", "TEXT"),
            ("pending_target_kind", "TEXT"),
            ("pending_target_ref", "TEXT"),
            ("pending_approval_layer", "TEXT"),
            ("pending_provider_key", "TEXT"),
            ("pending_checkpoint_ref", "TEXT"),
            ("capability_state_ref", "TEXT"),
            ("last_execution_outcome_json", "TEXT"),
            ("last_mediation_outcome_json", "TEXT"),
            ("last_mediation_outcome", "TEXT"),
            ("last_mediation_target_kind", "TEXT"),
            ("last_mediation_target_ref", "TEXT"),
            ("last_mediation_at", "INTEGER"),
            ("auth_challenge_state", "TEXT"),
            ("approval_lineage_json", "TEXT"),
            ("denied_exposure_count", "INTEGER NOT NULL DEFAULT 0"),
            ("granted_tool_count", "INTEGER NOT NULL DEFAULT 0"),
            ("injected_skill_message_count", "INTEGER NOT NULL DEFAULT 0"),
            ("deferred_capability_count", "INTEGER NOT NULL DEFAULT 0"),
            ("hidden_capability_count", "INTEGER NOT NULL DEFAULT 0"),
            ("degraded_provider_count", "INTEGER NOT NULL DEFAULT 0"),
            (
                "detail_json",
                r#"TEXT NOT NULL DEFAULT '{"summary":{"id":"","conversationId":"","projectId":"","title":"","sessionKind":"project","status":"draft","updatedAt":0,"lastMessagePreview":null,"configSnapshotId":"","effectiveConfigHash":"","startedFromScopeSet":[],"selectedActorRef":"","manifestRevision":"","sessionPolicy":{"selectedActorRef":"","selectedConfiguredModelId":"","executionPermissionMode":"","configSnapshotId":"","manifestRevision":"","capabilityPolicy":{},"memoryPolicy":{},"delegationPolicy":{},"approvalPreference":{}},"activeRunId":"","subrunCount":0,"memorySummary":{"summary":"","durableMemoryCount":0,"selectedMemoryIds":[]},"capabilitySummary":{"visibleTools":[],"discoverableSkills":[]}},"selectedActorRef":"","manifestRevision":"","sessionPolicy":{"selectedActorRef":"","selectedConfiguredModelId":"","executionPermissionMode":"","configSnapshotId":"","manifestRevision":"","capabilityPolicy":{},"memoryPolicy":{},"delegationPolicy":{},"approvalPreference":{}},"activeRunId":"","subrunCount":0,"memorySummary":{"summary":"","durableMemoryCount":0,"selectedMemoryIds":[]},"capabilitySummary":{"visibleTools":[],"discoverableSkills":[]},"run":{"id":"","sessionId":"","conversationId":"","status":"draft","currentStep":"ready","startedAt":0,"updatedAt":0,"configuredModelId":null,"configuredModelName":null,"modelId":null,"consumedTokens":null,"nextAction":null,"configSnapshotId":"","effectiveConfigHash":"","startedFromScopeSet":[],"runKind":"primary","parentRunId":null,"actorRef":"","delegatedByToolCallId":null,"approvalState":"not-required","usageSummary":{"inputTokens":0,"outputTokens":0,"totalTokens":0},"artifactRefs":[],"traceContext":{"sessionId":"","traceId":"","turnId":"","parentRunId":null},"checkpoint":{"currentIterationIndex":0,"usageSummary":{"inputTokens":0,"outputTokens":0,"totalTokens":0},"pendingApproval":null},"resolvedTarget":null,"requestedActorKind":null,"requestedActorId":null,"resolvedActorKind":null,"resolvedActorId":null,"resolvedActorLabel":null},"messages":[],"trace":[],"pendingApproval":null}'"#,
            ),
        ],
    )
}

pub(super) fn ensure_runtime_run_projection_columns(
    connection: &Connection,
) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "runtime_run_projections",
        &[
            ("run_kind", "TEXT NOT NULL DEFAULT 'primary'"),
            ("parent_run_id", "TEXT"),
            ("actor_ref", "TEXT NOT NULL DEFAULT ''"),
            ("delegated_by_tool_call_id", "TEXT"),
            ("workflow_run_id", "TEXT"),
            ("workflow_step_id", "TEXT"),
            ("workflow_status", "TEXT"),
            ("mailbox_ref", "TEXT"),
            ("handoff_ref", "TEXT"),
            ("background_state", "TEXT"),
            ("worker_total_subruns", "INTEGER NOT NULL DEFAULT 0"),
            ("worker_active_subruns", "INTEGER NOT NULL DEFAULT 0"),
            ("worker_completed_subruns", "INTEGER NOT NULL DEFAULT 0"),
            ("worker_failed_subruns", "INTEGER NOT NULL DEFAULT 0"),
            ("worker_dispatch_json", "TEXT"),
            ("workflow_run_detail_json", "TEXT"),
            ("approval_state", "TEXT NOT NULL DEFAULT 'not-required'"),
            ("trace_id", "TEXT NOT NULL DEFAULT ''"),
            ("turn_id", "TEXT NOT NULL DEFAULT ''"),
            ("capability_plan_summary_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("provider_state_summary_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("pending_mediation_json", "TEXT"),
            ("pending_mediation_kind", "TEXT"),
            ("pending_target_kind", "TEXT"),
            ("pending_target_ref", "TEXT"),
            ("pending_approval_layer", "TEXT"),
            ("pending_provider_key", "TEXT"),
            ("pending_checkpoint_ref", "TEXT"),
            ("capability_state_ref", "TEXT"),
            ("last_execution_outcome_json", "TEXT"),
            ("last_mediation_outcome_json", "TEXT"),
            ("last_mediation_outcome", "TEXT"),
            ("last_mediation_target_kind", "TEXT"),
            ("last_mediation_target_ref", "TEXT"),
            ("last_mediation_at", "INTEGER"),
            ("auth_challenge_state", "TEXT"),
            ("approval_lineage_json", "TEXT"),
            ("denied_exposure_count", "INTEGER NOT NULL DEFAULT 0"),
            ("granted_tool_count", "INTEGER NOT NULL DEFAULT 0"),
            ("injected_skill_message_count", "INTEGER NOT NULL DEFAULT 0"),
            ("deferred_capability_count", "INTEGER NOT NULL DEFAULT 0"),
            ("hidden_capability_count", "INTEGER NOT NULL DEFAULT 0"),
            ("degraded_provider_count", "INTEGER NOT NULL DEFAULT 0"),
            (
                "run_json",
                r#"TEXT NOT NULL DEFAULT '{"id":"","sessionId":"","conversationId":"","status":"draft","currentStep":"ready","startedAt":0,"updatedAt":0,"configuredModelId":null,"configuredModelName":null,"modelId":null,"consumedTokens":null,"nextAction":null,"configSnapshotId":"","effectiveConfigHash":"","startedFromScopeSet":[],"runKind":"primary","parentRunId":null,"actorRef":"","delegatedByToolCallId":null,"approvalState":"not-required","usageSummary":{"inputTokens":0,"outputTokens":0,"totalTokens":0},"artifactRefs":[],"traceContext":{"sessionId":"","traceId":"","turnId":"","parentRunId":null},"checkpoint":{"currentIterationIndex":0,"usageSummary":{"inputTokens":0,"outputTokens":0,"totalTokens":0},"pendingApproval":null},"resolvedTarget":null,"requestedActorKind":null,"requestedActorId":null,"resolvedActorKind":null,"resolvedActorId":null,"resolvedActorLabel":null}'"#,
            ),
        ],
    )
}

pub(super) fn ensure_runtime_phase_four_projection_tables(
    connection: &Connection,
) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_subrun_projections (
                run_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                parent_run_id TEXT,
                actor_ref TEXT NOT NULL DEFAULT '',
                label TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL,
                run_kind TEXT NOT NULL DEFAULT 'subrun',
                delegated_by_tool_call_id TEXT,
                workflow_run_id TEXT,
                mailbox_ref TEXT,
                handoff_ref TEXT,
                started_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                summary_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_subrun_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("parent_run_id", "TEXT"),
            ("actor_ref", "TEXT NOT NULL DEFAULT ''"),
            ("label", "TEXT NOT NULL DEFAULT ''"),
            ("status", "TEXT NOT NULL DEFAULT 'draft'"),
            ("run_kind", "TEXT NOT NULL DEFAULT 'subrun'"),
            ("delegated_by_tool_call_id", "TEXT"),
            ("workflow_run_id", "TEXT"),
            ("mailbox_ref", "TEXT"),
            ("handoff_ref", "TEXT"),
            ("started_at", "INTEGER NOT NULL DEFAULT 0"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("summary_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_mailbox_projections (
                mailbox_ref TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                channel TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL,
                pending_count INTEGER NOT NULL DEFAULT 0,
                total_messages INTEGER NOT NULL DEFAULT 0,
                latest_handoff_ref TEXT,
                body_storage_path TEXT,
                body_content_hash TEXT,
                updated_at INTEGER NOT NULL,
                summary_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_mailbox_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("channel", "TEXT NOT NULL DEFAULT ''"),
            ("status", "TEXT NOT NULL DEFAULT 'pending'"),
            ("pending_count", "INTEGER NOT NULL DEFAULT 0"),
            ("total_messages", "INTEGER NOT NULL DEFAULT 0"),
            ("latest_handoff_ref", "TEXT"),
            ("body_storage_path", "TEXT"),
            ("body_content_hash", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("summary_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_handoff_projections (
                handoff_ref TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                parent_run_id TEXT,
                delegated_by_tool_call_id TEXT,
                sender_actor_ref TEXT NOT NULL DEFAULT '',
                receiver_actor_ref TEXT NOT NULL DEFAULT '',
                mailbox_ref TEXT NOT NULL DEFAULT '',
                state TEXT NOT NULL,
                artifact_refs_json TEXT NOT NULL DEFAULT '[]',
                envelope_storage_path TEXT,
                envelope_content_hash TEXT,
                updated_at INTEGER NOT NULL,
                summary_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_handoff_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("parent_run_id", "TEXT"),
            ("delegated_by_tool_call_id", "TEXT"),
            ("sender_actor_ref", "TEXT NOT NULL DEFAULT ''"),
            ("receiver_actor_ref", "TEXT NOT NULL DEFAULT ''"),
            ("mailbox_ref", "TEXT NOT NULL DEFAULT ''"),
            ("state", "TEXT NOT NULL DEFAULT 'pending'"),
            ("artifact_refs_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("envelope_storage_path", "TEXT"),
            ("envelope_content_hash", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("summary_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_artifact_projections (
                artifact_ref TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                parent_run_id TEXT,
                delegated_by_tool_call_id TEXT,
                actor_ref TEXT NOT NULL DEFAULT '',
                workflow_run_id TEXT,
                storage_path TEXT NOT NULL DEFAULT '',
                content_hash TEXT NOT NULL DEFAULT '',
                byte_size INTEGER NOT NULL DEFAULT 0,
                content_type TEXT NOT NULL DEFAULT 'application/json',
                updated_at INTEGER NOT NULL DEFAULT 0,
                summary_json TEXT NOT NULL DEFAULT '{}'
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_artifact_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("parent_run_id", "TEXT"),
            ("delegated_by_tool_call_id", "TEXT"),
            ("actor_ref", "TEXT NOT NULL DEFAULT ''"),
            ("workflow_run_id", "TEXT"),
            ("storage_path", "TEXT NOT NULL DEFAULT ''"),
            ("content_hash", "TEXT NOT NULL DEFAULT ''"),
            ("byte_size", "INTEGER NOT NULL DEFAULT 0"),
            ("content_type", "TEXT NOT NULL DEFAULT 'application/json'"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("summary_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_workflow_projections (
                workflow_run_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                label TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL,
                total_steps INTEGER NOT NULL DEFAULT 0,
                completed_steps INTEGER NOT NULL DEFAULT 0,
                current_step_id TEXT,
                current_step_label TEXT,
                background_capable INTEGER NOT NULL DEFAULT 0,
                detail_storage_path TEXT,
                detail_content_hash TEXT,
                updated_at INTEGER NOT NULL,
                summary_json TEXT NOT NULL,
                detail_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_workflow_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("label", "TEXT NOT NULL DEFAULT ''"),
            ("status", "TEXT NOT NULL DEFAULT 'draft'"),
            ("total_steps", "INTEGER NOT NULL DEFAULT 0"),
            ("completed_steps", "INTEGER NOT NULL DEFAULT 0"),
            ("current_step_id", "TEXT"),
            ("current_step_label", "TEXT"),
            ("background_capable", "INTEGER NOT NULL DEFAULT 0"),
            ("detail_storage_path", "TEXT"),
            ("detail_content_hash", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("summary_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("detail_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_background_projections (
                run_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                workflow_run_id TEXT,
                status TEXT NOT NULL,
                background_capable INTEGER NOT NULL DEFAULT 0,
                state_storage_path TEXT,
                state_content_hash TEXT,
                updated_at INTEGER NOT NULL,
                summary_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_background_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("workflow_run_id", "TEXT"),
            ("status", "TEXT NOT NULL DEFAULT 'draft'"),
            ("background_capable", "INTEGER NOT NULL DEFAULT 0"),
            ("state_storage_path", "TEXT"),
            ("state_content_hash", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("summary_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_approval_projections (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                summary TEXT NOT NULL,
                detail TEXT NOT NULL,
                risk_level TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL,
                approval_layer TEXT,
                capability_id TEXT,
                checkpoint_ref TEXT,
                provider_key TEXT,
                required_permission TEXT,
                requires_approval INTEGER NOT NULL DEFAULT 0,
                requires_auth INTEGER NOT NULL DEFAULT 0,
                target_kind TEXT,
                target_ref TEXT,
                escalation_reason TEXT,
                approval_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_approval_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("tool_name", "TEXT NOT NULL DEFAULT ''"),
            ("summary", "TEXT NOT NULL DEFAULT ''"),
            ("detail", "TEXT NOT NULL DEFAULT ''"),
            ("risk_level", "TEXT NOT NULL DEFAULT 'medium'"),
            ("created_at", "INTEGER NOT NULL DEFAULT 0"),
            ("status", "TEXT NOT NULL DEFAULT 'pending'"),
            ("approval_layer", "TEXT"),
            ("capability_id", "TEXT"),
            ("checkpoint_ref", "TEXT"),
            ("provider_key", "TEXT"),
            ("required_permission", "TEXT"),
            ("requires_approval", "INTEGER NOT NULL DEFAULT 0"),
            ("requires_auth", "INTEGER NOT NULL DEFAULT 0"),
            ("target_kind", "TEXT"),
            ("target_ref", "TEXT"),
            ("escalation_reason", "TEXT"),
            ("approval_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_auth_challenge_projections (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                summary TEXT NOT NULL,
                detail TEXT NOT NULL,
                status TEXT NOT NULL,
                resolution TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                approval_layer TEXT,
                capability_id TEXT,
                checkpoint_ref TEXT,
                provider_key TEXT,
                required_permission TEXT,
                requires_approval INTEGER NOT NULL DEFAULT 0,
                requires_auth INTEGER NOT NULL DEFAULT 0,
                target_kind TEXT,
                target_ref TEXT,
                escalation_reason TEXT,
                challenge_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_auth_challenge_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("summary", "TEXT NOT NULL DEFAULT ''"),
            ("detail", "TEXT NOT NULL DEFAULT ''"),
            ("status", "TEXT NOT NULL DEFAULT 'pending'"),
            ("resolution", "TEXT"),
            ("created_at", "INTEGER NOT NULL DEFAULT 0"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("approval_layer", "TEXT"),
            ("capability_id", "TEXT"),
            ("checkpoint_ref", "TEXT"),
            ("provider_key", "TEXT"),
            ("required_permission", "TEXT"),
            ("requires_approval", "INTEGER NOT NULL DEFAULT 0"),
            ("requires_auth", "INTEGER NOT NULL DEFAULT 0"),
            ("target_kind", "TEXT"),
            ("target_ref", "TEXT"),
            ("escalation_reason", "TEXT"),
            ("challenge_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    Ok(())
}

pub(super) fn ensure_runtime_memory_projection_tables(
    connection: &Connection,
) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_memory_records (
                memory_id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                project_id TEXT,
                owner_ref TEXT,
                source_run_id TEXT,
                kind TEXT NOT NULL,
                scope TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                freshness_state TEXT NOT NULL,
                last_validated_at INTEGER,
                proposal_state TEXT NOT NULL,
                storage_path TEXT,
                content_hash TEXT,
                updated_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_memory_records",
        &[
            ("workspace_id", "TEXT NOT NULL DEFAULT ''"),
            ("project_id", "TEXT"),
            ("owner_ref", "TEXT"),
            ("source_run_id", "TEXT"),
            ("kind", "TEXT NOT NULL DEFAULT 'reference'"),
            ("scope", "TEXT NOT NULL DEFAULT 'user'"),
            ("title", "TEXT NOT NULL DEFAULT ''"),
            ("summary", "TEXT NOT NULL DEFAULT ''"),
            ("freshness_state", "TEXT NOT NULL DEFAULT 'fresh'"),
            ("last_validated_at", "INTEGER"),
            ("proposal_state", "TEXT NOT NULL DEFAULT 'pending'"),
            ("storage_path", "TEXT"),
            ("content_hash", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_memory_proposals (
                proposal_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                memory_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                scope TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                proposal_state TEXT NOT NULL,
                proposal_reason TEXT NOT NULL,
                review_json TEXT,
                artifact_storage_path TEXT,
                artifact_content_hash TEXT,
                updated_at INTEGER NOT NULL,
                proposal_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_memory_proposals",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("memory_id", "TEXT NOT NULL DEFAULT ''"),
            ("kind", "TEXT NOT NULL DEFAULT 'reference'"),
            ("scope", "TEXT NOT NULL DEFAULT 'user'"),
            ("title", "TEXT NOT NULL DEFAULT ''"),
            ("summary", "TEXT NOT NULL DEFAULT ''"),
            ("proposal_state", "TEXT NOT NULL DEFAULT 'pending'"),
            ("proposal_reason", "TEXT NOT NULL DEFAULT ''"),
            ("review_json", "TEXT"),
            ("artifact_storage_path", "TEXT"),
            ("artifact_content_hash", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("proposal_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    Ok(())
}

pub(super) fn ensure_cost_entry_columns(connection: &Connection) -> Result<(), AppError> {
    let mut stmt = connection
        .prepare("PRAGMA table_info(cost_entries)")
        .map_err(|error| AppError::database(error.to_string()))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

    if !columns.iter().any(|column| column == "configured_model_id") {
        connection
            .execute(
                "ALTER TABLE cost_entries ADD COLUMN configured_model_id TEXT",
                [],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS configured_model_usage_projections (
              configured_model_id TEXT PRIMARY KEY,
              used_tokens INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_token_usage_projections (
              project_id TEXT PRIMARY KEY,
              used_tokens INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    let project_projection_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM project_token_usage_projections",
            [],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    if project_projection_count == 0 {
        rebuild_project_token_usage_projections(connection)?;
    }

    Ok(())
}

fn rebuild_project_token_usage_projections(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute("DELETE FROM project_token_usage_projections", [])
        .map_err(|error| AppError::database(error.to_string()))?;
    connection
        .execute(
            "INSERT INTO project_token_usage_projections (project_id, used_tokens, updated_at)
             SELECT project_id,
                    SUM(CASE WHEN amount > 0 THEN amount ELSE 0 END) AS used_tokens,
                    MAX(created_at) AS updated_at
             FROM cost_entries
             WHERE project_id IS NOT NULL
               AND metric = 'tokens'
             GROUP BY project_id
             HAVING SUM(CASE WHEN amount > 0 THEN amount ELSE 0 END) > 0",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(super) fn ensure_resource_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "resources",
        &[
            ("scope", "TEXT"),
            ("visibility", "TEXT"),
            ("owner_user_id", "TEXT"),
            ("storage_path", "TEXT"),
            ("content_type", "TEXT"),
            ("byte_size", "INTEGER"),
            ("preview_kind", "TEXT"),
            ("source_artifact_id", "TEXT"),
        ],
    )
}

pub(super) fn ensure_knowledge_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "knowledge_records",
        &[
            ("scope", "TEXT"),
            ("visibility", "TEXT"),
            ("owner_user_id", "TEXT"),
        ],
    )?;

    connection
        .execute(
            "UPDATE knowledge_records
             SET scope = CASE
                 WHEN project_id IS NULL THEN 'workspace'
                 ELSE 'project'
             END
             WHERE scope IS NULL OR TRIM(scope) = ''",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    connection
        .execute(
            "UPDATE knowledge_records
             SET visibility = CASE
                 WHEN COALESCE(scope, '') = 'personal' THEN 'private'
                 ELSE 'public'
             END
             WHERE visibility IS NULL OR TRIM(visibility) = ''",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

fn infer_resource_preview_kind(
    kind: &str,
    name: &str,
    location: Option<&str>,
    content_type: Option<&str>,
) -> String {
    if kind == "folder" {
        return "folder".into();
    }
    if kind == "url" {
        return "url".into();
    }

    let content_type = content_type.unwrap_or_default().to_ascii_lowercase();
    if content_type.starts_with("image/") {
        return "image".into();
    }
    if content_type == "application/pdf" {
        return "pdf".into();
    }
    if content_type.starts_with("audio/") {
        return "audio".into();
    }
    if content_type.starts_with("video/") {
        return "video".into();
    }
    if content_type == "text/markdown" {
        return "markdown".into();
    }
    if content_type.starts_with("text/") || content_type == "application/json" {
        let extension = Path::new(name)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .or_else(|| {
                location.and_then(|value| {
                    Path::new(value)
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .map(|extension| extension.to_ascii_lowercase())
                })
            });
        if matches!(
            extension.as_deref(),
            Some(
                "rs" | "ts"
                    | "tsx"
                    | "js"
                    | "jsx"
                    | "vue"
                    | "py"
                    | "go"
                    | "java"
                    | "kt"
                    | "swift"
                    | "c"
                    | "cc"
                    | "cpp"
                    | "h"
                    | "hpp"
                    | "html"
                    | "css"
                    | "json"
                    | "yaml"
                    | "yml"
                    | "toml"
                    | "md"
                    | "sql"
                    | "sh"
            )
        ) {
            return if extension.as_deref() == Some("md") {
                "markdown".into()
            } else {
                "code".into()
            };
        }
        return if content_type == "text/markdown" {
            "markdown".into()
        } else {
            "text".into()
        };
    }

    let lower_name = name.to_ascii_lowercase();
    if lower_name.ends_with(".md") {
        return "markdown".into();
    }
    if lower_name.ends_with(".pdf") {
        return "pdf".into();
    }
    if matches!(
        lower_name.rsplit('.').next(),
        Some("png" | "jpg" | "jpeg" | "webp" | "gif" | "svg")
    ) {
        return "image".into();
    }
    if matches!(
        lower_name.rsplit('.').next(),
        Some("mp3" | "wav" | "ogg" | "m4a")
    ) {
        return "audio".into();
    }
    if matches!(
        lower_name.rsplit('.').next(),
        Some("mp4" | "mov" | "webm" | "avi" | "mkv")
    ) {
        return "video".into();
    }
    if matches!(
        lower_name.rsplit('.').next(),
        Some(
            "rs" | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "vue"
                | "py"
                | "go"
                | "java"
                | "kt"
                | "swift"
                | "c"
                | "cc"
                | "cpp"
                | "h"
                | "hpp"
                | "html"
                | "css"
                | "json"
                | "yaml"
                | "yml"
                | "toml"
                | "sql"
                | "sh"
        )
    ) {
        return "code".into();
    }

    "binary".into()
}

fn infer_resource_content_type(name: &str, location: Option<&str>) -> Option<String> {
    let extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .or_else(|| {
            location.and_then(|value| {
                Path::new(value)
                    .extension()
                    .and_then(|extension| extension.to_str())
            })
        })?
        .to_ascii_lowercase();

    let content_type = match extension.as_str() {
        "md" => "text/markdown",
        "txt" | "csv" | "rs" | "ts" | "tsx" | "js" | "jsx" | "vue" | "py" | "go" | "java"
        | "kt" | "swift" | "c" | "cc" | "cpp" | "h" | "hpp" | "html" | "css" | "yaml" | "yml"
        | "toml" | "sql" | "sh" => "text/plain",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    };

    Some(content_type.into())
}

fn backfill_project_resource_directories(
    connection: &Connection,
    paths: &WorkspacePaths,
) -> Result<(), AppError> {
    let mut stmt = connection
        .prepare("SELECT id, resource_directory FROM projects")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

    for (project_id, stored_directory) in rows {
        let resource_directory = stored_directory
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| paths.default_project_resource_directory(&project_id));
        connection
            .execute(
                "UPDATE projects SET resource_directory = ?2 WHERE id = ?1",
                params![project_id, resource_directory],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        fs::create_dir_all(paths.root.join(&resource_directory))?;
    }

    fs::create_dir_all(&paths.workspace_resources_dir)?;
    Ok(())
}

fn backfill_project_governance(
    connection: &Connection,
    workspace_owner_user_id: Option<&str>,
) -> Result<(), AppError> {
    let resolved_workspace_owner_user_id = workspace_owner_user_id
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string);
    let replace_bootstrap_placeholder = resolved_workspace_owner_user_id.is_some();
    let fallback_owner_user_id = resolved_workspace_owner_user_id
        .clone()
        .unwrap_or_else(|| BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID.to_string());
    let data_policies = load_data_policies(connection)?;
    let selected_project_members = data_policies
        .into_iter()
        .filter(|policy| {
            policy.subject_type == "user"
                && policy.resource_type == "project"
                && policy.scope_type == "selected-projects"
                && policy.effect == "allow"
        })
        .fold(
            std::collections::BTreeMap::<String, Vec<String>>::new(),
            |mut acc, policy| {
                for project_id in policy.project_ids {
                    acc.entry(project_id)
                        .or_default()
                        .push(policy.subject_id.clone());
                }
                acc
            },
        );

    let mut stmt = connection
        .prepare(
            "SELECT id, assignments_json, owner_user_id, member_user_ids_json, permission_overrides_json, linked_workspace_assets_json FROM projects",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

    for (
        project_id,
        assignments_json,
        stored_owner_user_id,
        stored_member_user_ids_json,
        stored_permission_overrides_json,
        stored_linked_workspace_assets_json,
    ) in rows
    {
        let owner_user_id = stored_owner_user_id
            .filter(|value| !value.trim().is_empty())
            .filter(|value| {
                !(replace_bootstrap_placeholder && value == BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID)
            })
            .unwrap_or_else(|| fallback_owner_user_id.clone());
        let member_user_ids = stored_member_user_ids_json
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(serde_json::from_str::<Vec<String>>)
            .transpose()?
            .unwrap_or_else(|| {
                selected_project_members
                    .get(&project_id)
                    .cloned()
                    .unwrap_or_default()
            })
            .into_iter()
            .filter(|user_id| {
                !(replace_bootstrap_placeholder && user_id == BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID)
            })
            .collect::<Vec<_>>();
        let permission_overrides = stored_permission_overrides_json
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(serde_json::from_str::<ProjectPermissionOverrides>)
            .transpose()?
            .unwrap_or_else(default_project_permission_overrides);
        let linked_workspace_assets = stored_linked_workspace_assets_json
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(serde_json::from_str::<ProjectLinkedWorkspaceAssets>)
            .transpose()?
            .unwrap_or_else(|| {
                let assignments = assignments_json
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(serde_json::from_str::<ProjectWorkspaceAssignments>)
                    .transpose()
                    .ok()
                    .flatten();
                ProjectLinkedWorkspaceAssets {
                    agent_ids: assignments
                        .as_ref()
                        .and_then(|value| value.agents.as_ref())
                        .map(|value| value.agent_ids.clone())
                        .unwrap_or_default(),
                    resource_ids: Vec::new(),
                    tool_source_keys: assignments
                        .as_ref()
                        .and_then(|value| value.tools.as_ref())
                        .map(|value| value.source_keys.clone())
                        .unwrap_or_default(),
                    knowledge_ids: Vec::new(),
                }
            });
        let normalized_members =
            normalized_project_member_user_ids(&owner_user_id, member_user_ids);

        connection
            .execute(
                "UPDATE projects
                 SET owner_user_id = ?2,
                     member_user_ids_json = ?3,
                     permission_overrides_json = ?4,
                     linked_workspace_assets_json = ?5
                 WHERE id = ?1",
                params![
                    project_id,
                    owner_user_id,
                    serde_json::to_string(&normalized_members)?,
                    serde_json::to_string(&permission_overrides)?,
                    serde_json::to_string(&linked_workspace_assets)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}

fn backfill_default_project_assignments(connection: &Connection) -> Result<(), AppError> {
    let stored_assignments_json = connection
        .query_row(
            "SELECT assignments_json FROM projects WHERE id = ?1",
            params![DEFAULT_PROJECT_ID],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?
        .flatten();
    let parsed_assignments = stored_assignments_json
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(serde_json::from_str::<ProjectWorkspaceAssignments>)
        .transpose()?;
    let needs_model_backfill = parsed_assignments
        .as_ref()
        .and_then(|assignments| assignments.models.as_ref())
        .is_none_or(|models| {
            models.default_configured_model_id.trim().is_empty()
                || models.configured_model_ids.is_empty()
        });
    if !needs_model_backfill {
        return Ok(());
    }

    let next_assignments = match parsed_assignments {
        Some(mut assignments) => {
            assignments.models = Some(default_project_model_assignments());
            assignments
        }
        None => default_project_assignments(),
    };

    connection
        .execute(
            "UPDATE projects SET assignments_json = ?2 WHERE id = ?1",
            params![
                DEFAULT_PROJECT_ID,
                serde_json::to_string(&next_assignments)?,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(super) fn load_state(paths: WorkspacePaths) -> Result<InfraState, AppError> {
    let workspace_file: WorkspaceConfigFile =
        toml::from_str(&fs::read_to_string(&paths.workspace_config)?)?;
    let mut workspace = WorkspaceSummary {
        id: workspace_file.id,
        name: workspace_file.name,
        slug: workspace_file.slug,
        deployment: workspace_file.deployment,
        bootstrap_status: workspace_file.bootstrap_status,
        owner_user_id: workspace_file.owner_user_id,
        host: workspace_file.host,
        listen_address: workspace_file.listen_address,
        default_project_id: workspace_file.default_project_id,
        project_default_permissions: workspace_file.project_default_permissions,
    };

    let app_registry: AppRegistryFile =
        toml::from_str(&fs::read_to_string(&paths.app_registry_config)?)?;
    let connection =
        Connection::open(&paths.db_path).map_err(|error| AppError::database(error.to_string()))?;
    ensure_default_owner_role_permissions(&connection)?;
    backfill_project_resource_directories(&connection, &paths)?;
    backfill_default_project_assignments(&connection)?;
    let users = load_users(&connection)?;
    let owner_user_id = users
        .iter()
        .find(|user| {
            resolve_effective_role_ids(&connection, &user.record.id)
                .map(|(role_ids, _)| {
                    role_ids
                        .iter()
                        .any(|role_id| role_id == SYSTEM_OWNER_ROLE_ID)
                })
                .unwrap_or(false)
        })
        .map(|user| user.record.id.clone());
    let expected_bootstrap_status = if owner_user_id.is_some() {
        "ready"
    } else {
        "setup_required"
    };
    let workspace_needs_normalize = workspace.bootstrap_status != expected_bootstrap_status
        || workspace.owner_user_id != owner_user_id;
    if workspace_needs_normalize {
        workspace.bootstrap_status = expected_bootstrap_status.into();
        workspace.owner_user_id = owner_user_id;
        bootstrap::save_workspace_config_file(&paths.workspace_config, &workspace)?;
    }
    backfill_project_governance(&connection, workspace.owner_user_id.as_deref())?;
    for user in &users {
        ensure_personal_pet_for_user(&connection, &workspace.id, &user.record.id)?;
    }
    let projects = load_projects(&connection)?;
    let project_promotion_requests = load_project_promotion_requests(&connection)?;
    let sessions = load_sessions(&connection)?;
    let resources = load_resources(&connection)?;
    let knowledge_records = load_knowledge_records(&connection)?;
    let artifacts = load_artifact_records(&connection)?;
    let agents = load_agents(&connection)?;
    let project_agent_links = load_project_agent_links(&connection)?;
    let teams = load_teams(&connection)?;
    let project_team_links = load_project_team_links(&connection)?;
    let model_catalog = load_model_catalog(&connection)?;
    let provider_credentials = load_provider_credentials(&connection)?;
    let tools = load_tools(&connection)?;
    let automations = load_automations(&connection)?;
    let trace_events = load_trace_events(&connection)?;
    let audit_records = load_audit_records(&connection)?;
    let cost_entries = load_cost_entries(&connection)?;
    let pet_extensions = load_pet_agent_extensions(&connection)?;
    let pet_presences = load_pet_presences(&connection)?;
    let pet_bindings = load_pet_bindings(&connection)?;

    Ok(InfraState {
        paths,
        workspace: Mutex::new(workspace),
        users: Mutex::new(users),
        apps: Mutex::new(app_registry.apps),
        sessions: Mutex::new(sessions),
        projects: Mutex::new(projects),
        project_promotion_requests: Mutex::new(project_promotion_requests),
        resources: Mutex::new(resources),
        knowledge_records: Mutex::new(knowledge_records),
        agents: Mutex::new(agents),
        project_agent_links: Mutex::new(project_agent_links),
        teams: Mutex::new(teams),
        project_team_links: Mutex::new(project_team_links),
        model_catalog: Mutex::new(model_catalog),
        provider_credentials: Mutex::new(provider_credentials),
        tools: Mutex::new(tools),
        automations: Mutex::new(automations),
        artifacts: Mutex::new(artifacts),
        inbox: Mutex::new(Vec::new()),
        trace_events: Mutex::new(trace_events),
        audit_records: Mutex::new(audit_records),
        cost_entries: Mutex::new(cost_entries),
        pet_extensions: Mutex::new(pet_extensions),
        pet_presences: Mutex::new(pet_presences),
        pet_bindings: Mutex::new(pet_bindings),
    })
}

pub(super) fn load_users(connection: &Connection) -> Result<Vec<StoredUser>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, username, display_name, avatar_path, avatar_content_type, avatar_byte_size, avatar_content_hash,
                    status, password_hash, password_state, created_at, updated_at
             FROM users",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(StoredUser {
                record: UserRecord {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    display_name: row.get(2)?,
                    avatar_path: row.get(3)?,
                    avatar_content_type: row.get(4)?,
                    avatar_byte_size: row.get::<_, Option<i64>>(5)?.map(|value| value as u64),
                    avatar_content_hash: row.get(6)?,
                    status: row.get(7)?,
                    password_state: row.get(9)?,
                    created_at: row.get::<_, i64>(10)? as u64,
                    updated_at: row.get::<_, i64>(11)? as u64,
                },
                password_hash: row.get(8)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn drop_legacy_access_control_tables(connection: &Connection) -> Result<(), AppError> {
    for table in ["memberships", "roles", "permissions", "menus"] {
        connection
            .execute(&format!("DROP TABLE IF EXISTS {table}"), [])
            .map_err(|error| AppError::database(error.to_string()))?;
    }
    Ok(())
}

fn reset_legacy_sessions_table(connection: &Connection) -> Result<(), AppError> {
    let table_exists = connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'sessions' LIMIT 1",
            [],
            |_| Ok(()),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?
        .is_some();
    if !table_exists {
        return Ok(());
    }

    let expected_columns = [
        "id",
        "workspace_id",
        "user_id",
        "client_app_id",
        "token",
        "status",
        "created_at",
        "expires_at",
    ];
    let mut pragma = connection
        .prepare("PRAGMA table_info(sessions)")
        .map_err(|error| AppError::database(error.to_string()))?;
    let columns = pragma
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

    if columns != expected_columns {
        connection
            .execute("DROP TABLE sessions", [])
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}

pub(super) fn load_projects(connection: &Connection) -> Result<Vec<ProjectRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, name, status, description, resource_directory, assignments_json, owner_user_id, member_user_ids_json, permission_overrides_json, linked_workspace_assets_json FROM projects",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let assignments_json: Option<String> = row.get(6)?;
            let assignments = assignments_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<ProjectWorkspaceAssignments>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        6,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            let owner_user_id: Option<String> = row.get(7)?;
            let member_user_ids_json: Option<String> = row.get(8)?;
            let member_user_ids = member_user_ids_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<Vec<String>>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        8,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?
                .unwrap_or_default();
            let permission_overrides_json: Option<String> = row.get(9)?;
            let permission_overrides = permission_overrides_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<ProjectPermissionOverrides>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        9,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?
                .unwrap_or_else(default_project_permission_overrides);
            let linked_workspace_assets_json: Option<String> = row.get(10)?;
            let linked_workspace_assets = linked_workspace_assets_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<ProjectLinkedWorkspaceAssets>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        10,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?
                .unwrap_or_else(empty_project_linked_workspace_assets);
            let owner_user_id = owner_user_id.unwrap_or_else(|| "user-owner".into());
            Ok(ProjectRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                status: row.get(3)?,
                description: row.get(4)?,
                resource_directory: row.get(5)?,
                owner_user_id: owner_user_id.clone(),
                member_user_ids: normalized_project_member_user_ids(
                    &owner_user_id,
                    member_user_ids,
                ),
                permission_overrides,
                linked_workspace_assets,
                assignments,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_project_promotion_requests(
    connection: &Connection,
) -> Result<Vec<ProjectPromotionRequest>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, asset_type, asset_id, requested_by_user_id, submitted_by_owner_user_id, required_workspace_capability, status, reviewed_by_user_id, review_comment, created_at, updated_at, reviewed_at
             FROM project_promotion_requests
             ORDER BY created_at DESC, id DESC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectPromotionRequest {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                asset_type: row.get(3)?,
                asset_id: row.get(4)?,
                requested_by_user_id: row.get(5)?,
                submitted_by_owner_user_id: row.get(6)?,
                required_workspace_capability: row.get(7)?,
                status: row.get(8)?,
                reviewed_by_user_id: row.get(9)?,
                review_comment: row.get(10)?,
                created_at: row.get::<_, i64>(11)? as u64,
                updated_at: row.get::<_, i64>(12)? as u64,
                reviewed_at: row.get::<_, Option<i64>>(13)?.map(|value| value as u64),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn personal_pet_defaults(
    workspace_id: &str,
    owner_user_id: &str,
) -> (String, PetAgentExtensionRecord) {
    let mut hasher = Sha256::new();
    hasher.update(workspace_id.as_bytes());
    hasher.update(b":");
    hasher.update(owner_user_id.as_bytes());
    let digest = hasher.finalize();
    let species =
        PERSONAL_PET_SPECIES_REGISTRY[(digest[0] as usize) % PERSONAL_PET_SPECIES_REGISTRY.len()];
    let pet_id = format!("pet-{owner_user_id}");
    let display_name = format!("{}伙伴", species);
    let summary = format!("{display_name} 会陪着主人一起完成日常工作。");
    let greeting = format!("嗨，我是 {display_name}，今天一起推进事情吧。");
    let favorite_snack = match species {
        "duck" | "goose" => "玉米粒",
        "cat" | "dragon" | "octopus" => "新鲜小虾",
        "owl" | "ghost" => "夜宵",
        "penguin" | "turtle" | "snail" => "海藻沙拉",
        "axolotl" | "capybara" => "蔬果拼盘",
        "cactus" | "robot" => "阳光和电量",
        "rabbit" | "mushroom" | "chonk" | "blob" => "胡萝卜饼干",
        _ => "零食",
    };
    let extension = PetAgentExtensionRecord {
        pet_id: pet_id.clone(),
        workspace_id: workspace_id.into(),
        owner_user_id: owner_user_id.into(),
        species: species.into(),
        display_name,
        avatar_label: format!("{species} mascot"),
        summary,
        greeting,
        mood: "happy".into(),
        favorite_snack: favorite_snack.into(),
        prompt_hints: vec![
            "帮我整理一下今天的重点".into(),
            "我们接下来先做什么？".into(),
            "给我一句鼓励的话".into(),
        ],
        fallback_asset: species.into(),
        rive_asset: None,
        state_machine: None,
        updated_at: timestamp_now(),
    };
    (pet_id, extension)
}

pub(super) fn pet_context_key(owner_user_id: &str, project_id: Option<&str>) -> String {
    match project_id {
        Some(project_id) if !project_id.trim().is_empty() => {
            format!("{owner_user_id}::{PET_CONTEXT_SCOPE_PROJECT}::{project_id}")
        }
        _ => format!("{owner_user_id}::{PET_CONTEXT_SCOPE_HOME}"),
    }
}

pub(super) fn default_pet_profile(
    pet_id: &str,
    owner_user_id: &str,
    extension: &PetAgentExtensionRecord,
) -> PetProfile {
    PetProfile {
        id: pet_id.into(),
        species: extension.species.clone(),
        display_name: extension.display_name.clone(),
        owner_user_id: owner_user_id.into(),
        avatar_label: extension.avatar_label.clone(),
        summary: extension.summary.clone(),
        greeting: extension.greeting.clone(),
        mood: extension.mood.clone(),
        favorite_snack: extension.favorite_snack.clone(),
        prompt_hints: extension.prompt_hints.clone(),
        fallback_asset: extension.fallback_asset.clone(),
        rive_asset: extension.rive_asset.clone(),
        state_machine: extension.state_machine.clone(),
    }
}

pub(super) fn default_workspace_pet_presence_for(pet_id: &str) -> PetPresenceState {
    PetPresenceState {
        pet_id: pet_id.into(),
        is_visible: true,
        chat_open: false,
        motion_state: "idle".into(),
        unread_count: 0,
        last_interaction_at: 0,
        position: PetPosition { x: 0, y: 0 },
    }
}

pub(super) fn map_pet_message(pet_id: &str, message: &octopus_core::RuntimeMessage) -> PetMessage {
    PetMessage {
        id: message.id.clone(),
        pet_id: pet_id.into(),
        sender: if message.sender_type == "assistant" {
            "pet".into()
        } else {
            "user".into()
        },
        content: message.content.clone(),
        timestamp: message.timestamp,
    }
}

pub(super) fn load_runtime_messages_for_conversation(
    connection: &Connection,
    conversation_id: &str,
    pet_id: &str,
) -> Result<Vec<PetMessage>, AppError> {
    let detail_json: Option<String> = connection
        .query_row(
            "SELECT detail_json FROM runtime_session_projections WHERE conversation_id = ?1 ORDER BY updated_at DESC LIMIT 1",
            params![conversation_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    let Some(detail_json) = detail_json else {
        return Ok(vec![]);
    };
    let detail: octopus_core::RuntimeSessionDetail = serde_json::from_str(&detail_json)?;
    Ok(detail
        .messages
        .iter()
        .map(|message| map_pet_message(pet_id, message))
        .collect())
}

pub(super) fn row_to_pet_presence(row: &rusqlite::Row<'_>) -> rusqlite::Result<PetPresenceState> {
    Ok(PetPresenceState {
        pet_id: row.get(4)?,
        is_visible: row.get::<_, i64>(5)? != 0,
        chat_open: row.get::<_, i64>(6)? != 0,
        motion_state: row.get(7)?,
        unread_count: row.get::<_, i64>(8)? as u64,
        last_interaction_at: row.get::<_, i64>(9)? as u64,
        position: PetPosition {
            x: row.get(10)?,
            y: row.get(11)?,
        },
    })
}

pub(super) fn load_pet_presences(
    connection: &Connection,
) -> Result<HashMap<String, PetPresenceState>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT scope_key, owner_user_id, context_scope, project_id, pet_id, is_visible, chat_open, motion_state, unread_count, last_interaction_at, position_x, position_y FROM pet_presence",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row_to_pet_presence(row)?))
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<HashMap<_, _>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn row_to_pet_binding(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<PetConversationBinding> {
    Ok(PetConversationBinding {
        pet_id: row.get(4)?,
        workspace_id: row.get(5)?,
        owner_user_id: row
            .get::<_, Option<String>>(1)?
            .unwrap_or_else(|| BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID.into()),
        context_scope: row
            .get::<_, Option<String>>(2)?
            .unwrap_or_else(|| PET_CONTEXT_SCOPE_HOME.into()),
        project_id: row.get(3)?,
        conversation_id: row.get(6)?,
        session_id: row.get(7)?,
        updated_at: row.get::<_, i64>(8)? as u64,
    })
}

pub(super) fn load_pet_bindings(
    connection: &Connection,
) -> Result<HashMap<String, PetConversationBinding>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT scope_key, owner_user_id, context_scope, project_id, pet_id, workspace_id, conversation_id, session_id, updated_at FROM pet_conversation_bindings",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row_to_pet_binding(row)?))
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<HashMap<_, _>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_pet_agent_extensions(
    connection: &Connection,
) -> Result<HashMap<String, PetAgentExtensionRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT pet_id, workspace_id, owner_user_id, species, display_name, avatar_label,
                    summary, greeting, mood, favorite_snack, prompt_hints_json, fallback_asset,
                    rive_asset, state_machine, updated_at
             FROM pet_agent_extensions",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let prompt_hints_raw: String = row.get(10)?;
            Ok((
                row.get::<_, String>(2)?,
                PetAgentExtensionRecord {
                    pet_id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    owner_user_id: row.get(2)?,
                    species: row.get(3)?,
                    display_name: row.get(4)?,
                    avatar_label: row.get(5)?,
                    summary: row.get(6)?,
                    greeting: row.get(7)?,
                    mood: row.get(8)?,
                    favorite_snack: row.get(9)?,
                    prompt_hints: serde_json::from_str(&prompt_hints_raw).unwrap_or_default(),
                    fallback_asset: row.get(11)?,
                    rive_asset: row.get(12)?,
                    state_machine: row.get(13)?,
                    updated_at: row.get::<_, i64>(14)? as u64,
                },
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<HashMap<_, _>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn ensure_personal_pet_for_user(
    connection: &Connection,
    workspace_id: &str,
    owner_user_id: &str,
) -> Result<(), AppError> {
    let existing_pet_id: Option<String> = connection
        .query_row(
            "SELECT pet_id FROM pet_agent_extensions WHERE workspace_id = ?1 AND owner_user_id = ?2",
            params![workspace_id, owner_user_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if existing_pet_id.is_some() {
        return Ok(());
    }

    let (pet_id, extension) = personal_pet_defaults(workspace_id, owner_user_id);
    let pet_record = AgentRecord {
        id: pet_id.clone(),
        workspace_id: workspace_id.into(),
        project_id: None,
        scope: "personal".into(),
        owner_user_id: Some(owner_user_id.into()),
        asset_role: PERSONAL_PET_ASSET_ROLE.into(),
        name: extension.display_name.clone(),
        avatar_path: None,
        avatar: None,
        personality: extension.summary.clone(),
        tags: vec!["pet".into(), extension.species.clone()],
        prompt: format!(
            "{} 你是 {} 的个人宠物伙伴，保持亲切、轻量、鼓励式的交流。",
            extension.greeting, owner_user_id
        ),
        builtin_tool_keys: Vec::new(),
        skill_ids: Vec::new(),
        mcp_server_names: Vec::new(),
        task_domains: normalize_task_domains(Vec::new()),
        manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
        default_model_strategy: default_model_strategy(),
        capability_policy: capability_policy_from_sources(&[], &[], &[]),
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
        description: extension.summary.clone(),
        status: "active".into(),
        updated_at: extension.updated_at,
    };
    write_agent_record(connection, &pet_record, false)?;
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
                extension.pet_id,
                extension.workspace_id,
                extension.owner_user_id,
                extension.species,
                extension.display_name,
                extension.avatar_label,
                extension.summary,
                extension.greeting,
                extension.mood,
                extension.favorite_snack,
                json_string(&extension.prompt_hints)?,
                extension.fallback_asset,
                extension.rive_asset,
                extension.state_machine,
                extension.updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(super) fn load_resources(
    connection: &Connection,
) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, kind, name, location, origin, scope, visibility, owner_user_id, storage_path, content_type, byte_size, preview_kind, status, updated_at, tags, source_artifact_id FROM resources",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let kind: String = row.get(3)?;
            let name: String = row.get(4)?;
            let location: Option<String> = row.get(5)?;
            let content_type = row
                .get::<_, Option<String>>(11)?
                .or_else(|| infer_resource_content_type(&name, location.as_deref()));
            let preview_kind = row.get::<_, Option<String>>(13)?.unwrap_or_else(|| {
                infer_resource_preview_kind(
                    &kind,
                    &name,
                    location.as_deref(),
                    content_type.as_deref(),
                )
            });
            let tags_raw: String = row.get(16)?;
            Ok(WorkspaceResourceRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                kind: kind.clone(),
                name: name.clone(),
                location,
                origin: row.get(6)?,
                scope: row
                    .get::<_, Option<String>>(7)?
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| {
                        if row.get::<_, Option<String>>(2).ok().flatten().is_some() {
                            "project".into()
                        } else {
                            "workspace".into()
                        }
                    }),
                visibility: row
                    .get::<_, Option<String>>(8)?
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "public".into()),
                owner_user_id: row
                    .get::<_, Option<String>>(9)?
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "user-owner".into()),
                storage_path: row.get(10)?,
                content_type,
                byte_size: row.get::<_, Option<i64>>(12)?.map(|value| value as u64),
                preview_kind,
                status: row.get(14)?,
                updated_at: row.get::<_, i64>(15)? as u64,
                tags: serde_json::from_str(&tags_raw).unwrap_or_default(),
                source_artifact_id: row.get(17)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_knowledge_records(
    connection: &Connection,
) -> Result<Vec<KnowledgeRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT
                id,
                workspace_id,
                project_id,
                title,
                summary,
                kind,
                COALESCE(scope, CASE WHEN project_id IS NULL THEN 'workspace' ELSE 'project' END) AS scope,
                status,
                COALESCE(
                    visibility,
                    CASE
                        WHEN COALESCE(scope, CASE WHEN project_id IS NULL THEN 'workspace' ELSE 'project' END) = 'personal'
                            THEN 'private'
                        ELSE 'public'
                    END
                ) AS visibility,
                owner_user_id,
                source_type,
                source_ref,
                updated_at
             FROM knowledge_records",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(KnowledgeRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                title: row.get(3)?,
                summary: row.get(4)?,
                kind: row.get(5)?,
                scope: row.get(6)?,
                status: row.get(7)?,
                visibility: row.get(8)?,
                owner_user_id: row.get(9)?,
                source_type: row.get(10)?,
                source_ref: row.get(11)?,
                updated_at: row.get::<_, i64>(12)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_artifact_records(
    connection: &Connection,
) -> Result<Vec<ArtifactRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, conversation_id, title, status, preview_kind,
                    latest_version, promotion_state, updated_at, content_type
             FROM artifact_records
             ORDER BY updated_at DESC, id ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let id = row.get::<_, String>(0)?;
            let title = row.get::<_, String>(4)?;
            let preview_kind = row.get::<_, String>(6)?;
            let latest_version = row.get::<_, i64>(7)?.max(0) as u32;
            let updated_at = row.get::<_, i64>(9)?.max(0) as u64;
            let content_type = row.get::<_, Option<String>>(10)?;
            Ok(ArtifactRecord {
                id: id.clone(),
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                conversation_id: row.get(3)?,
                title: title.clone(),
                status: row.get(5)?,
                preview_kind: preview_kind.clone(),
                latest_version,
                latest_version_ref: ArtifactVersionReference {
                    artifact_id: id,
                    version: latest_version,
                    title,
                    preview_kind,
                    updated_at,
                    content_type: content_type.clone(),
                },
                promotion_state: row.get(8)?,
                updated_at,
                content_type,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn agent_avatar(paths: &WorkspacePaths, avatar_path: Option<&str>) -> Option<String> {
    let avatar_path = avatar_path?;
    let absolute_path = paths.root.join(avatar_path);
    let bytes = fs::read(&absolute_path).ok()?;
    let content_type = match absolute_path
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        _ => return Some(avatar_path.to_string()),
    };
    Some(format!(
        "data:{content_type};base64,{}",
        BASE64_STANDARD.encode(bytes)
    ))
}

pub(super) fn load_agents(connection: &Connection) -> Result<Vec<AgentRecord>, AppError> {
    let workspace_root = connection
        .path()
        .map(Path::new)
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| AppError::database("could not resolve workspace root"))?;
    let paths = WorkspacePaths::new(workspace_root);
    let mut stmt = connection
        .prepare(
            "SELECT
                id, workspace_id, project_id, scope, owner_user_id, asset_role, name, avatar_path, personality, tags, prompt,
                builtin_tool_keys, skill_ids, mcp_server_names, task_domains, manifest_revision,
                default_model_strategy_json, capability_policy_json, permission_envelope_json,
                memory_policy_json, delegation_policy_json, approval_preference_json,
                output_contract_json, shared_capability_policy_json, integration_source_json,
                trust_metadata_json, dependency_resolution_json, import_metadata_json,
                description, status, updated_at
             FROM agents",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let avatar_path: Option<String> = row.get(7)?;
            let avatar = agent_avatar(&paths, avatar_path.as_deref());
            let tags_raw: String = row.get(9)?;
            let builtin_tool_keys_raw: String = row.get(11)?;
            let skill_ids_raw: String = row.get(12)?;
            let mcp_server_names_raw: String = row.get(13)?;
            let task_domains_raw: String = row.get(14)?;
            let builtin_tool_keys: Vec<String> =
                serde_json::from_str(&builtin_tool_keys_raw).unwrap_or_default();
            let skill_ids: Vec<String> = serde_json::from_str(&skill_ids_raw).unwrap_or_default();
            let mcp_server_names: Vec<String> =
                serde_json::from_str(&mcp_server_names_raw).unwrap_or_default();
            let default_model_strategy_raw: String = row.get(16)?;
            let capability_policy_raw: String = row.get(17)?;
            let permission_envelope_raw: String = row.get(18)?;
            let memory_policy_raw: String = row.get(19)?;
            let delegation_policy_raw: String = row.get(20)?;
            let approval_preference_raw: String = row.get(21)?;
            let output_contract_raw: String = row.get(22)?;
            let shared_capability_policy_raw: String = row.get(23)?;
            let integration_source_raw: Option<String> = row.get(24)?;
            let trust_metadata_raw: String = row.get(25)?;
            let dependency_resolution_raw: String = row.get(26)?;
            let import_metadata_raw: String = row.get(27)?;
            Ok(AgentRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                scope: row.get(3)?,
                owner_user_id: row.get(4)?,
                asset_role: row
                    .get::<_, Option<String>>(5)?
                    .unwrap_or_else(octopus_core::default_agent_asset_role),
                name: row.get(6)?,
                avatar_path,
                avatar,
                personality: row.get(8)?,
                tags: serde_json::from_str(&tags_raw).unwrap_or_default(),
                prompt: row.get(10)?,
                builtin_tool_keys: builtin_tool_keys.clone(),
                skill_ids: skill_ids.clone(),
                mcp_server_names: mcp_server_names.clone(),
                task_domains: parse_json_or_default(&task_domains_raw, || {
                    normalize_task_domains(Vec::new())
                }),
                manifest_revision: row.get(15)?,
                default_model_strategy: parse_json_or_default(
                    &default_model_strategy_raw,
                    default_model_strategy,
                ),
                capability_policy: parse_json_or_default(&capability_policy_raw, || {
                    capability_policy_from_sources(
                        &builtin_tool_keys,
                        &skill_ids,
                        &mcp_server_names,
                    )
                }),
                permission_envelope: parse_json_or_default(
                    &permission_envelope_raw,
                    default_permission_envelope,
                ),
                memory_policy: parse_json_or_default(
                    &memory_policy_raw,
                    default_agent_memory_policy,
                ),
                delegation_policy: parse_json_or_default(
                    &delegation_policy_raw,
                    default_agent_delegation_policy,
                ),
                approval_preference: parse_json_or_default(
                    &approval_preference_raw,
                    default_approval_preference,
                ),
                output_contract: parse_json_or_default(
                    &output_contract_raw,
                    default_output_contract,
                ),
                shared_capability_policy: parse_json_or_default(
                    &shared_capability_policy_raw,
                    default_agent_shared_capability_policy,
                ),
                integration_source: integration_source_raw
                    .as_deref()
                    .and_then(|value| serde_json::from_str(value).ok()),
                trust_metadata: parse_json_or_default(
                    &trust_metadata_raw,
                    default_asset_trust_metadata,
                ),
                dependency_resolution: parse_json_or_default(&dependency_resolution_raw, Vec::new),
                import_metadata: parse_json_or_default(
                    &import_metadata_raw,
                    default_asset_import_metadata,
                ),
                description: row.get(28)?,
                status: row.get(29)?,
                updated_at: row.get::<_, i64>(30)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_project_agent_links(
    connection: &Connection,
) -> Result<Vec<ProjectAgentLinkRecord>, AppError> {
    let mut stmt = connection
        .prepare("SELECT workspace_id, project_id, agent_id, linked_at FROM project_agent_links")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectAgentLinkRecord {
                workspace_id: row.get(0)?,
                project_id: row.get(1)?,
                agent_id: row.get(2)?,
                linked_at: row.get::<_, i64>(3)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_bundle_asset_descriptor_records(
    connection: &Connection,
) -> Result<Vec<BundleAssetDescriptorRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT
                id, workspace_id, project_id, scope, asset_kind, source_id, display_name,
                source_path, storage_path, content_hash, byte_size, manifest_revision,
                task_domains_json, translation_mode, trust_metadata_json,
                dependency_resolution_json, import_metadata_json, updated_at
             FROM bundle_asset_descriptors",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let task_domains_raw: String = row.get(12)?;
            let trust_metadata_raw: String = row.get(14)?;
            let dependency_resolution_raw: String = row.get(15)?;
            let import_metadata_raw: String = row.get(16)?;
            Ok(BundleAssetDescriptorRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                scope: row.get(3)?,
                asset_kind: row.get(4)?,
                source_id: row.get(5)?,
                display_name: row.get(6)?,
                source_path: row.get(7)?,
                storage_path: row.get(8)?,
                content_hash: row.get(9)?,
                byte_size: row.get::<_, i64>(10)? as u64,
                manifest_revision: row.get(11)?,
                task_domains: parse_json_or_default(&task_domains_raw, Vec::new),
                translation_mode: row.get(13)?,
                trust_metadata: parse_json_or_default(
                    &trust_metadata_raw,
                    default_asset_trust_metadata,
                ),
                dependency_resolution: parse_json_or_default(&dependency_resolution_raw, Vec::new),
                import_metadata: parse_json_or_default(
                    &import_metadata_raw,
                    default_asset_import_metadata,
                ),
                updated_at: row.get::<_, i64>(17)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_teams(connection: &Connection) -> Result<Vec<TeamRecord>, AppError> {
    let workspace_root = connection
        .path()
        .map(Path::new)
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| AppError::database("could not resolve workspace root"))?;
    let paths = WorkspacePaths::new(workspace_root);
    let mut stmt = connection
        .prepare(
            "SELECT
                id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt,
                builtin_tool_keys, skill_ids, mcp_server_names, task_domains, manifest_revision,
                default_model_strategy_json, capability_policy_json, permission_envelope_json,
                memory_policy_json, delegation_policy_json, approval_preference_json,
                output_contract_json, shared_capability_policy_json, leader_agent_id,
                member_agent_ids, leader_ref, member_refs, team_topology_json,
                shared_memory_policy_json, mailbox_policy_json, artifact_handoff_policy_json,
                workflow_affordance_json, worker_concurrency_limit, integration_source_json,
                trust_metadata_json, dependency_resolution_json, import_metadata_json,
                description, status, updated_at
             FROM teams",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let avatar_path: Option<String> = row.get(5)?;
            let avatar = agent_avatar(&paths, avatar_path.as_deref());
            let tags_raw: String = row.get(7)?;
            let builtin_tool_keys_raw: String = row.get(9)?;
            let skill_ids_raw: String = row.get(10)?;
            let mcp_server_names_raw: String = row.get(11)?;
            let task_domains_raw: String = row.get(12)?;
            let builtin_tool_keys: Vec<String> =
                serde_json::from_str(&builtin_tool_keys_raw).unwrap_or_default();
            let skill_ids: Vec<String> = serde_json::from_str(&skill_ids_raw).unwrap_or_default();
            let mcp_server_names: Vec<String> =
                serde_json::from_str(&mcp_server_names_raw).unwrap_or_default();
            let default_model_strategy_raw: String = row.get(14)?;
            let capability_policy_raw: String = row.get(15)?;
            let permission_envelope_raw: String = row.get(16)?;
            let memory_policy_raw: String = row.get(17)?;
            let delegation_policy_raw: String = row.get(18)?;
            let approval_preference_raw: String = row.get(19)?;
            let output_contract_raw: String = row.get(20)?;
            let shared_capability_policy_raw: String = row.get(21)?;
            let member_agent_ids_raw: String = row.get(23)?;
            let member_refs_raw: String = row.get(25)?;
            let team_topology_raw: String = row.get(26)?;
            let shared_memory_policy_raw: String = row.get(27)?;
            let mailbox_policy_raw: String = row.get(28)?;
            let artifact_handoff_policy_raw: String = row.get(29)?;
            let workflow_affordance_raw: String = row.get(30)?;
            let integration_source_raw: Option<String> = row.get(32)?;
            let trust_metadata_raw: String = row.get(33)?;
            let dependency_resolution_raw: String = row.get(34)?;
            let import_metadata_raw: String = row.get(35)?;
            let leader_agent_id: Option<String> = row.get(22)?;
            let member_agent_ids: Vec<String> =
                serde_json::from_str(&member_agent_ids_raw).unwrap_or_default();
            let leader_ref: String = row.get(24)?;
            let member_refs = parse_json_or_default(&member_refs_raw, Vec::new);
            Ok(TeamRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                scope: row.get(3)?,
                name: row.get(4)?,
                avatar_path,
                avatar,
                personality: row.get(6)?,
                tags: serde_json::from_str(&tags_raw).unwrap_or_default(),
                prompt: row.get(8)?,
                builtin_tool_keys: builtin_tool_keys.clone(),
                skill_ids: skill_ids.clone(),
                mcp_server_names: mcp_server_names.clone(),
                task_domains: parse_json_or_default(&task_domains_raw, || {
                    normalize_task_domains(Vec::new())
                }),
                manifest_revision: row.get(13)?,
                default_model_strategy: parse_json_or_default(
                    &default_model_strategy_raw,
                    default_model_strategy,
                ),
                capability_policy: parse_json_or_default(&capability_policy_raw, || {
                    capability_policy_from_sources(
                        &builtin_tool_keys,
                        &skill_ids,
                        &mcp_server_names,
                    )
                }),
                permission_envelope: parse_json_or_default(
                    &permission_envelope_raw,
                    default_permission_envelope,
                ),
                memory_policy: parse_json_or_default(
                    &memory_policy_raw,
                    default_team_memory_policy,
                ),
                delegation_policy: parse_json_or_default(
                    &delegation_policy_raw,
                    default_team_delegation_policy,
                ),
                approval_preference: parse_json_or_default(
                    &approval_preference_raw,
                    default_approval_preference,
                ),
                output_contract: parse_json_or_default(
                    &output_contract_raw,
                    default_output_contract,
                ),
                shared_capability_policy: parse_json_or_default(
                    &shared_capability_policy_raw,
                    default_team_shared_capability_policy,
                ),
                leader_agent_id: leader_agent_id.clone(),
                member_agent_ids: member_agent_ids.clone(),
                leader_ref: leader_ref.clone(),
                member_refs: member_refs.clone(),
                team_topology: parse_json_or_default(&team_topology_raw, || {
                    team_topology_from_refs(Some(leader_ref.clone()), member_refs.clone())
                }),
                shared_memory_policy: parse_json_or_default(
                    &shared_memory_policy_raw,
                    default_shared_memory_policy,
                ),
                mailbox_policy: parse_json_or_default(&mailbox_policy_raw, default_mailbox_policy),
                artifact_handoff_policy: parse_json_or_default(
                    &artifact_handoff_policy_raw,
                    default_artifact_handoff_policy,
                ),
                workflow_affordance: parse_json_or_default(&workflow_affordance_raw, || {
                    workflow_affordance_from_task_domains(&Vec::new(), true, true)
                }),
                worker_concurrency_limit: row.get::<_, i64>(31)? as u64,
                integration_source: integration_source_raw
                    .as_deref()
                    .and_then(|value| serde_json::from_str(value).ok()),
                trust_metadata: parse_json_or_default(
                    &trust_metadata_raw,
                    default_asset_trust_metadata,
                ),
                dependency_resolution: parse_json_or_default(&dependency_resolution_raw, Vec::new),
                import_metadata: parse_json_or_default(
                    &import_metadata_raw,
                    default_asset_import_metadata,
                ),
                description: row.get(36)?,
                status: row.get(37)?,
                updated_at: row.get::<_, i64>(38)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_project_team_links(
    connection: &Connection,
) -> Result<Vec<ProjectTeamLinkRecord>, AppError> {
    let mut stmt = connection
        .prepare("SELECT workspace_id, project_id, team_id, linked_at FROM project_team_links")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectTeamLinkRecord {
                workspace_id: row.get(0)?,
                project_id: row.get(1)?,
                team_id: row.get(2)?,
                linked_at: row.get::<_, i64>(3)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_model_catalog(
    connection: &Connection,
) -> Result<Vec<ModelCatalogRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, label, provider, description, recommended_for, availability, default_permission FROM model_catalog",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ModelCatalogRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                label: row.get(2)?,
                provider: row.get(3)?,
                description: row.get(4)?,
                recommended_for: row.get(5)?,
                availability: row.get(6)?,
                default_permission: row.get(7)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_provider_credentials(
    connection: &Connection,
) -> Result<Vec<ProviderCredentialRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, provider, name, base_url, status FROM provider_credentials",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProviderCredentialRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                provider: row.get(2)?,
                name: row.get(3)?,
                base_url: row.get(4)?,
                status: row.get(5)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_tools(connection: &Connection) -> Result<Vec<ToolRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, kind, name, description, status, permission_mode, updated_at FROM tools",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ToolRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                kind: row.get(2)?,
                name: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                permission_mode: row.get(6)?,
                updated_at: row.get::<_, i64>(7)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_automations(connection: &Connection) -> Result<Vec<AutomationRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, title, description, cadence, owner_type, owner_id, status, next_run_at, last_run_at, output FROM automations",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(AutomationRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                cadence: row.get(5)?,
                owner_type: row.get(6)?,
                owner_id: row.get(7)?,
                status: row.get(8)?,
                next_run_at: row.get::<_, Option<i64>>(9)?.map(|value| value as u64),
                last_run_at: row.get::<_, Option<i64>>(10)?.map(|value| value as u64),
                output: row.get(11)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_sessions(connection: &Connection) -> Result<Vec<SessionRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, user_id, client_app_id, token, status, created_at, expires_at
             FROM sessions",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(SessionRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                user_id: row.get(2)?,
                client_app_id: row.get(3)?,
                token: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get::<_, i64>(6)? as u64,
                expires_at: row.get::<_, Option<i64>>(7)?.map(|value| value as u64),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_trace_events(
    connection: &Connection,
) -> Result<Vec<TraceEventRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, run_id, session_id, event_kind, title, detail, created_at
             FROM trace_events ORDER BY created_at ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(TraceEventRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                run_id: row.get(3)?,
                session_id: row.get(4)?,
                event_kind: row.get(5)?,
                title: row.get(6)?,
                detail: row.get(7)?,
                created_at: row.get::<_, i64>(8)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_audit_records(connection: &Connection) -> Result<Vec<AuditRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, actor_type, actor_id, action, resource, outcome, created_at
             FROM audit_records ORDER BY created_at ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(AuditRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                actor_type: row.get(3)?,
                actor_id: row.get(4)?,
                action: row.get(5)?,
                resource: row.get(6)?,
                outcome: row.get(7)?,
                created_at: row.get::<_, i64>(8)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_cost_entries(connection: &Connection) -> Result<Vec<CostLedgerEntry>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, run_id, configured_model_id, metric, amount, unit, created_at
             FROM cost_entries ORDER BY created_at ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(CostLedgerEntry {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                run_id: row.get(3)?,
                configured_model_id: row.get(4)?,
                metric: row.get(5)?,
                amount: row.get(6)?,
                unit: row.get(7)?,
                created_at: row.get::<_, i64>(8)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn default_workspace_resources() -> Vec<WorkspaceResourceRecord> {
    let now = timestamp_now();
    vec![
        WorkspaceResourceRecord {
            id: "res-workspace-handbook".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: None,
            kind: "file".into(),
            name: "Workspace Handbook".into(),
            location: Some("/docs/workspace-handbook.md".into()),
            origin: "source".into(),
            scope: "workspace".into(),
            visibility: "public".into(),
            owner_user_id: "user-owner".into(),
            storage_path: Some("data/resources/workspace/workspace-handbook.md".into()),
            content_type: Some("text/markdown".into()),
            byte_size: Some(63),
            preview_kind: "markdown".into(),
            status: "healthy".into(),
            updated_at: now,
            tags: vec!["workspace".into(), "handbook".into()],
            source_artifact_id: None,
        },
        WorkspaceResourceRecord {
            id: "res-project-board".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: Some(DEFAULT_PROJECT_ID.into()),
            kind: "folder".into(),
            name: "Project Delivery Board".into(),
            location: Some("/projects/default".into()),
            origin: "generated".into(),
            scope: "project".into(),
            visibility: "public".into(),
            owner_user_id: "user-owner".into(),
            storage_path: Some(format!(
                "data/projects/{DEFAULT_PROJECT_ID}/resources/delivery-board"
            )),
            content_type: None,
            byte_size: None,
            preview_kind: "folder".into(),
            status: "configured".into(),
            updated_at: now,
            tags: vec!["project".into(), "delivery".into()],
            source_artifact_id: None,
        },
    ]
}

pub(super) fn default_knowledge_records() -> Vec<KnowledgeRecord> {
    let now = timestamp_now();
    vec![
        KnowledgeRecord {
            id: "kn-workspace-onboarding".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: None,
            title: "Workspace onboarding".into(),
            summary: "Shared operating rules, review expectations, and release cadence for this workspace.".into(),
            kind: "shared".into(),
            scope: "workspace".into(),
            status: "shared".into(),
            visibility: "public".into(),
            owner_user_id: None,
            source_type: "artifact".into(),
            source_ref: "workspace-handbook".into(),
            updated_at: now,
        },
        KnowledgeRecord {
            id: "kn-project-brief".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: Some(DEFAULT_PROJECT_ID.into()),
            title: "Default project brief".into(),
            summary: "Project goals, runtime expectations, and delivery checkpoints.".into(),
            kind: "private".into(),
            scope: "project".into(),
            status: "reviewed".into(),
            visibility: "public".into(),
            owner_user_id: None,
            source_type: "run".into(),
            source_ref: "default-project".into(),
            updated_at: now,
        },
    ]
}

#[allow(dead_code)]
pub(super) fn default_agent_records() -> Vec<AgentRecord> {
    let now = timestamp_now();
    vec![
        AgentRecord {
            id: "agent-orchestrator".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: None,
            scope: "workspace".into(),
            owner_user_id: None,
            asset_role: default_agent_asset_role(),
            name: "Workspace Orchestrator".into(),
            avatar_path: None,
            avatar: None,
            personality: "System coordinator".into(),
            tags: vec!["workspace".into(), "orchestration".into()],
            prompt: "Coordinate work across the workspace and keep execution aligned.".into(),
            builtin_tool_keys: vec![],
            skill_ids: vec![],
            mcp_server_names: vec![],
            task_domains: normalize_task_domains(vec!["workspace".into(), "orchestration".into()]),
            manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
            default_model_strategy: default_model_strategy(),
            capability_policy: capability_policy_from_sources(&[], &[], &[]),
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
            description: "Coordinates projects, approvals, and execution policies.".into(),
            status: "active".into(),
            updated_at: now,
        },
        AgentRecord {
            id: "agent-project-delivery".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: Some(DEFAULT_PROJECT_ID.into()),
            scope: "project".into(),
            owner_user_id: None,
            asset_role: default_agent_asset_role(),
            name: "Project Delivery Agent".into(),
            avatar_path: None,
            avatar: None,
            personality: "Delivery lead".into(),
            tags: vec!["project".into(), "delivery".into()],
            prompt: "Track project work, runtime sessions, and follow-up actions.".into(),
            builtin_tool_keys: vec![],
            skill_ids: vec![],
            mcp_server_names: vec![],
            task_domains: normalize_task_domains(vec!["project".into(), "delivery".into()]),
            manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
            default_model_strategy: default_model_strategy(),
            capability_policy: capability_policy_from_sources(&[], &[], &[]),
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
            description: "Tracks project work, runtime sessions, and follow-up actions.".into(),
            status: "active".into(),
            updated_at: now,
        },
    ]
}

#[allow(dead_code)]
pub(super) fn default_team_records() -> Vec<TeamRecord> {
    let now = timestamp_now();
    vec![TeamRecord {
        id: "team-workspace-core".into(),
        workspace_id: DEFAULT_WORKSPACE_ID.into(),
        project_id: None,
        scope: "workspace".into(),
        name: "Workspace Core".into(),
        avatar_path: None,
        avatar: None,
        personality: "Governance team".into(),
        tags: vec!["workspace".into(), "governance".into()],
        prompt: "Maintain workspace-wide standards and governance.".into(),
        builtin_tool_keys: vec![],
        skill_ids: vec![],
        mcp_server_names: vec![],
        task_domains: normalize_task_domains(vec!["workspace".into(), "governance".into()]),
        manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
        default_model_strategy: default_model_strategy(),
        capability_policy: capability_policy_from_sources(&[], &[], &[]),
        permission_envelope: default_permission_envelope(),
        memory_policy: default_team_memory_policy(),
        delegation_policy: default_team_delegation_policy(),
        approval_preference: default_approval_preference(),
        output_contract: default_output_contract(),
        shared_capability_policy: default_team_shared_capability_policy(),
        leader_agent_id: Some("agent-orchestrator".into()),
        member_agent_ids: vec!["agent-orchestrator".into()],
        leader_ref: "agent-orchestrator".into(),
        member_refs: vec!["agent-orchestrator".into()],
        team_topology: team_topology_from_refs(
            Some("agent-orchestrator".into()),
            vec!["agent-orchestrator".into()],
        ),
        shared_memory_policy: default_shared_memory_policy(),
        mailbox_policy: default_mailbox_policy(),
        artifact_handoff_policy: default_artifact_handoff_policy(),
        workflow_affordance: workflow_affordance_from_task_domains(
            &normalize_task_domains(vec!["workspace".into(), "governance".into()]),
            true,
            true,
        ),
        worker_concurrency_limit: default_team_delegation_policy().max_worker_count,
        integration_source: None,
        trust_metadata: default_asset_trust_metadata(),
        dependency_resolution: Vec::new(),
        import_metadata: default_asset_import_metadata(),
        description: "Maintains workspace-wide operating standards and governance.".into(),
        status: "active".into(),
        updated_at: now,
    }]
}

pub(super) fn default_model_catalog() -> Vec<ModelCatalogRecord> {
    vec![
        ModelCatalogRecord {
            id: "claude-sonnet-4-5".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            label: "Claude Sonnet 4.5".into(),
            provider: "Anthropic".into(),
            description: "Balanced reasoning model for daily runtime turns.".into(),
            recommended_for: "Planning, coding, and reviews".into(),
            availability: "healthy".into(),
            default_permission: "auto".into(),
        },
        ModelCatalogRecord {
            id: "gpt-4o".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            label: "GPT-4o".into(),
            provider: "OpenAI".into(),
            description: "Fast multimodal model for general assistant work.".into(),
            recommended_for: "Conversation and lightweight execution".into(),
            availability: "configured".into(),
            default_permission: "auto".into(),
        },
    ]
}

pub(super) fn default_provider_credentials() -> Vec<ProviderCredentialRecord> {
    vec![
        ProviderCredentialRecord {
            id: "cred-anthropic".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            provider: "Anthropic".into(),
            name: "Anthropic Primary".into(),
            base_url: None,
            status: "healthy".into(),
        },
        ProviderCredentialRecord {
            id: "cred-openai".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            provider: "OpenAI".into(),
            name: "OpenAI Backup".into(),
            base_url: None,
            status: "unconfigured".into(),
        },
    ]
}

pub(super) fn default_tool_records() -> Vec<ToolRecord> {
    let now = timestamp_now();
    vec![
        ToolRecord {
            id: "tool-filesystem".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            kind: "builtin".into(),
            name: "Filesystem".into(),
            description: "Read and write files inside the workspace boundary.".into(),
            status: "active".into(),
            permission_mode: "ask".into(),
            updated_at: now,
        },
        ToolRecord {
            id: "tool-shell".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            kind: "builtin".into(),
            name: "Shell".into(),
            description: "Execute workspace commands with approval.".into(),
            status: "active".into(),
            permission_mode: "ask".into(),
            updated_at: now,
        },
    ]
}

#[allow(dead_code)]
pub(super) fn default_automation_records() -> Vec<AutomationRecord> {
    let now = timestamp_now();
    vec![AutomationRecord {
        id: "auto-daily-summary".into(),
        workspace_id: DEFAULT_WORKSPACE_ID.into(),
        project_id: Some(DEFAULT_PROJECT_ID.into()),
        title: "Daily summary".into(),
        description: "Summarize active runtime work for the default project.".into(),
        cadence: "Weekdays 09:30".into(),
        owner_type: "agent".into(),
        owner_id: "agent-project-delivery".into(),
        status: "active".into(),
        next_run_at: Some(now + 86_400_000),
        last_run_at: None,
        output: "Inbox summary".into(),
    }]
}

pub(super) fn avatar_data_url(paths: &WorkspacePaths, user: &StoredUser) -> Option<String> {
    let avatar_path = user.record.avatar_path.as_ref()?;
    let Some(content_type) = user.record.avatar_content_type.as_deref() else {
        return Some(avatar_path.clone());
    };
    let Ok(bytes) = fs::read(paths.root.join(avatar_path)) else {
        return Some(avatar_path.clone());
    };
    Some(format!(
        "data:{content_type};base64,{}",
        BASE64_STANDARD.encode(bytes)
    ))
}

pub(super) fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("hash-{:x}", hasher.finish())
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
}
