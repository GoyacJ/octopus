use super::*;
use crate::persistence::{
    backfill_default_project_assignments, backfill_project_governance,
    backfill_project_resource_directories, map_db_error,
};
use crate::project_tasks::{
    load_project_task_interventions, load_project_task_runs, load_project_task_scheduler_claims,
    load_project_tasks,
};
use octopus_core::ArtifactVersionReference;
use octopus_core::{BundleAssetDescriptorRecord, ProjectModelAssignments};
use octopus_persistence::Database;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub(super) const BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID: &str = "user-owner";
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
    #[serde(default)]
    pub(super) avatar_path: Option<String>,
    #[serde(default)]
    pub(super) avatar_content_type: Option<String>,
    pub(super) slug: String,
    pub(super) deployment: String,
    pub(super) bootstrap_status: String,
    pub(super) owner_user_id: Option<String>,
    pub(super) host: String,
    pub(super) listen_address: String,
    pub(super) default_project_id: String,
    #[serde(default)]
    pub(super) mapped_directory: Option<String>,
    #[serde(default)]
    pub(super) mapped_directory_default: Option<String>,
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
        tasks: "allow".into(),
    }
}

pub(super) fn default_project_permission_overrides() -> ProjectPermissionOverrides {
    ProjectPermissionOverrides {
        agents: "inherit".into(),
        resources: "inherit".into(),
        tools: "inherit".into(),
        knowledge: "inherit".into(),
        tasks: "inherit".into(),
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
    pub(super) database: Database,
    pub(super) workspace: Mutex<WorkspaceSummary>,
    pub(super) workspace_avatar_path: Mutex<Option<String>>,
    pub(super) workspace_avatar_content_type: Mutex<Option<String>>,
    pub(super) users: Mutex<Vec<StoredUser>>,
    pub(super) apps: Mutex<Vec<ClientAppRecord>>,
    pub(super) sessions: Mutex<Vec<SessionRecord>>,
    pub(super) projects: Mutex<Vec<ProjectRecord>>,
    pub(super) project_promotion_requests: Mutex<Vec<ProjectPromotionRequest>>,
    pub(super) project_deletion_requests: Mutex<Vec<ProjectDeletionRequest>>,
    pub(super) resources: Mutex<Vec<WorkspaceResourceRecord>>,
    pub(super) knowledge_records: Mutex<Vec<KnowledgeRecord>>,
    #[allow(dead_code)]
    pub(super) project_tasks: Mutex<Vec<ProjectTaskRecord>>,
    #[allow(dead_code)]
    pub(super) project_task_runs: Mutex<Vec<ProjectTaskRunRecord>>,
    #[allow(dead_code)]
    pub(super) project_task_interventions: Mutex<Vec<ProjectTaskInterventionRecord>>,
    #[allow(dead_code)]
    pub(super) project_task_scheduler_claims: Mutex<Vec<ProjectTaskSchedulerClaimRecord>>,
    pub(super) agents: Mutex<Vec<AgentRecord>>,
    pub(super) project_agent_links: Mutex<Vec<ProjectAgentLinkRecord>>,
    pub(super) teams: Mutex<Vec<TeamRecord>>,
    pub(super) project_team_links: Mutex<Vec<ProjectTeamLinkRecord>>,
    pub(super) model_catalog: Mutex<Vec<ModelCatalogRecord>>,
    pub(super) provider_credentials: Mutex<Vec<ProviderCredentialRecord>>,
    pub(super) tools: Mutex<Vec<ToolRecord>>,
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
        self.database.acquire().map_err(map_db_error)
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

pub(super) fn initialize_workspace_config(paths: &WorkspacePaths) -> Result<(), AppError> {
    if paths.workspace_config.exists() {
        return Ok(());
    }

    let config = WorkspaceConfigFile {
        id: DEFAULT_WORKSPACE_ID.into(),
        name: "Octopus Local Workspace".into(),
        avatar_path: None,
        avatar_content_type: None,
        slug: "local-workspace".into(),
        deployment: "local".into(),
        bootstrap_status: "setup_required".into(),
        owner_user_id: None,
        host: "127.0.0.1".into(),
        listen_address: "127.0.0.1".into(),
        default_project_id: DEFAULT_PROJECT_ID.into(),
        mapped_directory: None,
        mapped_directory_default: Some(workspace_root_display_path(paths)),
        project_default_permissions: ProjectDefaultPermissions {
            agents: "allow".into(),
            resources: "allow".into(),
            tools: "allow".into(),
            knowledge: "allow".into(),
            tasks: "allow".into(),
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
    let member_refs_json = json_string(&record.member_refs)?;
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };

    let sql = format!(
        "{verb} INTO teams (
            id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt,
            builtin_tool_keys, skill_ids, mcp_server_names, task_domains, manifest_revision,
            default_model_strategy_json, capability_policy_json, permission_envelope_json,
            memory_policy_json, delegation_policy_json, approval_preference_json,
            output_contract_json, shared_capability_policy_json, leader_ref, member_refs,
            team_topology_json, shared_memory_policy_json, mailbox_policy_json,
            artifact_handoff_policy_json, workflow_affordance_json, worker_concurrency_limit,
            integration_source_json, trust_metadata_json, dependency_resolution_json,
            import_metadata_json, description, status, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
            ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17,
            ?18, ?19, ?20,
            ?21, ?22, ?23, ?24,
            ?25, ?26, ?27,
            ?28, ?29, ?30,
            ?31, ?32, ?33,
            ?34, ?35, ?36, ?37
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

pub(super) fn load_state(
    paths: WorkspacePaths,
    database: Database,
) -> Result<InfraState, AppError> {
    let workspace_file: WorkspaceConfigFile =
        toml::from_str(&fs::read_to_string(&paths.workspace_config)?)?;
    let workspace_avatar_path = workspace_file.avatar_path.clone();
    let workspace_avatar_content_type = workspace_file.avatar_content_type.clone();
    let mut workspace = WorkspaceSummary {
        id: workspace_file.id,
        name: workspace_file.name,
        avatar: stored_avatar_data_url(
            &paths,
            workspace_file.avatar_path.as_deref(),
            workspace_file.avatar_content_type.as_deref(),
        ),
        slug: workspace_file.slug,
        deployment: workspace_file.deployment,
        bootstrap_status: workspace_file.bootstrap_status,
        owner_user_id: workspace_file.owner_user_id,
        host: workspace_file.host,
        listen_address: workspace_file.listen_address,
        default_project_id: workspace_file.default_project_id,
        mapped_directory: stored_mapped_directory(workspace_file.mapped_directory.as_deref()),
        mapped_directory_default: workspace_file
            .mapped_directory_default
            .filter(|value| !value.trim().is_empty())
            .or_else(|| Some(workspace_root_display_path(&paths))),
        project_default_permissions: workspace_file.project_default_permissions,
    };

    let app_registry: AppRegistryFile =
        toml::from_str(&fs::read_to_string(&paths.app_registry_config)?)?;
    let connection = database.acquire().map_err(map_db_error)?;
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
        bootstrap::save_workspace_config_file(
            &paths.workspace_config,
            &workspace,
            workspace_avatar_path.as_deref(),
            workspace_avatar_content_type.as_deref(),
        )?;
    }
    backfill_project_governance(&connection, workspace.owner_user_id.as_deref())?;
    for user in &users {
        ensure_personal_pet_for_user(&connection, &workspace.id, &user.record.id)?;
    }
    let projects = load_projects(&connection)?;
    let project_promotion_requests = load_project_promotion_requests(&connection)?;
    let project_deletion_requests = load_project_deletion_requests(&connection)?;
    let sessions = load_sessions(&connection)?;
    let resources = load_resources(&connection)?;
    let knowledge_records = load_knowledge_records(&connection)?;
    let project_tasks = load_project_tasks(&connection)?;
    let project_task_runs = load_project_task_runs(&connection)?;
    let project_task_interventions = load_project_task_interventions(&connection)?;
    let project_task_scheduler_claims = load_project_task_scheduler_claims(&connection)?;
    let artifacts = load_artifact_records(&connection)?;
    let agents = load_agents(&connection)?;
    let project_agent_links = load_project_agent_links(&connection)?;
    let teams = load_teams(&connection)?;
    let project_team_links = load_project_team_links(&connection)?;
    let model_catalog = load_model_catalog(&connection)?;
    let provider_credentials = load_provider_credentials(&connection)?;
    let tools = load_tools(&connection)?;
    let trace_events = load_trace_events(&connection)?;
    let audit_records = load_audit_records(&connection)?;
    let cost_entries = load_cost_entries(&connection)?;
    let pet_extensions = load_pet_agent_extensions(&connection)?;
    let pet_presences = load_pet_presences(&connection)?;
    let pet_bindings = load_pet_bindings(&connection)?;

    Ok(InfraState {
        paths,
        database,
        workspace: Mutex::new(workspace),
        workspace_avatar_path: Mutex::new(workspace_avatar_path),
        workspace_avatar_content_type: Mutex::new(workspace_avatar_content_type),
        users: Mutex::new(users),
        apps: Mutex::new(app_registry.apps),
        sessions: Mutex::new(sessions),
        projects: Mutex::new(projects),
        project_promotion_requests: Mutex::new(project_promotion_requests),
        project_deletion_requests: Mutex::new(project_deletion_requests),
        resources: Mutex::new(resources),
        knowledge_records: Mutex::new(knowledge_records),
        project_tasks: Mutex::new(project_tasks),
        project_task_runs: Mutex::new(project_task_runs),
        project_task_interventions: Mutex::new(project_task_interventions),
        project_task_scheduler_claims: Mutex::new(project_task_scheduler_claims),
        agents: Mutex::new(agents),
        project_agent_links: Mutex::new(project_agent_links),
        teams: Mutex::new(teams),
        project_team_links: Mutex::new(project_team_links),
        model_catalog: Mutex::new(model_catalog),
        provider_credentials: Mutex::new(provider_credentials),
        tools: Mutex::new(tools),
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

pub(super) fn load_projects(connection: &Connection) -> Result<Vec<ProjectRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, name, status, description, resource_directory, leader_agent_id, manager_user_id, preset_code, assignments_json, owner_user_id, member_user_ids_json, permission_overrides_json, linked_workspace_assets_json FROM projects",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let leader_agent_id: Option<String> = row.get(6)?;
            let manager_user_id: Option<String> = row.get(7)?;
            let preset_code: Option<String> = row.get(8)?;
            let assignments_json: Option<String> = row.get(9)?;
            let assignments = assignments_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<ProjectWorkspaceAssignments>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        9,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            let owner_user_id: Option<String> = row.get(10)?;
            let member_user_ids_json: Option<String> = row.get(11)?;
            let member_user_ids = member_user_ids_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<Vec<String>>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        11,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?
                .unwrap_or_default();
            let permission_overrides_json: Option<String> = row.get(12)?;
            let permission_overrides = permission_overrides_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<ProjectPermissionOverrides>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        12,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?
                .unwrap_or_else(default_project_permission_overrides);
            let linked_workspace_assets_json: Option<String> = row.get(13)?;
            let linked_workspace_assets = linked_workspace_assets_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<ProjectLinkedWorkspaceAssets>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        13,
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
                leader_agent_id,
                manager_user_id,
                preset_code,
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

pub(super) fn load_project_deletion_requests(
    connection: &Connection,
) -> Result<Vec<ProjectDeletionRequest>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, requested_by_user_id, status, reason, reviewed_by_user_id, review_comment, created_at, updated_at, reviewed_at
             FROM project_deletion_requests
             ORDER BY created_at DESC, id DESC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectDeletionRequest {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                requested_by_user_id: row.get(3)?,
                status: row.get(4)?,
                reason: row.get(5)?,
                reviewed_by_user_id: row.get(6)?,
                review_comment: row.get(7)?,
                created_at: row.get::<_, i64>(8)? as u64,
                updated_at: row.get::<_, i64>(9)? as u64,
                reviewed_at: row.get::<_, Option<i64>>(10)?.map(|value| value as u64),
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

pub(super) fn load_project_artifact_records(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<ArtifactRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, conversation_id, title, status, preview_kind,
                    latest_version, promotion_state, updated_at, content_type
             FROM artifact_records
             WHERE project_id = ?1
             ORDER BY updated_at DESC, id ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([project_id], |row| {
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
                output_contract_json, shared_capability_policy_json, leader_ref, member_refs,
                team_topology_json,
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
            let leader_ref: String = row.get(22)?;
            let member_refs_raw: String = row.get(23)?;
            let team_topology_raw: String = row.get(24)?;
            let shared_memory_policy_raw: String = row.get(25)?;
            let mailbox_policy_raw: String = row.get(26)?;
            let artifact_handoff_policy_raw: String = row.get(27)?;
            let workflow_affordance_raw: String = row.get(28)?;
            let integration_source_raw: Option<String> = row.get(30)?;
            let trust_metadata_raw: String = row.get(31)?;
            let dependency_resolution_raw: String = row.get(32)?;
            let import_metadata_raw: String = row.get(33)?;
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
                worker_concurrency_limit: row.get::<_, i64>(29)? as u64,
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
                description: row.get(34)?,
                status: row.get(35)?,
                updated_at: row.get::<_, i64>(36)? as u64,
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

pub(super) fn avatar_data_url(paths: &WorkspacePaths, user: &StoredUser) -> Option<String> {
    stored_avatar_data_url(
        paths,
        user.record.avatar_path.as_deref(),
        user.record.avatar_content_type.as_deref(),
    )
}

pub(super) fn stored_avatar_data_url(
    paths: &WorkspacePaths,
    avatar_path: Option<&str>,
    content_type: Option<&str>,
) -> Option<String> {
    let avatar_path = avatar_path?;
    let Some(content_type) = content_type else {
        return Some(avatar_path.to_string());
    };
    let Ok(bytes) = fs::read(paths.root.join(avatar_path)) else {
        return Some(avatar_path.to_string());
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

    #[test]
    fn initialize_database_creates_runtime_secret_records_table() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let database = open_workspace_database(&paths).expect("database handle");
        crate::initialize_database(&database).expect("database");

        let connection = database.acquire().expect("db");
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
        let database = open_workspace_database(&paths).expect("database handle");
        crate::initialize_database(&database).expect("database");
        crate::seed_defaults(&database, &paths).expect("seed defaults");

        let connection = database.acquire().expect("db");
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

        let database = open_workspace_database(&paths).expect("database handle");
        let state = load_state(paths, database).expect("load state");

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
