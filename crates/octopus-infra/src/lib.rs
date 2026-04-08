mod agent_import;

use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    fs,
    hash::{Hash, Hasher},
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use octopus_core::{
    timestamp_now, AgentRecord, AppError, ArtifactRecord, AuditRecord, AuthorizationDecision,
    AutomationRecord, AvatarUploadPayload, BindPetConversationInput,
    ChangeCurrentUserPasswordRequest, ChangeCurrentUserPasswordResponse, ClientAppRecord,
    CopyWorkspaceSkillToManagedInput, CostLedgerEntry, CreateProjectRequest,
    CreateWorkspaceResourceFolderInput, CreateWorkspaceResourceInput, CreateWorkspaceSkillInput,
    CreateWorkspaceUserRequest, ImportWorkspaceAgentBundleInput, ImportWorkspaceAgentBundlePreview,
    ImportWorkspaceAgentBundlePreviewInput, ImportWorkspaceAgentBundleResult,
    ImportWorkspaceSkillArchiveInput, ImportWorkspaceSkillFolderInput, InboxItemRecord,
    KnowledgeEntryRecord, KnowledgeRecord, LoginRequest, LoginResponse, MenuRecord,
    ModelCatalogRecord, PermissionRecord, PetConversationBinding, PetMessage, PetPosition,
    PetPresenceState, PetProfile, PetWorkspaceSnapshot, ProjectAgentLinkInput,
    ProjectAgentLinkRecord, ProjectRecord, ProjectTeamLinkInput, ProjectTeamLinkRecord,
    ProjectWorkspaceAssignments, ProviderCredentialRecord, RegisterWorkspaceOwnerRequest,
    RegisterWorkspaceOwnerResponse, RoleRecord, SavePetPresenceInput, SessionRecord,
    SystemBootstrapStatus, TeamRecord, ToolRecord, TraceEventRecord,
    UpdateCurrentUserProfileRequest, UpdateProjectRequest, UpdateWorkspaceResourceInput,
    UpdateWorkspaceSkillFileInput, UpdateWorkspaceSkillInput, UpdateWorkspaceUserRequest,
    UpsertAgentInput, UpsertTeamInput, UpsertWorkspaceMcpServerInput, UserRecord,
    UserRecordSummary, WorkspaceDirectoryUploadEntry, WorkspaceMcpServerDocument,
    WorkspaceMembershipRecord, WorkspaceResourceRecord, WorkspaceSkillDocument,
    WorkspaceSkillFileDocument, WorkspaceSkillTreeDocument, WorkspaceSkillTreeNode,
    WorkspaceSummary, WorkspaceToolCatalogEntry, WorkspaceToolCatalogSnapshot,
    WorkspaceToolDisablePatch, WorkspaceToolManagementCapabilities, DEFAULT_PROJECT_ID,
    DEFAULT_WORKSPACE_ID,
};
use octopus_platform::{
    AppRegistryService, ArtifactService, AuthService, InboxService, KnowledgeService,
    ObservationService, RbacService, WorkspaceService,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zip::ZipArchive;

#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    pub root: PathBuf,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub tmp_dir: PathBuf,
    pub workspace_config: PathBuf,
    pub app_registry_config: PathBuf,
    pub runtime_config_dir: PathBuf,
    pub runtime_project_config_dir: PathBuf,
    pub runtime_user_config_dir: PathBuf,
    pub db_path: PathBuf,
    pub blobs_dir: PathBuf,
    pub user_avatars_dir: PathBuf,
    pub artifacts_dir: PathBuf,
    pub knowledge_dir: PathBuf,
    pub inbox_dir: PathBuf,
    pub managed_skills_dir: PathBuf,
    pub runtime_sessions_dir: PathBuf,
    pub runtime_events_dir: PathBuf,
    pub runtime_traces_dir: PathBuf,
    pub runtime_approvals_dir: PathBuf,
    pub runtime_cache_dir: PathBuf,
    pub audit_log_dir: PathBuf,
    pub server_log_dir: PathBuf,
}

impl WorkspacePaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let config_dir = root.join("config");
        let data_dir = root.join("data");
        let runtime_dir = root.join("runtime");
        let logs_dir = root.join("logs");
        let tmp_dir = root.join("tmp");
        let runtime_config_dir = config_dir.join("runtime");
        let runtime_project_config_dir = runtime_config_dir.join("projects");
        let runtime_user_config_dir = runtime_config_dir.join("users");
        let blobs_dir = data_dir.join("blobs");
        let user_avatars_dir = blobs_dir.join("avatars");
        let artifacts_dir = data_dir.join("artifacts");
        let knowledge_dir = data_dir.join("knowledge");
        let inbox_dir = data_dir.join("inbox");
        let managed_skills_dir = data_dir.join("skills");
        let runtime_sessions_dir = runtime_dir.join("sessions");
        let runtime_events_dir = runtime_dir.join("events");
        let runtime_traces_dir = runtime_dir.join("traces");
        let runtime_approvals_dir = runtime_dir.join("approvals");
        let runtime_cache_dir = runtime_dir.join("cache");
        let audit_log_dir = logs_dir.join("audit");
        let server_log_dir = logs_dir.join("server");

        Self {
            workspace_config: config_dir.join("workspace.toml"),
            app_registry_config: config_dir.join("app-registry.toml"),
            runtime_config_dir,
            runtime_project_config_dir,
            runtime_user_config_dir,
            db_path: data_dir.join("main.db"),
            root,
            config_dir,
            data_dir,
            runtime_dir,
            logs_dir,
            tmp_dir,
            blobs_dir,
            user_avatars_dir,
            artifacts_dir,
            knowledge_dir,
            inbox_dir,
            managed_skills_dir,
            runtime_sessions_dir,
            runtime_events_dir,
            runtime_traces_dir,
            runtime_approvals_dir,
            runtime_cache_dir,
            audit_log_dir,
            server_log_dir,
        }
    }

    pub fn ensure_layout(&self) -> Result<(), AppError> {
        for path in [
            &self.root,
            &self.config_dir,
            &self.runtime_config_dir,
            &self.runtime_project_config_dir,
            &self.runtime_user_config_dir,
            &self.data_dir,
            &self.runtime_dir,
            &self.logs_dir,
            &self.tmp_dir,
            &self.blobs_dir,
            &self.user_avatars_dir,
            &self.artifacts_dir,
            &self.knowledge_dir,
            &self.inbox_dir,
            &self.managed_skills_dir,
            &self.runtime_sessions_dir,
            &self.runtime_events_dir,
            &self.runtime_traces_dir,
            &self.runtime_approvals_dir,
            &self.runtime_cache_dir,
            &self.audit_log_dir,
            &self.server_log_dir,
        ] {
            fs::create_dir_all(path)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceConfigFile {
    id: String,
    name: String,
    slug: String,
    deployment: String,
    bootstrap_status: String,
    owner_user_id: Option<String>,
    host: String,
    listen_address: String,
    default_project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppRegistryFile {
    apps: Vec<ClientAppRecord>,
}

#[derive(Debug, Clone)]
struct StoredUser {
    record: UserRecord,
    password_hash: String,
    membership: WorkspaceMembershipRecord,
}

#[derive(Debug)]
struct InfraState {
    paths: WorkspacePaths,
    workspace: Mutex<WorkspaceSummary>,
    users: Mutex<Vec<StoredUser>>,
    apps: Mutex<Vec<ClientAppRecord>>,
    sessions: Mutex<Vec<SessionRecord>>,
    projects: Mutex<Vec<ProjectRecord>>,
    resources: Mutex<Vec<WorkspaceResourceRecord>>,
    knowledge_records: Mutex<Vec<KnowledgeRecord>>,
    agents: Mutex<Vec<AgentRecord>>,
    project_agent_links: Mutex<Vec<ProjectAgentLinkRecord>>,
    teams: Mutex<Vec<TeamRecord>>,
    project_team_links: Mutex<Vec<ProjectTeamLinkRecord>>,
    model_catalog: Mutex<Vec<ModelCatalogRecord>>,
    provider_credentials: Mutex<Vec<ProviderCredentialRecord>>,
    tools: Mutex<Vec<ToolRecord>>,
    automations: Mutex<Vec<AutomationRecord>>,
    roles: Mutex<Vec<RoleRecord>>,
    permissions: Mutex<Vec<PermissionRecord>>,
    menus: Mutex<Vec<MenuRecord>>,
    artifacts: Mutex<Vec<ArtifactRecord>>,
    inbox: Mutex<Vec<InboxItemRecord>>,
    trace_events: Mutex<Vec<TraceEventRecord>>,
    audit_records: Mutex<Vec<AuditRecord>>,
    cost_entries: Mutex<Vec<CostLedgerEntry>>,
    workspace_pet_presence: Mutex<PetPresenceState>,
    project_pet_presences: Mutex<Vec<(String, PetPresenceState)>>,
    workspace_pet_binding: Mutex<Option<PetConversationBinding>>,
    project_pet_bindings: Mutex<Vec<(String, PetConversationBinding)>>,
}

impl InfraState {
    fn open_db(&self) -> Result<Connection, AppError> {
        Connection::open(&self.paths.db_path).map_err(|error| AppError::database(error.to_string()))
    }

    fn workspace_snapshot(&self) -> Result<WorkspaceSummary, AppError> {
        self.workspace
            .lock()
            .map_err(|_| AppError::runtime("workspace mutex poisoned"))
            .map(|workspace| workspace.clone())
    }

    fn workspace_id(&self) -> Result<String, AppError> {
        Ok(self.workspace_snapshot()?.id)
    }

    fn save_workspace_config(&self) -> Result<(), AppError> {
        let workspace = self.workspace_snapshot()?;
        save_workspace_config_file(&self.paths.workspace_config, &workspace)
    }
}

#[derive(Clone)]
pub struct InfraWorkspaceService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraAuthService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraAppRegistryService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraRbacService {
    _state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraArtifactService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraInboxService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraKnowledgeService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraObservationService {
    state: Arc<InfraState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SkillDefinitionSource {
    WorkspaceManaged,
    ProjectClaw,
    ProjectCodex,
    ProjectClaude,
    UserClawConfigHome,
    UserCodexHome,
    UserClaw,
    UserCodex,
    UserClaude,
}

impl SkillDefinitionSource {
    fn key(self) -> &'static str {
        match self {
            Self::WorkspaceManaged => "workspace-managed",
            Self::ProjectClaw => "project-claw",
            Self::ProjectCodex => "project-codex",
            Self::ProjectClaude => "project-claude",
            Self::UserClawConfigHome => "user-claw-config-home",
            Self::UserCodexHome => "user-codex-home",
            Self::UserClaw => "user-claw-home",
            Self::UserCodex => "user-codex-home-legacy",
            Self::UserClaude => "user-claude-home",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkillSourceOrigin {
    SkillsDir,
    LegacyCommandsDir,
}

impl SkillSourceOrigin {
    fn as_str(self) -> &'static str {
        match self {
            Self::SkillsDir => "skills_dir",
            Self::LegacyCommandsDir => "legacy_commands_dir",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SkillCatalogRoot {
    source: SkillDefinitionSource,
    path: PathBuf,
    origin: SkillSourceOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SkillCatalogEntry {
    name: String,
    description: Option<String>,
    source: SkillDefinitionSource,
    origin: SkillSourceOrigin,
    path: PathBuf,
    shadowed_by: Option<String>,
}

fn normalize_required_permission(permission: runtime::PermissionMode) -> Option<String> {
    match permission {
        runtime::PermissionMode::ReadOnly => Some("readonly".into()),
        runtime::PermissionMode::WorkspaceWrite => Some("workspace-write".into()),
        runtime::PermissionMode::DangerFullAccess => Some("danger-full-access".into()),
        runtime::PermissionMode::Prompt | runtime::PermissionMode::Allow => None,
    }
}

fn display_path(path: &Path, workspace_root: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(workspace_root) {
        return relative.to_string_lossy().replace('\\', "/");
    }

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        if let Ok(relative) = path.strip_prefix(&home) {
            let suffix = relative.to_string_lossy().replace('\\', "/");
            return if suffix.is_empty() {
                "~".into()
            } else {
                format!("~/{}", suffix)
            };
        }
    }

    path.to_string_lossy().replace('\\', "/")
}

fn discover_skill_roots(cwd: &Path) -> Vec<SkillCatalogRoot> {
    let mut roots = Vec::new();

    push_unique_skill_root(
        &mut roots,
        SkillDefinitionSource::WorkspaceManaged,
        cwd.join("data").join("skills"),
        SkillSourceOrigin::SkillsDir,
    );

    for ancestor in cwd.ancestors() {
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::ProjectClaw,
            ancestor.join(".claw").join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::ProjectCodex,
            ancestor.join(".codex").join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::ProjectClaude,
            ancestor.join(".claude").join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::ProjectClaw,
            ancestor.join(".claw").join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::ProjectCodex,
            ancestor.join(".codex").join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::ProjectClaude,
            ancestor.join(".claude").join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
    }

    if let Ok(claw_config_home) = env::var("CLAW_CONFIG_HOME") {
        let claw_config_home = PathBuf::from(claw_config_home);
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserClawConfigHome,
            claw_config_home.join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserClawConfigHome,
            claw_config_home.join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
    }

    if let Ok(codex_home) = env::var("CODEX_HOME") {
        let codex_home = PathBuf::from(codex_home);
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserCodexHome,
            codex_home.join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserCodexHome,
            codex_home.join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
    }

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserClaw,
            home.join(".claw").join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserClaw,
            home.join(".claw").join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserCodex,
            home.join(".codex").join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserCodex,
            home.join(".codex").join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserClaude,
            home.join(".claude").join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserClaude,
            home.join(".claude").join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
    }

    roots
}

fn push_unique_skill_root(
    roots: &mut Vec<SkillCatalogRoot>,
    source: SkillDefinitionSource,
    path: PathBuf,
    origin: SkillSourceOrigin,
) {
    if path.is_dir() && !roots.iter().any(|existing| existing.path == path) {
        roots.push(SkillCatalogRoot {
            source,
            path,
            origin,
        });
    }
}

fn load_skills_from_roots(roots: &[SkillCatalogRoot]) -> Result<Vec<SkillCatalogEntry>, AppError> {
    let mut skills = Vec::new();
    let mut active_sources = BTreeMap::<String, String>::new();

    for root in roots {
        let mut root_skills = Vec::new();
        for entry in fs::read_dir(&root.path)? {
            let entry = entry?;
            match root.origin {
                SkillSourceOrigin::SkillsDir => {
                    if !entry.path().is_dir() {
                        continue;
                    }
                    let skill_path = entry.path().join("SKILL.md");
                    if !skill_path.is_file() {
                        continue;
                    }
                    let contents = fs::read_to_string(&skill_path)?;
                    let (name, description) = parse_skill_frontmatter(&contents);
                    root_skills.push(SkillCatalogEntry {
                        name: name
                            .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string()),
                        description,
                        source: root.source,
                        origin: root.origin,
                        path: skill_path,
                        shadowed_by: None,
                    });
                }
                SkillSourceOrigin::LegacyCommandsDir => {
                    let path = entry.path();
                    let markdown_path = if path.is_dir() {
                        let skill_path = path.join("SKILL.md");
                        if !skill_path.is_file() {
                            continue;
                        }
                        skill_path
                    } else if path
                        .extension()
                        .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("md"))
                    {
                        path
                    } else {
                        continue;
                    };

                    let contents = fs::read_to_string(&markdown_path)?;
                    let fallback_name = markdown_path.file_stem().map_or_else(
                        || entry.file_name().to_string_lossy().to_string(),
                        |stem| stem.to_string_lossy().to_string(),
                    );
                    let (name, description) = parse_skill_frontmatter(&contents);
                    root_skills.push(SkillCatalogEntry {
                        name: name.unwrap_or(fallback_name),
                        description,
                        source: root.source,
                        origin: root.origin,
                        path: markdown_path,
                        shadowed_by: None,
                    });
                }
            }
        }

        root_skills.sort_by(|left, right| left.name.cmp(&right.name));
        for mut skill in root_skills {
            let key = skill.name.to_ascii_lowercase();
            if let Some(existing) = active_sources.get(&key) {
                skill.shadowed_by = Some(existing.clone());
            } else {
                active_sources.insert(key, skill.source.key().into());
            }
            skills.push(skill);
        }
    }

    Ok(skills)
}

fn parse_skill_frontmatter(contents: &str) -> (Option<String>, Option<String>) {
    let mut lines = contents.lines();
    if lines.next().map(str::trim) != Some("---") {
        return (None, None);
    }

    let mut name = None;
    let mut description = None;
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("name:") {
            let value = unquote_frontmatter_value(value.trim());
            if !value.is_empty() {
                name = Some(value);
            }
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("description:") {
            let value = unquote_frontmatter_value(value.trim());
            if !value.is_empty() {
                description = Some(value);
            }
        }
    }

    (name, description)
}

fn unquote_frontmatter_value(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|trimmed| trimmed.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|trimmed| trimmed.strip_suffix('\''))
        })
        .unwrap_or(value)
        .trim()
        .to_string()
}

fn load_workspace_runtime_config(
    paths: &WorkspacePaths,
) -> Result<runtime::RuntimeConfig, AppError> {
    let workspace_config_path = paths.runtime_config_dir.join("workspace.json");
    let exists = workspace_config_path.exists();
    let document = if exists {
        let contents = fs::read_to_string(&workspace_config_path)?;
        if contents.trim().is_empty() {
            Some(BTreeMap::new())
        } else {
            let parsed = runtime::JsonValue::parse(&contents)
                .map_err(|error| AppError::runtime(format!("invalid runtime config: {error}")))?;
            let object = parsed.as_object().ok_or_else(|| {
                AppError::invalid_input(format!(
                    "{} must contain a top-level JSON object",
                    workspace_config_path.display()
                ))
            })?;
            Some(object.clone())
        }
    } else {
        None
    };

    runtime::ConfigLoader::new(&paths.root, &paths.runtime_config_dir)
        .load_from_documents(&[runtime::ConfigDocument {
            source: runtime::ConfigSource::Project,
            path: workspace_config_path,
            exists,
            loaded: document.is_some(),
            document,
        }])
        .map_err(|error| AppError::runtime(error.to_string()))
}

fn load_workspace_runtime_document(
    paths: &WorkspacePaths,
) -> Result<serde_json::Map<String, serde_json::Value>, AppError> {
    let workspace_config_path = paths.runtime_config_dir.join("workspace.json");
    match fs::read_to_string(&workspace_config_path) {
        Ok(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(serde_json::Map::new());
            }
            let parsed: serde_json::Value = serde_json::from_str(trimmed)?;
            parsed.as_object().cloned().ok_or_else(|| {
                AppError::invalid_input("workspace runtime config must be a JSON object")
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::Map::new()),
        Err(error) => Err(error.into()),
    }
}

fn validate_workspace_runtime_document(
    paths: &WorkspacePaths,
    document: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), AppError> {
    let rendered = serde_json::to_string(&serde_json::Value::Object(document.clone()))?;
    let parsed = runtime::JsonValue::parse(&rendered)
        .map_err(|error| AppError::invalid_input(format!("invalid runtime config: {error}")))?;
    let object = parsed
        .as_object()
        .cloned()
        .ok_or_else(|| AppError::invalid_input("workspace runtime config must be a JSON object"))?;
    runtime::ConfigLoader::new(&paths.root, &paths.runtime_config_dir)
        .load_from_documents(&[runtime::ConfigDocument {
            source: runtime::ConfigSource::Project,
            path: paths.runtime_config_dir.join("workspace.json"),
            exists: true,
            loaded: true,
            document: Some(object),
        }])
        .map_err(|error| AppError::invalid_input(error.to_string()))?;
    Ok(())
}

fn write_workspace_runtime_document(
    paths: &WorkspacePaths,
    document: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), AppError> {
    fs::create_dir_all(&paths.runtime_config_dir)?;
    let rendered = serde_json::to_vec_pretty(&serde_json::Value::Object(document.clone()))?;
    fs::write(paths.runtime_config_dir.join("workspace.json"), rendered)?;
    Ok(())
}

fn workspace_relative_path(path: &Path, workspace_root: &Path) -> Option<String> {
    path.strip_prefix(workspace_root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

fn catalog_hash_id(prefix: &str, value: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{prefix}-{:x}", hasher.finish())
}

fn skill_source_key(path: &Path, workspace_root: &Path) -> String {
    format!("skill:{}", display_path(path, workspace_root))
}

fn disabled_source_keys(
    document: &serde_json::Map<String, serde_json::Value>,
) -> std::collections::BTreeSet<String> {
    document
        .get("toolCatalog")
        .and_then(|value| value.as_object())
        .and_then(|tool_catalog| tool_catalog.get("disabledSourceKeys"))
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn set_disabled_source_keys(
    document: &mut serde_json::Map<String, serde_json::Value>,
    source_keys: &std::collections::BTreeSet<String>,
) -> Result<(), AppError> {
    let tool_catalog = document
        .entry("toolCatalog")
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| AppError::invalid_input("toolCatalog must be a JSON object"))?;
    tool_catalog.insert(
        "disabledSourceKeys".into(),
        serde_json::Value::Array(
            source_keys
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        ),
    );
    Ok(())
}

fn validate_skill_slug(slug: &str) -> Result<String, AppError> {
    let slug = slug.trim();
    if slug.is_empty() {
        return Err(AppError::invalid_input("skill slug is required"));
    }
    if slug.contains('/') || slug.contains('\\') || slug.contains("..") {
        return Err(AppError::invalid_input(
            "skill slug must be a safe relative directory name",
        ));
    }
    if !slug
        .chars()
        .all(|char| char.is_ascii_alphanumeric() || matches!(char, '-' | '_' | '.'))
    {
        return Err(AppError::invalid_input(
            "skill slug may only contain letters, numbers, hyphen, underscore, or dot",
        ));
    }
    Ok(slug.to_string())
}

fn workspace_owned_skill_root(paths: &WorkspacePaths) -> PathBuf {
    paths.managed_skills_dir.clone()
}

fn is_workspace_owned_skill(relative_path: Option<&str>, origin: SkillSourceOrigin) -> bool {
    relative_path.is_some_and(|value| value.starts_with("data/skills/"))
        && origin == SkillSourceOrigin::SkillsDir
}

fn skill_root_path(path: &Path, source_origin: SkillSourceOrigin) -> Result<PathBuf, AppError> {
    match source_origin {
        SkillSourceOrigin::SkillsDir => path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| AppError::invalid_input("workspace skill path is invalid")),
        SkillSourceOrigin::LegacyCommandsDir => Ok(path.to_path_buf()),
    }
}

fn build_skill_tree_node(path: &Path, root: &Path) -> Result<WorkspaceSkillTreeNode, AppError> {
    let metadata = fs::metadata(path)?;
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("."));
    let relative = path
        .strip_prefix(root)
        .ok()
        .map(|value| value.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| name.clone());

    if metadata.is_dir() {
        let mut children = fs::read_dir(path)?
            .map(|entry| entry.map(|item| item.path()))
            .collect::<Result<Vec<_>, _>>()?;
        children.sort_by(|left, right| {
            let left_is_dir = left.is_dir();
            let right_is_dir = right.is_dir();
            right_is_dir
                .cmp(&left_is_dir)
                .then_with(|| left.file_name().cmp(&right.file_name()))
        });
        return Ok(WorkspaceSkillTreeNode {
            path: relative,
            name,
            kind: "directory".into(),
            children: Some(
                children
                    .iter()
                    .map(|child| build_skill_tree_node(child, root))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            byte_size: None,
            is_text: None,
        });
    }

    let bytes = fs::read(path)?;
    let is_text = std::str::from_utf8(&bytes).is_ok() && !bytes.contains(&0);
    Ok(WorkspaceSkillTreeNode {
        path: relative,
        name,
        kind: "file".into(),
        children: None,
        byte_size: Some(metadata.len()),
        is_text: Some(is_text),
    })
}

fn build_skill_tree(
    root: &Path,
    source_origin: SkillSourceOrigin,
) -> Result<Vec<WorkspaceSkillTreeNode>, AppError> {
    match source_origin {
        SkillSourceOrigin::SkillsDir => {
            let mut children = fs::read_dir(root)?
                .map(|entry| entry.map(|item| item.path()))
                .collect::<Result<Vec<_>, _>>()?;
            children.sort_by(|left, right| {
                let left_is_dir = left.is_dir();
                let right_is_dir = right.is_dir();
                right_is_dir
                    .cmp(&left_is_dir)
                    .then_with(|| left.file_name().cmp(&right.file_name()))
            });
            children
                .iter()
                .map(|child| build_skill_tree_node(child, root))
                .collect()
        }
        SkillSourceOrigin::LegacyCommandsDir => Ok(vec![build_skill_tree_node(root, root)?]),
    }
}

fn collect_tree_files(
    skill_root: &Path,
    node: &WorkspaceSkillTreeNode,
    collected: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), AppError> {
    if node.kind == "directory" {
        for child in node.children.as_deref().unwrap_or(&[]) {
            collect_tree_files(skill_root, child, collected)?;
        }
        return Ok(());
    }

    let path = skill_root.join(&node.path);
    collected.push((node.path.clone(), fs::read(path)?));
    Ok(())
}

fn content_type_for_skill_file(path: &Path, is_text: bool) -> Option<String> {
    match path
        .extension()
        .and_then(OsStr::to_str)
        .map(|value| value.to_ascii_lowercase())
    {
        Some(extension) if extension == "md" => Some("text/markdown".into()),
        Some(extension) if matches!(extension.as_str(), "json") => Some("application/json".into()),
        Some(extension) if matches!(extension.as_str(), "yaml" | "yml") => {
            Some("application/yaml".into())
        }
        Some(extension)
            if matches!(
                extension.as_str(),
                "ts" | "tsx" | "js" | "jsx" | "rs" | "py" | "toml" | "txt" | "css" | "html" | "vue"
            ) =>
        {
            Some("text/plain".into())
        }
        _ if is_text => Some("text/plain".into()),
        _ => None,
    }
}

fn language_for_skill_file(path: &Path) -> Option<String> {
    match path
        .extension()
        .and_then(OsStr::to_str)
        .map(|value| value.to_ascii_lowercase())
    {
        Some(extension) if extension == "md" => Some("markdown".into()),
        Some(extension) if extension == "json" => Some("json".into()),
        Some(extension) if matches!(extension.as_str(), "yaml" | "yml") => Some("yaml".into()),
        Some(extension) if extension == "rs" => Some("rust".into()),
        Some(extension) if matches!(extension.as_str(), "ts" | "tsx") => Some("typescript".into()),
        Some(extension) if matches!(extension.as_str(), "js" | "jsx") => Some("javascript".into()),
        Some(extension) if extension == "py" => Some("python".into()),
        Some(extension) if extension == "vue" => Some("vue".into()),
        Some(extension) if extension == "toml" => Some("toml".into()),
        _ => None,
    }
}

fn validate_skill_file_relative_path(relative_path: &str) -> Result<String, AppError> {
    let trimmed = relative_path.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_input("skill file path is required"));
    }
    let path = Path::new(trimmed);
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(AppError::invalid_input(
                    "skill file path must stay within the skill root",
                ));
            }
        }
    }
    Ok(trimmed.replace('\\', "/"))
}

fn resolve_skill_file_path(
    skill_root: &Path,
    source_origin: SkillSourceOrigin,
    relative_path: &str,
) -> Result<PathBuf, AppError> {
    let relative_path = validate_skill_file_relative_path(relative_path)?;
    match source_origin {
        SkillSourceOrigin::SkillsDir => Ok(skill_root.join(relative_path)),
        SkillSourceOrigin::LegacyCommandsDir => {
            if relative_path
                != skill_root
                    .file_name()
                    .map(|value| value.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default()
            {
                return Err(AppError::not_found("workspace skill file"));
            }
            Ok(skill_root.to_path_buf())
        }
    }
}

fn skill_file_document_from_path(
    workspace_root: &Path,
    skill_id: &str,
    source_key: &str,
    skill_root: &Path,
    source_origin: SkillSourceOrigin,
    path: &Path,
    readonly: bool,
) -> Result<WorkspaceSkillFileDocument, AppError> {
    let metadata = fs::metadata(path)?;
    let bytes = fs::read(path)?;
    let is_text = std::str::from_utf8(&bytes).is_ok() && !bytes.contains(&0);
    let content = if is_text {
        Some(String::from_utf8(bytes).map_err(|error| AppError::invalid_input(error.to_string()))?)
    } else {
        None
    };
    let relative_path = match source_origin {
        SkillSourceOrigin::SkillsDir => path
            .strip_prefix(skill_root)
            .ok()
            .map(|value| value.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            }),
        SkillSourceOrigin::LegacyCommandsDir => path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    };

    Ok(WorkspaceSkillFileDocument {
        skill_id: skill_id.into(),
        source_key: source_key.into(),
        path: relative_path,
        display_path: display_path(path, workspace_root),
        byte_size: metadata.len(),
        is_text,
        content,
        content_type: content_type_for_skill_file(path, is_text),
        language: language_for_skill_file(path),
        readonly,
    })
}

fn skill_tree_document_from_path(
    workspace_root: &Path,
    skill_id: &str,
    source_key: &str,
    path: &Path,
    source_origin: SkillSourceOrigin,
) -> Result<WorkspaceSkillTreeDocument, AppError> {
    let root = skill_root_path(path, source_origin)?;
    let root_display_path = display_path(&root, workspace_root);
    Ok(WorkspaceSkillTreeDocument {
        skill_id: skill_id.into(),
        source_key: source_key.into(),
        display_path: display_path(path, workspace_root),
        root_path: root_display_path,
        tree: build_skill_tree(&root, source_origin)?,
    })
}

fn write_skill_tree_files(skill_dir: &Path, files: &[(String, Vec<u8>)]) -> Result<(), AppError> {
    for (relative_path, bytes) in files {
        let target_path = skill_dir.join(relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, bytes)?;
    }
    Ok(())
}

fn normalize_uploaded_files(
    files: &[WorkspaceDirectoryUploadEntry],
) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    if files.is_empty() {
        return Err(AppError::invalid_input("skill import files are required"));
    }
    let mut normalized = Vec::new();
    for file in files {
        let relative_path = validate_skill_file_relative_path(&file.relative_path)?;
        let bytes = BASE64_STANDARD
            .decode(&file.data_base64)
            .map_err(|error| AppError::invalid_input(format!("invalid file data: {error}")))?;
        normalized.push((relative_path, bytes));
    }

    let has_root_skill = normalized.iter().any(|(path, _)| path == "SKILL.md");
    if has_root_skill {
        return Ok(normalized);
    }

    let prefixes = normalized
        .iter()
        .filter_map(|(path, _)| path.split('/').next().map(ToOwned::to_owned))
        .collect::<std::collections::BTreeSet<_>>();
    if prefixes.len() != 1 {
        return Err(AppError::invalid_input(
            "skill import must contain SKILL.md at the root or under a single top-level directory",
        ));
    }
    let prefix = prefixes.iter().next().cloned().unwrap_or_default();
    let stripped = normalized
        .into_iter()
        .map(|(path, bytes)| {
            let stripped = path
                .strip_prefix(&format!("{prefix}/"))
                .ok_or_else(|| AppError::invalid_input("invalid imported file path"))?;
            Ok((stripped.to_string(), bytes))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    if !stripped.iter().any(|(path, _)| path == "SKILL.md") {
        return Err(AppError::invalid_input(
            "imported skill must contain SKILL.md at the root",
        ));
    }
    Ok(stripped)
}

fn extract_archive_entries(
    archive: &ImportWorkspaceSkillArchiveInput,
) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    let bytes = BASE64_STANDARD
        .decode(&archive.archive.data_base64)
        .map_err(|error| AppError::invalid_input(format!("invalid archive data: {error}")))?;
    let cursor = Cursor::new(bytes);
    let mut zip = ZipArchive::new(cursor)
        .map_err(|error: zip::result::ZipError| AppError::invalid_input(error.to_string()))?;
    let mut files = Vec::new();
    for index in 0..zip.len() {
        let mut file = zip
            .by_index(index)
            .map_err(|error: zip::result::ZipError| AppError::invalid_input(error.to_string()))?;
        if file.is_dir() {
            continue;
        }
        let relative_path = validate_skill_file_relative_path(file.name())?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        files.push((relative_path, bytes));
    }
    normalize_archive_entries(&files)
}

fn normalize_archive_entries(
    files: &[(String, Vec<u8>)],
) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    let entries = files
        .iter()
        .map(|(path, bytes)| WorkspaceDirectoryUploadEntry {
            relative_path: path.clone(),
            file_name: Path::new(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            content_type: String::new(),
            data_base64: BASE64_STANDARD.encode(bytes),
            byte_size: bytes.len() as u64,
        })
        .collect::<Vec<_>>();
    normalize_uploaded_files(&entries)
}

fn skill_document_from_path(
    workspace_root: &Path,
    path: &Path,
    source_origin: SkillSourceOrigin,
) -> Result<WorkspaceSkillDocument, AppError> {
    let content = fs::read_to_string(path)?;
    let (name, description) = parse_skill_frontmatter(&content);
    let display_path_value = display_path(path, workspace_root);
    let relative_path = workspace_relative_path(path, workspace_root);
    let workspace_owned = is_workspace_owned_skill(relative_path.as_deref(), source_origin);
    let root = skill_root_path(path, source_origin)?;

    Ok(WorkspaceSkillDocument {
        id: catalog_hash_id("skill", &display_path_value),
        source_key: skill_source_key(path, workspace_root),
        name: name.unwrap_or_else(|| {
            path.parent()
                .and_then(|parent| parent.file_name())
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| display_path_value.clone())
        }),
        description: description.unwrap_or_default(),
        content,
        display_path: display_path_value,
        root_path: display_path(&root, workspace_root),
        tree: build_skill_tree(&root, source_origin)?,
        source_origin: source_origin.as_str().into(),
        workspace_owned,
        relative_path,
    })
}

fn rewrite_skill_frontmatter_name(path: &Path, skill_name: &str) -> Result<(), AppError> {
    let existing = fs::read_to_string(path)?;
    let ends_with_newline = existing.ends_with('\n');
    let mut lines = existing.lines().map(str::to_string).collect::<Vec<_>>();
    if lines.first().map(|line| line.trim()) != Some("---") {
        return Ok(());
    }

    let mut closing_index = None;
    let mut name_index = None;
    for (index, line) in lines.iter().enumerate().skip(1) {
        let trimmed = line.trim();
        if trimmed == "---" {
            closing_index = Some(index);
            break;
        }
        if trimmed.starts_with("name:") {
            name_index = Some(index);
        }
    }

    let Some(closing_index) = closing_index else {
        return Ok(());
    };

    let normalized_name = format!("name: {skill_name}");
    if let Some(index) = name_index {
        if lines[index] == normalized_name {
            return Ok(());
        }
        lines[index] = normalized_name;
    } else {
        lines.insert(closing_index, normalized_name);
    }

    let mut updated = lines.join("\n");
    if ends_with_newline {
        updated.push('\n');
    }
    fs::write(path, updated)?;
    Ok(())
}

fn ensure_object_value<'a>(
    value: &'a mut serde_json::Value,
    field_name: &str,
) -> Result<&'a mut serde_json::Map<String, serde_json::Value>, AppError> {
    value
        .as_object_mut()
        .ok_or_else(|| AppError::invalid_input(format!("{field_name} must be a JSON object")))
}

fn ensure_top_level_object<'a>(
    document: &'a mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<&'a mut serde_json::Map<String, serde_json::Value>, AppError> {
    let value = document
        .entry(key)
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    ensure_object_value(value, key)
}

fn mcp_scope_label(_scope: runtime::ConfigSource) -> &'static str {
    "workspace"
}

fn mcp_endpoint(config: &runtime::McpServerConfig) -> String {
    match config {
        runtime::McpServerConfig::Stdio(config) => {
            if config.args.is_empty() {
                format!("stdio: {}", config.command)
            } else {
                format!("stdio: {} {}", config.command, config.args.join(" "))
            }
        }
        runtime::McpServerConfig::Sse(config) | runtime::McpServerConfig::Http(config) => {
            config.url.clone()
        }
        runtime::McpServerConfig::Ws(config) => config.url.clone(),
        runtime::McpServerConfig::Sdk(config) => format!("sdk: {}", config.name),
        runtime::McpServerConfig::ManagedProxy(config) => config.url.clone(),
    }
}

impl InfraWorkspaceService {
    fn find_skill_catalog_entry(&self, skill_id: &str) -> Result<SkillCatalogEntry, AppError> {
        let workspace_root = self.state.paths.root.clone();
        load_skills_from_roots(&discover_skill_roots(&workspace_root))?
            .into_iter()
            .find(|skill| {
                catalog_hash_id("skill", &display_path(&skill.path, &workspace_root)) == skill_id
            })
            .ok_or_else(|| AppError::not_found("workspace skill"))
    }

    fn get_workspace_skill_document(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let entry = self.find_skill_catalog_entry(skill_id)?;
        skill_document_from_path(&self.state.paths.root, &entry.path, entry.origin)
    }

    fn get_workspace_skill_tree_document(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillTreeDocument, AppError> {
        let entry = self.find_skill_catalog_entry(skill_id)?;
        skill_tree_document_from_path(
            &self.state.paths.root,
            skill_id,
            &skill_source_key(&entry.path, &self.state.paths.root),
            &entry.path,
            entry.origin,
        )
    }

    fn get_workspace_skill_file_document(
        &self,
        skill_id: &str,
        relative_path: &str,
    ) -> Result<WorkspaceSkillFileDocument, AppError> {
        let entry = self.find_skill_catalog_entry(skill_id)?;
        let source_key = skill_source_key(&entry.path, &self.state.paths.root);
        let relative = workspace_relative_path(&entry.path, &self.state.paths.root);
        let readonly = !is_workspace_owned_skill(relative.as_deref(), entry.origin);
        let skill_root = skill_root_path(&entry.path, entry.origin)?;
        let path = resolve_skill_file_path(&skill_root, entry.origin, relative_path)?;
        if !path.exists() {
            return Err(AppError::not_found("workspace skill file"));
        }
        skill_file_document_from_path(
            &self.state.paths.root,
            skill_id,
            &source_key,
            &skill_root,
            entry.origin,
            &path,
            readonly,
        )
    }

    fn ensure_workspace_owned_skill_entry(
        &self,
        skill_id: &str,
    ) -> Result<SkillCatalogEntry, AppError> {
        let entry = self.find_skill_catalog_entry(skill_id)?;
        let relative = workspace_relative_path(&entry.path, &self.state.paths.root);
        if !is_workspace_owned_skill(relative.as_deref(), entry.origin) {
            return Err(AppError::invalid_input(
                "only workspace-owned managed skills can be edited or deleted",
            ));
        }
        Ok(entry)
    }

    fn import_skill_files_to_managed_root(
        &self,
        slug: &str,
        files: Vec<(String, Vec<u8>)>,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let slug = validate_skill_slug(slug)?;
        let skill_dir = workspace_owned_skill_root(&self.state.paths).join(&slug);
        if skill_dir.exists() {
            return Err(AppError::conflict(format!(
                "workspace skill '{slug}' already exists"
            )));
        }
        if !files
            .iter()
            .any(|(relative_path, _)| relative_path == "SKILL.md")
        {
            return Err(AppError::invalid_input(
                "imported skill must contain SKILL.md at the root",
            ));
        }
        fs::create_dir_all(&skill_dir)?;
        write_skill_tree_files(&skill_dir, &files)?;
        let skill_path = skill_dir.join("SKILL.md");
        rewrite_skill_frontmatter_name(&skill_path, &slug)?;
        skill_document_from_path(
            &self.state.paths.root,
            &skill_path,
            SkillSourceOrigin::SkillsDir,
        )
    }

    fn load_mcp_server_document(
        &self,
        server_name: &str,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        let document = load_workspace_runtime_document(&self.state.paths)?;
        let config = document
            .get("mcpServers")
            .and_then(|value| value.as_object())
            .and_then(|servers| servers.get(server_name))
            .cloned()
            .ok_or_else(|| AppError::not_found("workspace mcp server"))?;
        let config = config
            .as_object()
            .cloned()
            .ok_or_else(|| AppError::invalid_input("mcp server config must be a JSON object"))?;

        Ok(WorkspaceMcpServerDocument {
            server_name: server_name.into(),
            source_key: format!("mcp:{server_name}"),
            display_path: "config/runtime/workspace.json".into(),
            scope: "workspace".into(),
            config: serde_json::Value::Object(config),
        })
    }

    fn save_workspace_runtime_document(
        &self,
        document: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), AppError> {
        validate_workspace_runtime_document(&self.state.paths, &document)?;
        write_workspace_runtime_document(&self.state.paths, &document)
    }

    async fn build_tool_catalog(&self) -> Result<WorkspaceToolCatalogSnapshot, AppError> {
        let workspace_id = self.state.workspace_id()?;
        let workspace_root = self.state.paths.root.clone();
        let runtime_document = load_workspace_runtime_document(&self.state.paths)?;
        let disabled_keys = disabled_source_keys(&runtime_document);
        let mut entries = Vec::new();

        for spec in tools::mvp_tool_specs() {
            let source_key = format!("builtin:{}", spec.name);
            entries.push(WorkspaceToolCatalogEntry {
                id: format!("builtin-{}", spec.name),
                workspace_id: workspace_id.clone(),
                name: spec.name.into(),
                kind: "builtin".into(),
                description: spec.description.into(),
                required_permission: normalize_required_permission(spec.required_permission),
                availability: "healthy".into(),
                source_key: source_key.clone(),
                display_path: "runtime builtin registry".into(),
                disabled: disabled_keys.contains(&source_key),
                management: WorkspaceToolManagementCapabilities {
                    can_disable: true,
                    can_edit: false,
                    can_delete: false,
                },
                builtin_key: Some(spec.name.into()),
                active: None,
                shadowed_by: None,
                source_origin: None,
                workspace_owned: None,
                relative_path: None,
                server_name: None,
                endpoint: None,
                tool_names: None,
                status_detail: None,
                scope: None,
            });
        }

        for skill in load_skills_from_roots(&discover_skill_roots(&workspace_root))? {
            let is_active = skill.shadowed_by.is_none();
            let source_key = skill_source_key(&skill.path, &workspace_root);
            let relative_path = workspace_relative_path(&skill.path, &workspace_root);
            let workspace_owned = is_workspace_owned_skill(relative_path.as_deref(), skill.origin);
            entries.push(WorkspaceToolCatalogEntry {
                id: catalog_hash_id("skill", &display_path(&skill.path, &workspace_root)),
                workspace_id: workspace_id.clone(),
                name: skill.name.clone(),
                kind: "skill".into(),
                description: skill.description.unwrap_or_default(),
                required_permission: None,
                availability: if is_active {
                    "healthy".into()
                } else {
                    "configured".into()
                },
                source_key: source_key.clone(),
                display_path: display_path(&skill.path, &workspace_root),
                disabled: disabled_keys.contains(&source_key),
                management: WorkspaceToolManagementCapabilities {
                    can_disable: true,
                    can_edit: workspace_owned,
                    can_delete: workspace_owned,
                },
                builtin_key: None,
                active: Some(is_active),
                shadowed_by: skill.shadowed_by.clone(),
                source_origin: Some(skill.origin.as_str().into()),
                workspace_owned: Some(workspace_owned),
                relative_path,
                server_name: None,
                endpoint: None,
                tool_names: None,
                status_detail: None,
                scope: None,
            });
        }

        let runtime_config = load_workspace_runtime_config(&self.state.paths)?;
        let mut manager = runtime::McpServerManager::from_runtime_config(&runtime_config);
        let discovery_report = manager.discover_tools_best_effort().await;
        let discovered_tool_names = discovery_report.tools.iter().fold(
            BTreeMap::<String, Vec<String>>::new(),
            |mut grouped, tool| {
                grouped
                    .entry(tool.server_name.clone())
                    .or_default()
                    .push(tool.qualified_name.clone());
                grouped
            },
        );
        let failed_servers = discovery_report
            .failed_servers
            .iter()
            .map(|failure| (failure.server_name.clone(), failure.error.clone()))
            .collect::<BTreeMap<_, _>>();
        let unsupported_servers = discovery_report
            .unsupported_servers
            .iter()
            .map(|server| (server.server_name.clone(), server.reason.clone()))
            .collect::<BTreeMap<_, _>>();

        for (server_name, scoped_config) in runtime_config.mcp().servers() {
            let tool_names = discovered_tool_names
                .get(server_name)
                .cloned()
                .unwrap_or_default();
            let status_detail = failed_servers
                .get(server_name)
                .cloned()
                .or_else(|| unsupported_servers.get(server_name).cloned());
            let availability = if status_detail.is_some() {
                "attention"
            } else if tool_names.is_empty() {
                "configured"
            } else {
                "healthy"
            };
            let source_key = format!("mcp:{server_name}");

            entries.push(WorkspaceToolCatalogEntry {
                id: format!("mcp-{server_name}"),
                workspace_id: workspace_id.clone(),
                name: server_name.clone(),
                kind: "mcp".into(),
                description: "Configured MCP server.".into(),
                required_permission: None,
                availability: availability.into(),
                source_key: source_key.clone(),
                display_path: "config/runtime/workspace.json".into(),
                disabled: disabled_keys.contains(&source_key),
                management: WorkspaceToolManagementCapabilities {
                    can_disable: true,
                    can_edit: true,
                    can_delete: true,
                },
                builtin_key: None,
                active: None,
                shadowed_by: None,
                source_origin: None,
                workspace_owned: None,
                relative_path: None,
                server_name: Some(server_name.clone()),
                endpoint: Some(mcp_endpoint(&scoped_config.config)),
                tool_names: Some(tool_names),
                status_detail,
                scope: Some(mcp_scope_label(scoped_config.scope).into()),
            });
        }

        entries.sort_by(|left, right| {
            left.kind.cmp(&right.kind).then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
        });

        Ok(WorkspaceToolCatalogSnapshot { entries })
    }
}

#[derive(Clone)]
pub struct InfraBundle {
    pub paths: WorkspacePaths,
    pub workspace: Arc<InfraWorkspaceService>,
    pub auth: Arc<InfraAuthService>,
    pub app_registry: Arc<InfraAppRegistryService>,
    pub rbac: Arc<InfraRbacService>,
    pub artifact: Arc<InfraArtifactService>,
    pub inbox: Arc<InfraInboxService>,
    pub knowledge: Arc<InfraKnowledgeService>,
    pub observation: Arc<InfraObservationService>,
}

pub fn initialize_workspace(root: impl Into<PathBuf>) -> Result<WorkspacePaths, AppError> {
    let paths = WorkspacePaths::new(root);
    paths.ensure_layout()?;
    initialize_workspace_config(&paths)?;
    initialize_app_registry(&paths)?;
    initialize_database(&paths)?;
    seed_defaults(&paths)?;
    Ok(paths)
}

pub fn build_infra_bundle(root: impl Into<PathBuf>) -> Result<InfraBundle, AppError> {
    let paths = initialize_workspace(root)?;
    let state = Arc::new(load_state(paths.clone())?);

    Ok(InfraBundle {
        paths: paths.clone(),
        workspace: Arc::new(InfraWorkspaceService {
            state: Arc::clone(&state),
        }),
        auth: Arc::new(InfraAuthService {
            state: Arc::clone(&state),
        }),
        app_registry: Arc::new(InfraAppRegistryService {
            state: Arc::clone(&state),
        }),
        rbac: Arc::new(InfraRbacService {
            _state: Arc::clone(&state),
        }),
        artifact: Arc::new(InfraArtifactService {
            state: Arc::clone(&state),
        }),
        inbox: Arc::new(InfraInboxService {
            state: Arc::clone(&state),
        }),
        knowledge: Arc::new(InfraKnowledgeService {
            state: Arc::clone(&state),
        }),
        observation: Arc::new(InfraObservationService { state }),
    })
}

fn initialize_workspace_config(paths: &WorkspacePaths) -> Result<(), AppError> {
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
    };
    fs::write(&paths.workspace_config, toml::to_string_pretty(&config)?)?;
    Ok(())
}

fn save_workspace_config_file(path: &Path, workspace: &WorkspaceSummary) -> Result<(), AppError> {
    let config = WorkspaceConfigFile {
        id: workspace.id.clone(),
        name: workspace.name.clone(),
        slug: workspace.slug.clone(),
        deployment: workspace.deployment.clone(),
        bootstrap_status: workspace.bootstrap_status.clone(),
        owner_user_id: workspace.owner_user_id.clone(),
        host: workspace.host.clone(),
        listen_address: workspace.listen_address.clone(),
        default_project_id: workspace.default_project_id.clone(),
    };
    fs::write(path, toml::to_string_pretty(&config)?)?;
    Ok(())
}

fn initialize_app_registry(paths: &WorkspacePaths) -> Result<(), AppError> {
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

fn initialize_database(paths: &WorkspacePaths) -> Result<(), AppError> {
    let connection =
        Connection::open(&paths.db_path).map_err(|error| AppError::database(error.to_string()))?;

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
            CREATE TABLE IF NOT EXISTS memberships (
              workspace_id TEXT NOT NULL,
              user_id TEXT NOT NULL,
              role_ids TEXT NOT NULL,
              scope_mode TEXT NOT NULL,
              scope_project_ids TEXT NOT NULL,
              PRIMARY KEY (workspace_id, user_id)
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
              expires_at INTEGER,
              role_ids TEXT NOT NULL,
              scope_project_ids TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS projects (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              name TEXT NOT NULL,
              status TEXT NOT NULL,
              description TEXT NOT NULL,
              assignments_json TEXT
            );
            CREATE TABLE IF NOT EXISTS resources (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              kind TEXT NOT NULL,
              name TEXT NOT NULL,
              location TEXT,
              origin TEXT NOT NULL,
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
              status TEXT NOT NULL,
              source_type TEXT NOT NULL,
              source_ref TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
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
              leader_agent_id TEXT,
              member_agent_ids TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
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
            CREATE TABLE IF NOT EXISTS roles (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              name TEXT NOT NULL,
              code TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              permission_ids TEXT NOT NULL,
              menu_ids TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS permissions (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              name TEXT NOT NULL,
              code TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              kind TEXT NOT NULL,
              target_type TEXT,
              target_ids TEXT NOT NULL,
              action TEXT,
              member_permission_ids TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS menus (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              parent_id TEXT,
              source TEXT NOT NULL,
              label TEXT NOT NULL,
              route_name TEXT,
              status TEXT NOT NULL,
              order_value INTEGER NOT NULL
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
              detail_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pet_presence (
              scope_key TEXT PRIMARY KEY,
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
              project_id TEXT,
              pet_id TEXT NOT NULL,
              workspace_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              session_id TEXT,
              updated_at INTEGER NOT NULL
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
              run_json TEXT NOT NULL
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
            ",
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    ensure_user_avatar_columns(&connection)?;
    ensure_agent_record_columns(&connection)?;
    ensure_team_record_columns(&connection)?;
    ensure_project_assignment_columns(&connection)?;
    ensure_project_agent_link_table(&connection)?;
    ensure_project_team_link_table(&connection)?;
    ensure_runtime_config_snapshot_columns(&connection)?;
    ensure_runtime_session_projection_columns(&connection)?;
    ensure_cost_entry_columns(&connection)?;
    ensure_resource_columns(&connection)?;
    agent_import::ensure_import_source_tables(&connection)?;

    Ok(())
}

fn seed_defaults(paths: &WorkspacePaths) -> Result<(), AppError> {
    let connection =
        Connection::open(&paths.db_path).map_err(|error| AppError::database(error.to_string()))?;
    let default_menu_records = default_menu_records();

    let project_exists: Option<String> = connection
        .query_row(
            "SELECT id FROM projects WHERE id = ?1",
            params![DEFAULT_PROJECT_ID],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if project_exists.is_none() {
        connection
            .execute(
                "INSERT INTO projects (id, workspace_id, name, status, description, assignments_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    DEFAULT_PROJECT_ID,
                    DEFAULT_WORKSPACE_ID,
                    "Default Project",
                    "active",
                    "Bootstrap project for the local workspace.",
                    Option::<String>::None,
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
                    "INSERT INTO resources (id, workspace_id, project_id, kind, name, location, origin, status, updated_at, tags, source_artifact_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.kind,
                        record.name,
                        record.location,
                        record.origin,
                        record.status,
                        record.updated_at as i64,
                        serde_json::to_string(&record.tags)?,
                        record.source_artifact_id,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
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
                    "INSERT INTO knowledge_records (id, workspace_id, project_id, title, summary, kind, status, source_type, source_ref, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.title,
                        record.summary,
                        record.kind,
                        record.status,
                        record.source_type,
                        record.source_ref,
                        record.updated_at as i64,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    let agents_exist: Option<String> = connection
        .query_row("SELECT id FROM agents LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if agents_exist.is_none() {
        let workspace_has_managed_skills = agent_import::workspace_has_managed_skills(paths)?;
        let seeded_agent_ids = if workspace_has_managed_skills {
            Vec::new()
        } else {
            agent_import::seed_bundled_agent_bundle(&connection, paths, DEFAULT_WORKSPACE_ID)?
        };

        if seeded_agent_ids.is_empty() && !workspace_has_managed_skills {
            for record in default_agent_records() {
                connection
                    .execute(
                        "INSERT INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                            record.description,
                            record.status,
                            record.updated_at as i64,
                        ],
                    )
                    .map_err(|error| AppError::database(error.to_string()))?;
            }
        }
    }

    let teams_exist: Option<String> = connection
        .query_row("SELECT id FROM teams LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if teams_exist.is_none() {
        for record in default_team_records() {
            write_team_record(&connection, &record, false)?;
        }
    }

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

    let automations_exist: Option<String> = connection
        .query_row("SELECT id FROM automations LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if automations_exist.is_none() {
        for record in default_automation_records() {
            connection
                .execute(
                    "INSERT INTO automations (id, workspace_id, project_id, title, description, cadence, owner_type, owner_id, status, next_run_at, last_run_at, output)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.title,
                        record.description,
                        record.cadence,
                        record.owner_type,
                        record.owner_id,
                        record.status,
                        record.next_run_at.map(|value| value as i64),
                        record.last_run_at.map(|value| value as i64),
                        record.output,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    let roles_exist: Option<String> = connection
        .query_row("SELECT id FROM roles LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if roles_exist.is_none() {
        for record in default_role_records() {
            connection
                .execute(
                    "INSERT INTO roles (id, workspace_id, name, code, description, status, permission_ids, menu_ids)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.name,
                        record.code,
                        record.description,
                        record.status,
                        serde_json::to_string(&record.permission_ids)?,
                        serde_json::to_string(&record.menu_ids)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    let permissions_exist: Option<String> = connection
        .query_row("SELECT id FROM permissions LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if permissions_exist.is_none() {
        for record in default_permission_records() {
            connection
                .execute(
                    "INSERT INTO permissions (id, workspace_id, name, code, description, status, kind, target_type, target_ids, action, member_permission_ids)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.name,
                        record.code,
                        record.description,
                        record.status,
                        record.kind,
                        record.target_type,
                        serde_json::to_string(&record.target_ids)?,
                        record.action,
                        serde_json::to_string(&record.member_permission_ids)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    for record in &default_menu_records {
        connection
            .execute(
                "INSERT OR IGNORE INTO menus (id, workspace_id, parent_id, source, label, route_name, status, order_value)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    record.id,
                    record.workspace_id,
                    record.parent_id,
                    record.source,
                    record.label,
                    record.route_name,
                    record.status,
                    record.order,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    let owner_menu_ids_raw: Option<String> = connection
        .query_row(
            "SELECT menu_ids FROM roles WHERE id = 'owner' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if let Some(owner_menu_ids_raw) = owner_menu_ids_raw {
        let mut owner_menu_ids: Vec<String> =
            serde_json::from_str(&owner_menu_ids_raw).unwrap_or_default();
        let mut changed = false;

        for menu_id in default_menu_records.iter().map(|record| record.id.as_str()) {
            if owner_menu_ids.iter().any(|existing| existing == menu_id) {
                continue;
            }
            owner_menu_ids.push(menu_id.into());
            changed = true;
        }

        if changed {
            connection
                .execute(
                    "UPDATE roles SET menu_ids = ?1 WHERE id = 'owner'",
                    params![serde_json::to_string(&owner_menu_ids)?],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

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

fn table_columns(connection: &Connection, table_name: &str) -> Result<Vec<String>, AppError> {
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

fn ensure_columns(
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

fn ensure_user_avatar_columns(connection: &Connection) -> Result<(), AppError> {
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

fn ensure_agent_record_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "agents",
        &[
            ("avatar_path", "TEXT"),
            ("personality", "TEXT NOT NULL DEFAULT ''"),
            ("tags", "TEXT NOT NULL DEFAULT '[]'"),
            ("prompt", "TEXT NOT NULL DEFAULT ''"),
            ("builtin_tool_keys", "TEXT NOT NULL DEFAULT '[]'"),
            ("skill_ids", "TEXT NOT NULL DEFAULT '[]'"),
            ("mcp_server_names", "TEXT NOT NULL DEFAULT '[]'"),
        ],
    )
}

fn ensure_team_record_columns(connection: &Connection) -> Result<(), AppError> {
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
            ("leader_agent_id", "TEXT"),
            ("member_agent_ids", "TEXT NOT NULL DEFAULT '[]'"),
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

fn write_team_record(
    connection: &Connection,
    record: &TeamRecord,
    replace: bool,
) -> Result<(), AppError> {
    let member_agent_ids_json = serde_json::to_string(&record.member_agent_ids)?;
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
            "{verb} INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, leader_agent_id, member_ids, member_agent_ids, description, status, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)"
        )
    } else {
        format!(
            "{verb} INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, leader_agent_id, member_agent_ids, description, status, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)"
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
                record.leader_agent_id,
                member_agent_ids_json,
                member_agent_ids_json,
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
                record.leader_agent_id,
                member_agent_ids_json,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        )
    }
    .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

fn ensure_project_assignment_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(connection, "projects", &[("assignments_json", "TEXT")])
}

fn ensure_project_agent_link_table(connection: &Connection) -> Result<(), AppError> {
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

fn ensure_project_team_link_table(connection: &Connection) -> Result<(), AppError> {
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

fn ensure_runtime_config_snapshot_columns(connection: &Connection) -> Result<(), AppError> {
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

fn ensure_runtime_session_projection_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "runtime_session_projections",
        &[
            ("session_kind", "TEXT NOT NULL DEFAULT 'project'"),
            ("last_message_preview", "TEXT"),
            ("config_snapshot_id", "TEXT NOT NULL DEFAULT ''"),
            ("effective_config_hash", "TEXT NOT NULL DEFAULT ''"),
            ("started_from_scope_set", "TEXT NOT NULL DEFAULT '[]'"),
            (
                "detail_json",
                r#"TEXT NOT NULL DEFAULT '{"summary":{"id":"","conversationId":"","projectId":"","title":"","sessionKind":"project","status":"draft","updatedAt":0,"lastMessagePreview":null,"configSnapshotId":"","effectiveConfigHash":"","startedFromScopeSet":[]},"run":{"id":"","sessionId":"","conversationId":"","status":"draft","currentStep":"ready","startedAt":0,"updatedAt":0,"configuredModelId":null,"configuredModelName":null,"modelId":null,"consumedTokens":null,"nextAction":null,"configSnapshotId":"","effectiveConfigHash":"","startedFromScopeSet":[],"resolvedTarget":null,"requestedActorKind":null,"requestedActorId":null,"resolvedActorKind":null,"resolvedActorId":null,"resolvedActorLabel":null},"messages":[],"trace":[],"pendingApproval":null}'"#,
            ),
        ],
    )
}

fn ensure_cost_entry_columns(connection: &Connection) -> Result<(), AppError> {
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

    Ok(())
}

fn ensure_resource_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(connection, "resources", &[("source_artifact_id", "TEXT")])
}

fn load_state(paths: WorkspacePaths) -> Result<InfraState, AppError> {
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
    };

    let app_registry: AppRegistryFile =
        toml::from_str(&fs::read_to_string(&paths.app_registry_config)?)?;
    let connection =
        Connection::open(&paths.db_path).map_err(|error| AppError::database(error.to_string()))?;
    let users = load_users(&connection)?;
    let owner_user_id = users
        .iter()
        .find(|user| {
            user.membership
                .role_ids
                .iter()
                .any(|role_id| role_id == "owner")
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
        save_workspace_config_file(&paths.workspace_config, &workspace)?;
    }
    let projects = load_projects(&connection)?;
    let sessions = load_sessions(&connection)?;
    let resources = load_resources(&connection)?;
    let knowledge_records = load_knowledge_records(&connection)?;
    let agents = load_agents(&connection)?;
    let project_agent_links = load_project_agent_links(&connection)?;
    let teams = load_teams(&connection)?;
    let project_team_links = load_project_team_links(&connection)?;
    let model_catalog = load_model_catalog(&connection)?;
    let provider_credentials = load_provider_credentials(&connection)?;
    let tools = load_tools(&connection)?;
    let automations = load_automations(&connection)?;
    let roles = load_roles(&connection)?;
    let permissions = load_permissions(&connection)?;
    let menus = load_menus(&connection)?;
    let trace_events = load_trace_events(&connection)?;
    let audit_records = load_audit_records(&connection)?;
    let cost_entries = load_cost_entries(&connection)?;
    let workspace_pet_presence =
        load_pet_presence(&connection, "workspace")?.unwrap_or_else(default_workspace_pet_presence);
    let project_pet_presences = load_all_project_pet_presences(&connection)?;
    let workspace_pet_binding = load_pet_binding(&connection, "workspace")?;
    let project_pet_bindings = load_all_project_pet_bindings(&connection)?;

    Ok(InfraState {
        paths,
        workspace: Mutex::new(workspace),
        users: Mutex::new(users),
        apps: Mutex::new(app_registry.apps),
        sessions: Mutex::new(sessions),
        projects: Mutex::new(projects),
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
        roles: Mutex::new(roles),
        permissions: Mutex::new(permissions),
        menus: Mutex::new(menus),
        artifacts: Mutex::new(Vec::new()),
        inbox: Mutex::new(Vec::new()),
        trace_events: Mutex::new(trace_events),
        audit_records: Mutex::new(audit_records),
        cost_entries: Mutex::new(cost_entries),
        workspace_pet_presence: Mutex::new(workspace_pet_presence),
        project_pet_presences: Mutex::new(project_pet_presences),
        workspace_pet_binding: Mutex::new(workspace_pet_binding),
        project_pet_bindings: Mutex::new(project_pet_bindings),
    })
}

fn load_users(connection: &Connection) -> Result<Vec<StoredUser>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT u.id, u.username, u.display_name, u.avatar_path, u.avatar_content_type, u.avatar_byte_size, u.avatar_content_hash,
                    u.status, u.password_hash, u.password_state, u.created_at, u.updated_at,
                    m.workspace_id, m.role_ids, m.scope_mode, m.scope_project_ids
             FROM users u
             LEFT JOIN memberships m ON m.user_id = u.id",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let role_ids_raw: String = row.get(13)?;
            let scope_project_ids_raw: String = row.get(15)?;
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
                membership: WorkspaceMembershipRecord {
                    workspace_id: row.get(12)?,
                    user_id: row.get(0)?,
                    role_ids: serde_json::from_str(&role_ids_raw).unwrap_or_default(),
                    scope_mode: row.get(14)?,
                    scope_project_ids: serde_json::from_str(&scope_project_ids_raw)
                        .unwrap_or_default(),
                },
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_projects(connection: &Connection) -> Result<Vec<ProjectRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, name, status, description, assignments_json FROM projects",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let assignments_json: Option<String> = row.get(5)?;
            let assignments = assignments_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<ProjectWorkspaceAssignments>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            Ok(ProjectRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                status: row.get(3)?,
                description: row.get(4)?,
                assignments,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn default_pet_profile() -> PetProfile {
    PetProfile {
        id: "pet-octopus".into(),
        species: "octopus".into(),
        display_name: "小章".into(),
        owner_user_id: "user-owner".into(),
        avatar_label: "Octopus mascot".into(),
        summary: "Octopus 首席吉祥物，负责卖萌和加油。".into(),
        greeting: "嗨！我是小章，今天也要加油哦！".into(),
        mood: "happy".into(),
        favorite_snack: "新鲜小虾".into(),
        prompt_hints: vec![
            "最近有什么好消息？".into(),
            "给我讲个冷笑话".into(),
            "我们要加油呀！".into(),
        ],
        fallback_asset: "octopus".into(),
        rive_asset: None,
        state_machine: None,
    }
}

fn default_workspace_pet_presence() -> PetPresenceState {
    PetPresenceState {
        pet_id: "pet-octopus".into(),
        is_visible: true,
        chat_open: false,
        motion_state: "idle".into(),
        unread_count: 0,
        last_interaction_at: 0,
        position: PetPosition { x: 0, y: 0 },
    }
}

fn map_pet_message(pet_id: &str, message: &octopus_core::RuntimeMessage) -> PetMessage {
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

fn load_runtime_messages_for_conversation(
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

fn row_to_pet_presence(row: &rusqlite::Row<'_>) -> rusqlite::Result<PetPresenceState> {
    Ok(PetPresenceState {
        pet_id: row.get(2)?,
        is_visible: row.get::<_, i64>(3)? != 0,
        chat_open: row.get::<_, i64>(4)? != 0,
        motion_state: row.get(5)?,
        unread_count: row.get::<_, i64>(6)? as u64,
        last_interaction_at: row.get::<_, i64>(7)? as u64,
        position: PetPosition {
            x: row.get(8)?,
            y: row.get(9)?,
        },
    })
}

fn load_pet_presence(
    connection: &Connection,
    scope_key: &str,
) -> Result<Option<PetPresenceState>, AppError> {
    connection
        .query_row(
            "SELECT scope_key, project_id, pet_id, is_visible, chat_open, motion_state, unread_count, last_interaction_at, position_x, position_y FROM pet_presence WHERE scope_key = ?1",
            params![scope_key],
            row_to_pet_presence,
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_all_project_pet_presences(
    connection: &Connection,
) -> Result<Vec<(String, PetPresenceState)>, AppError> {
    let mut stmt = connection
        .prepare("SELECT scope_key, project_id, pet_id, is_visible, chat_open, motion_state, unread_count, last_interaction_at, position_x, position_y FROM pet_presence WHERE project_id IS NOT NULL")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                row_to_pet_presence(row)?,
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn row_to_pet_binding(row: &rusqlite::Row<'_>) -> rusqlite::Result<PetConversationBinding> {
    Ok(PetConversationBinding {
        pet_id: row.get(2)?,
        workspace_id: row.get(3)?,
        project_id: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        conversation_id: row.get(4)?,
        session_id: row.get(5)?,
        updated_at: row.get::<_, i64>(6)? as u64,
    })
}

fn load_pet_binding(
    connection: &Connection,
    scope_key: &str,
) -> Result<Option<PetConversationBinding>, AppError> {
    connection
        .query_row(
            "SELECT scope_key, project_id, pet_id, workspace_id, conversation_id, session_id, updated_at FROM pet_conversation_bindings WHERE scope_key = ?1",
            params![scope_key],
            row_to_pet_binding,
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_all_project_pet_bindings(
    connection: &Connection,
) -> Result<Vec<(String, PetConversationBinding)>, AppError> {
    let mut stmt = connection
        .prepare("SELECT scope_key, project_id, pet_id, workspace_id, conversation_id, session_id, updated_at FROM pet_conversation_bindings WHERE project_id IS NOT NULL")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                row_to_pet_binding(row)?,
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_resources(connection: &Connection) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, kind, name, location, origin, status, updated_at, tags, source_artifact_id FROM resources",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let tags_raw: String = row.get(9)?;
            Ok(WorkspaceResourceRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                kind: row.get(3)?,
                name: row.get(4)?,
                location: row.get(5)?,
                origin: row.get(6)?,
                status: row.get(7)?,
                updated_at: row.get::<_, i64>(8)? as u64,
                tags: serde_json::from_str(&tags_raw).unwrap_or_default(),
                source_artifact_id: row.get(10)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_knowledge_records(connection: &Connection) -> Result<Vec<KnowledgeRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, title, summary, kind, status, source_type, source_ref, updated_at FROM knowledge_records",
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
                status: row.get(6)?,
                source_type: row.get(7)?,
                source_ref: row.get(8)?,
                updated_at: row.get::<_, i64>(9)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn agent_avatar(paths: &WorkspacePaths, avatar_path: Option<&str>) -> Option<String> {
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
        _ => return Some(avatar_path.to_string()),
    };
    Some(format!(
        "data:{content_type};base64,{}",
        BASE64_STANDARD.encode(bytes)
    ))
}

fn load_agents(connection: &Connection) -> Result<Vec<AgentRecord>, AppError> {
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
            "SELECT id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at FROM agents",
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
            Ok(AgentRecord {
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
                builtin_tool_keys: serde_json::from_str(&builtin_tool_keys_raw).unwrap_or_default(),
                skill_ids: serde_json::from_str(&skill_ids_raw).unwrap_or_default(),
                mcp_server_names: serde_json::from_str(&mcp_server_names_raw).unwrap_or_default(),
                integration_source: None,
                description: row.get(12)?,
                status: row.get(13)?,
                updated_at: row.get::<_, i64>(14)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_project_agent_links(
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

fn load_teams(connection: &Connection) -> Result<Vec<TeamRecord>, AppError> {
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
            "SELECT id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, leader_agent_id, member_agent_ids, description, status, updated_at FROM teams",
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
            let member_agent_ids_raw: String = row.get(13)?;
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
                builtin_tool_keys: serde_json::from_str(&builtin_tool_keys_raw).unwrap_or_default(),
                skill_ids: serde_json::from_str(&skill_ids_raw).unwrap_or_default(),
                mcp_server_names: serde_json::from_str(&mcp_server_names_raw).unwrap_or_default(),
                leader_agent_id: row.get(12)?,
                member_agent_ids: serde_json::from_str(&member_agent_ids_raw).unwrap_or_default(),
                integration_source: None,
                description: row.get(14)?,
                status: row.get(15)?,
                updated_at: row.get::<_, i64>(16)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_project_team_links(
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

fn load_model_catalog(connection: &Connection) -> Result<Vec<ModelCatalogRecord>, AppError> {
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

fn load_provider_credentials(
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

fn load_tools(connection: &Connection) -> Result<Vec<ToolRecord>, AppError> {
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

fn load_automations(connection: &Connection) -> Result<Vec<AutomationRecord>, AppError> {
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

fn load_roles(connection: &Connection) -> Result<Vec<RoleRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, name, code, description, status, permission_ids, menu_ids FROM roles",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let permission_ids_raw: String = row.get(6)?;
            let menu_ids_raw: String = row.get(7)?;
            Ok(RoleRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                code: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                permission_ids: serde_json::from_str(&permission_ids_raw).unwrap_or_default(),
                menu_ids: serde_json::from_str(&menu_ids_raw).unwrap_or_default(),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_permissions(connection: &Connection) -> Result<Vec<PermissionRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, name, code, description, status, kind, target_type, target_ids, action, member_permission_ids FROM permissions",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let target_ids_raw: String = row.get(8)?;
            let member_permission_ids_raw: String = row.get(10)?;
            Ok(PermissionRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                code: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                kind: row.get(6)?,
                target_type: row.get(7)?,
                target_ids: serde_json::from_str(&target_ids_raw).unwrap_or_default(),
                action: row.get(9)?,
                member_permission_ids: serde_json::from_str(&member_permission_ids_raw)
                    .unwrap_or_default(),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_menus(connection: &Connection) -> Result<Vec<MenuRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, parent_id, source, label, route_name, status, order_value FROM menus ORDER BY order_value ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(MenuRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                parent_id: row.get(2)?,
                source: row.get(3)?,
                label: row.get(4)?,
                route_name: row.get(5)?,
                status: row.get(6)?,
                order: row.get(7)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_sessions(connection: &Connection) -> Result<Vec<SessionRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, user_id, client_app_id, token, status, created_at, expires_at, role_ids, scope_project_ids
             FROM sessions",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let role_ids_raw: String = row.get(8)?;
            let scope_project_ids_raw: String = row.get(9)?;
            Ok(SessionRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                user_id: row.get(2)?,
                client_app_id: row.get(3)?,
                token: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get::<_, i64>(6)? as u64,
                expires_at: row.get::<_, Option<i64>>(7)?.map(|value| value as u64),
                role_ids: serde_json::from_str(&role_ids_raw).unwrap_or_default(),
                scope_project_ids: serde_json::from_str(&scope_project_ids_raw).unwrap_or_default(),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_trace_events(connection: &Connection) -> Result<Vec<TraceEventRecord>, AppError> {
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

fn load_audit_records(connection: &Connection) -> Result<Vec<AuditRecord>, AppError> {
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

fn load_cost_entries(connection: &Connection) -> Result<Vec<CostLedgerEntry>, AppError> {
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

fn default_workspace_resources() -> Vec<WorkspaceResourceRecord> {
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
            status: "configured".into(),
            updated_at: now,
            tags: vec!["project".into(), "delivery".into()],
            source_artifact_id: None,
        },
    ]
}

fn default_knowledge_records() -> Vec<KnowledgeRecord> {
    let now = timestamp_now();
    vec![
        KnowledgeRecord {
            id: "kn-workspace-onboarding".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: None,
            title: "Workspace onboarding".into(),
            summary: "Shared operating rules, review expectations, and release cadence for this workspace.".into(),
            kind: "shared".into(),
            status: "shared".into(),
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
            status: "reviewed".into(),
            source_type: "run".into(),
            source_ref: "default-project".into(),
            updated_at: now,
        },
    ]
}

fn default_agent_records() -> Vec<AgentRecord> {
    let now = timestamp_now();
    vec![
        AgentRecord {
            id: "agent-orchestrator".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: None,
            scope: "workspace".into(),
            name: "Workspace Orchestrator".into(),
            avatar_path: None,
            avatar: None,
            personality: "System coordinator".into(),
            tags: vec!["workspace".into(), "orchestration".into()],
            prompt: "Coordinate work across the workspace and keep execution aligned.".into(),
            builtin_tool_keys: vec![],
            skill_ids: vec![],
            mcp_server_names: vec![],
            integration_source: None,
            description: "Coordinates projects, approvals, and execution policies.".into(),
            status: "active".into(),
            updated_at: now,
        },
        AgentRecord {
            id: "agent-project-delivery".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: Some(DEFAULT_PROJECT_ID.into()),
            scope: "project".into(),
            name: "Project Delivery Agent".into(),
            avatar_path: None,
            avatar: None,
            personality: "Delivery lead".into(),
            tags: vec!["project".into(), "delivery".into()],
            prompt: "Track project work, runtime sessions, and follow-up actions.".into(),
            builtin_tool_keys: vec![],
            skill_ids: vec![],
            mcp_server_names: vec![],
            integration_source: None,
            description: "Tracks project work, runtime sessions, and follow-up actions.".into(),
            status: "active".into(),
            updated_at: now,
        },
    ]
}

fn default_team_records() -> Vec<TeamRecord> {
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
        leader_agent_id: Some("agent-orchestrator".into()),
        member_agent_ids: vec!["agent-orchestrator".into()],
        integration_source: None,
        description: "Maintains workspace-wide operating standards and governance.".into(),
        status: "active".into(),
        updated_at: now,
    }]
}

fn default_model_catalog() -> Vec<ModelCatalogRecord> {
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

fn default_provider_credentials() -> Vec<ProviderCredentialRecord> {
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

fn default_tool_records() -> Vec<ToolRecord> {
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

fn default_automation_records() -> Vec<AutomationRecord> {
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

fn default_permission_records() -> Vec<PermissionRecord> {
    vec![
        PermissionRecord {
            id: "perm-workspace-read".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            name: "Workspace Read".into(),
            code: "workspace.read".into(),
            description: "Read workspace-level resources and projections.".into(),
            status: "active".into(),
            kind: "atomic".into(),
            target_type: Some("workspace".into()),
            target_ids: vec![DEFAULT_WORKSPACE_ID.into()],
            action: Some("read".into()),
            member_permission_ids: Vec::new(),
        },
        PermissionRecord {
            id: "perm-runtime-read".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            name: "Runtime Read".into(),
            code: "runtime.read".into(),
            description: "Read runtime sessions and event streams.".into(),
            status: "active".into(),
            kind: "atomic".into(),
            target_type: Some("project".into()),
            target_ids: vec![DEFAULT_PROJECT_ID.into()],
            action: Some("read".into()),
            member_permission_ids: Vec::new(),
        },
    ]
}

fn default_menu_records() -> Vec<MenuRecord> {
    vec![
        MenuRecord {
            id: "menu-workspace-overview".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Overview".into(),
            route_name: Some("workspace-overview".into()),
            status: "active".into(),
            order: 10,
        },
        MenuRecord {
            id: "menu-workspace-projects".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Projects".into(),
            route_name: Some("workspace-projects".into()),
            status: "active".into(),
            order: 15,
        },
        MenuRecord {
            id: "menu-workspace-resources".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Resources".into(),
            route_name: Some("workspace-resources".into()),
            status: "active".into(),
            order: 20,
        },
        MenuRecord {
            id: "menu-workspace-knowledge".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Knowledge".into(),
            route_name: Some("workspace-knowledge".into()),
            status: "active".into(),
            order: 30,
        },
        MenuRecord {
            id: "menu-workspace-agents".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Agents".into(),
            route_name: Some("workspace-agents".into()),
            status: "active".into(),
            order: 40,
        },
        MenuRecord {
            id: "menu-workspace-teams".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Teams".into(),
            route_name: Some("workspace-teams".into()),
            status: "active".into(),
            order: 50,
        },
        MenuRecord {
            id: "menu-workspace-models".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Models".into(),
            route_name: Some("workspace-models".into()),
            status: "active".into(),
            order: 60,
        },
        MenuRecord {
            id: "menu-workspace-tools".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Tools".into(),
            route_name: Some("workspace-tools".into()),
            status: "active".into(),
            order: 70,
        },
        MenuRecord {
            id: "menu-workspace-automations".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Automations".into(),
            route_name: Some("workspace-automations".into()),
            status: "active".into(),
            order: 80,
        },
        MenuRecord {
            id: "menu-workspace-user-center".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "User Center".into(),
            route_name: Some("workspace-user-center".into()),
            status: "active".into(),
            order: 90,
        },
        MenuRecord {
            id: "menu-workspace-user-center-profile".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-user-center".into()),
            source: "user-center".into(),
            label: "Profile".into(),
            route_name: Some("workspace-user-center-profile".into()),
            status: "active".into(),
            order: 100,
        },
        MenuRecord {
            id: "menu-workspace-user-center-pet".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-user-center".into()),
            source: "user-center".into(),
            label: "Pet".into(),
            route_name: Some("workspace-user-center-pet".into()),
            status: "active".into(),
            order: 105,
        },
        MenuRecord {
            id: "menu-workspace-user-center-users".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-user-center".into()),
            source: "user-center".into(),
            label: "Users".into(),
            route_name: Some("workspace-user-center-users".into()),
            status: "active".into(),
            order: 110,
        },
        MenuRecord {
            id: "menu-workspace-user-center-roles".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-user-center".into()),
            source: "user-center".into(),
            label: "Roles".into(),
            route_name: Some("workspace-user-center-roles".into()),
            status: "active".into(),
            order: 120,
        },
        MenuRecord {
            id: "menu-workspace-user-center-permissions".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-user-center".into()),
            source: "user-center".into(),
            label: "Permissions".into(),
            route_name: Some("workspace-user-center-permissions".into()),
            status: "active".into(),
            order: 130,
        },
        MenuRecord {
            id: "menu-workspace-user-center-menus".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-user-center".into()),
            source: "user-center".into(),
            label: "Menus".into(),
            route_name: Some("workspace-user-center-menus".into()),
            status: "active".into(),
            order: 140,
        },
    ]
}

fn default_role_records() -> Vec<RoleRecord> {
    vec![RoleRecord {
        id: "owner".into(),
        workspace_id: DEFAULT_WORKSPACE_ID.into(),
        name: "Owner".into(),
        code: "owner".into(),
        description: "Full workspace access.".into(),
        status: "active".into(),
        permission_ids: default_permission_records()
            .into_iter()
            .map(|record| record.id)
            .collect(),
        menu_ids: default_menu_records()
            .into_iter()
            .map(|record| record.id)
            .collect(),
    }]
}

fn avatar_data_url(paths: &WorkspacePaths, user: &StoredUser) -> Option<String> {
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

fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("hash-{:x}", hasher.finish())
}

fn to_user_summary(paths: &WorkspacePaths, user: &StoredUser) -> UserRecordSummary {
    UserRecordSummary {
        id: user.record.id.clone(),
        username: user.record.username.clone(),
        display_name: user.record.display_name.clone(),
        avatar: avatar_data_url(paths, user),
        status: user.record.status.clone(),
        password_state: user.record.password_state.clone(),
        role_ids: user.membership.role_ids.clone(),
        scope_project_ids: user.membership.scope_project_ids.clone(),
    }
}

fn default_client_apps() -> Vec<ClientAppRecord> {
    vec![
        ClientAppRecord {
            id: "octopus-desktop".into(),
            name: "Octopus Desktop".into(),
            platform: "desktop".into(),
            status: "active".into(),
            first_party: true,
            allowed_origins: Vec::new(),
            allowed_hosts: vec!["127.0.0.1".into(), "localhost".into()],
            session_policy: "session_token".into(),
            default_scopes: vec!["workspace".into(), "runtime".into()],
        },
        ClientAppRecord {
            id: "octopus-web".into(),
            name: "Octopus Web".into(),
            platform: "web".into(),
            status: "active".into(),
            first_party: true,
            allowed_origins: vec!["http://127.0.0.1".into(), "http://localhost".into()],
            allowed_hosts: vec!["127.0.0.1".into(), "localhost".into()],
            session_policy: "session_token".into(),
            default_scopes: vec!["workspace".into(), "runtime".into()],
        },
        ClientAppRecord {
            id: "octopus-mobile".into(),
            name: "Octopus Mobile".into(),
            platform: "mobile".into(),
            status: "disabled".into(),
            first_party: true,
            allowed_origins: Vec::new(),
            allowed_hosts: Vec::new(),
            session_policy: "session_token".into(),
            default_scopes: vec!["workspace".into()],
        },
    ]
}

fn hash_password(password: &str) -> String {
    format!("plain::{password}")
}

fn verify_password(password: &str, hash: &str) -> bool {
    hash == hash_password(password)
}

fn append_json_line(path: &Path, value: &impl Serialize) -> Result<(), AppError> {
    let mut raw = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };
    raw.push_str(&serde_json::to_string(value)?);
    raw.push('\n');
    fs::write(path, raw)?;
    Ok(())
}

impl InfraWorkspaceService {
    fn now() -> u64 {
        timestamp_now()
    }

    fn ensure_project_exists(&self, project_id: &str) -> Result<(), AppError> {
        let exists = self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?
            .iter()
            .any(|project| project.id == project_id);
        if exists {
            Ok(())
        } else {
            Err(AppError::not_found("project not found"))
        }
    }

    fn pet_scope_key(project_id: Option<&str>) -> String {
        project_id.unwrap_or("workspace").to_string()
    }

    fn workspace_pet_snapshot(&self) -> Result<PetWorkspaceSnapshot, AppError> {
        let profile = default_pet_profile();
        let presence = self
            .state
            .workspace_pet_presence
            .lock()
            .map_err(|_| AppError::runtime("workspace pet presence mutex poisoned"))?
            .clone();
        let binding = self
            .state
            .workspace_pet_binding
            .lock()
            .map_err(|_| AppError::runtime("workspace pet binding mutex poisoned"))?
            .clone();
        let messages = if let Some(binding) = binding.as_ref() {
            load_runtime_messages_for_conversation(
                &self.state.open_db()?,
                &binding.conversation_id,
                &profile.id,
            )?
        } else {
            vec![]
        };
        Ok(PetWorkspaceSnapshot {
            profile,
            presence,
            binding,
            messages,
        })
    }

    fn project_pet_snapshot(&self, project_id: &str) -> Result<PetWorkspaceSnapshot, AppError> {
        self.ensure_project_exists(project_id)?;
        let profile = default_pet_profile();
        let presence = self
            .state
            .project_pet_presences
            .lock()
            .map_err(|_| AppError::runtime("project pet presences mutex poisoned"))?
            .iter()
            .find(|(id, _)| id == project_id)
            .map(|(_, presence)| presence.clone())
            .unwrap_or_else(default_workspace_pet_presence);
        let binding = self
            .state
            .project_pet_bindings
            .lock()
            .map_err(|_| AppError::runtime("project pet bindings mutex poisoned"))?
            .iter()
            .find(|(id, _)| id == project_id)
            .map(|(_, binding)| binding.clone());
        let messages = if let Some(binding) = binding.as_ref() {
            load_runtime_messages_for_conversation(
                &self.state.open_db()?,
                &binding.conversation_id,
                &profile.id,
            )?
        } else {
            vec![]
        };
        Ok(PetWorkspaceSnapshot {
            profile,
            presence,
            binding,
            messages,
        })
    }

    fn persist_pet_presence(
        &self,
        project_id: Option<&str>,
        presence: &PetPresenceState,
    ) -> Result<(), AppError> {
        let scope_key = Self::pet_scope_key(project_id);
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO pet_presence (scope_key, project_id, pet_id, is_visible, chat_open, motion_state, unread_count, last_interaction_at, position_x, position_y)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                scope_key,
                project_id,
                presence.pet_id,
                if presence.is_visible { 1 } else { 0 },
                if presence.chat_open { 1 } else { 0 },
                presence.motion_state,
                presence.unread_count as i64,
                presence.last_interaction_at as i64,
                presence.position.x,
                presence.position.y,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    fn persist_pet_binding(
        &self,
        project_id: Option<&str>,
        binding: &PetConversationBinding,
    ) -> Result<(), AppError> {
        let scope_key = Self::pet_scope_key(project_id);
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO pet_conversation_bindings (scope_key, project_id, pet_id, workspace_id, conversation_id, session_id, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                scope_key,
                project_id,
                binding.pet_id,
                binding.workspace_id,
                binding.conversation_id,
                binding.session_id,
                binding.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    fn normalize_project_name(name: &str) -> Result<String, AppError> {
        let normalized = name.trim();
        if normalized.is_empty() {
            return Err(AppError::invalid_input("project name is required"));
        }
        Ok(normalized.into())
    }

    fn normalize_project_description(description: &str) -> String {
        description.trim().into()
    }

    fn normalize_project_status(status: &str) -> Result<String, AppError> {
        match status.trim() {
            "active" => Ok("active".into()),
            "archived" => Ok("archived".into()),
            _ => Err(AppError::invalid_input(
                "project status must be active or archived",
            )),
        }
    }

    fn next_active_project_id(
        projects: &[ProjectRecord],
        current_project_id: &str,
    ) -> Option<String> {
        projects
            .iter()
            .find(|project| project.id != current_project_id && project.status == "active")
            .map(|project| project.id.clone())
    }

    fn replace_or_push<T, F>(items: &mut Vec<T>, value: T, matcher: F)
    where
        F: Fn(&T) -> bool,
    {
        if let Some(existing) = items.iter_mut().find(|item| matcher(item)) {
            *existing = value;
        } else {
            items.push(value);
        }
    }

    fn remove_avatar_file(&self, avatar_path: Option<&str>) -> Result<(), AppError> {
        let Some(avatar_path) = avatar_path else {
            return Ok(());
        };

        match fs::remove_file(self.state.paths.root.join(avatar_path)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    fn persist_workspace_avatar(
        &self,
        entity_id: &str,
        avatar: &AvatarUploadPayload,
    ) -> Result<(String, String, u64, String), AppError> {
        let content_type = avatar.content_type.trim().to_ascii_lowercase();
        if !matches!(
            content_type.as_str(),
            "image/png" | "image/jpeg" | "image/jpg" | "image/webp"
        ) {
            return Err(AppError::invalid_input("avatar must be png, jpeg, or webp"));
        }
        if avatar.byte_size == 0 || avatar.byte_size > 2 * 1024 * 1024 {
            return Err(AppError::invalid_input("avatar must be 2 MiB or smaller"));
        }

        let bytes = BASE64_STANDARD
            .decode(&avatar.data_base64)
            .map_err(|_| AppError::invalid_input("avatar payload is invalid"))?;
        if bytes.len() as u64 != avatar.byte_size {
            return Err(AppError::invalid_input("avatar byte size mismatch"));
        }

        let extension = match content_type.as_str() {
            "image/png" => "png",
            "image/webp" => "webp",
            _ => "jpg",
        };
        let relative_path = format!("data/blobs/avatars/{entity_id}.{extension}");
        let absolute_path = self.state.paths.root.join(&relative_path);
        fs::write(&absolute_path, &bytes)?;

        Ok((
            relative_path,
            content_type,
            avatar.byte_size,
            content_hash(&bytes),
        ))
    }

    fn persist_avatar(
        &self,
        user_id: &str,
        avatar: &AvatarUploadPayload,
    ) -> Result<(String, String, u64, String), AppError> {
        self.persist_workspace_avatar(user_id, avatar)
    }

    fn build_agent_record(
        &self,
        agent_id: &str,
        input: UpsertAgentInput,
        current: Option<&AgentRecord>,
    ) -> Result<AgentRecord, AppError> {
        let next_avatar_path = if input.remove_avatar.unwrap_or(false) {
            None
        } else if let Some(avatar) = input.avatar.as_ref() {
            Some(self.persist_workspace_avatar(agent_id, avatar)?.0)
        } else {
            current.and_then(|record| record.avatar_path.clone())
        };
        let avatar = agent_avatar(&self.state.paths, next_avatar_path.as_deref());

        Ok(AgentRecord {
            id: agent_id.into(),
            workspace_id: if input.workspace_id.trim().is_empty() {
                self.state.workspace_id()?
            } else {
                input.workspace_id
            },
            project_id: input.project_id,
            scope: input.scope,
            name: input.name.trim().into(),
            avatar_path: next_avatar_path,
            avatar,
            personality: input.personality.trim().into(),
            tags: input.tags,
            prompt: input.prompt.trim().into(),
            builtin_tool_keys: input.builtin_tool_keys,
            skill_ids: input.skill_ids,
            mcp_server_names: input.mcp_server_names,
            integration_source: None,
            description: input.description.trim().into(),
            status: input.status.trim().into(),
            updated_at: Self::now(),
        })
    }

    fn build_team_record(
        &self,
        team_id: &str,
        input: UpsertTeamInput,
        current: Option<&TeamRecord>,
    ) -> Result<TeamRecord, AppError> {
        let next_avatar_path = if input.remove_avatar.unwrap_or(false) {
            None
        } else if let Some(avatar) = input.avatar.as_ref() {
            Some(self.persist_workspace_avatar(team_id, avatar)?.0)
        } else {
            current.and_then(|record| record.avatar_path.clone())
        };
        let avatar = agent_avatar(&self.state.paths, next_avatar_path.as_deref());

        Ok(TeamRecord {
            id: team_id.into(),
            workspace_id: if input.workspace_id.trim().is_empty() {
                self.state.workspace_id()?
            } else {
                input.workspace_id
            },
            project_id: input.project_id,
            scope: input.scope,
            name: input.name.trim().into(),
            avatar_path: next_avatar_path,
            avatar,
            personality: input.personality.trim().into(),
            tags: input.tags,
            prompt: input.prompt.trim().into(),
            builtin_tool_keys: input.builtin_tool_keys,
            skill_ids: input.skill_ids,
            mcp_server_names: input.mcp_server_names,
            leader_agent_id: input
                .leader_agent_id
                .filter(|value| !value.trim().is_empty()),
            member_agent_ids: input.member_agent_ids,
            integration_source: None,
            description: input.description.trim().into(),
            status: input.status.trim().into(),
            updated_at: Self::now(),
        })
    }

    fn validate_workspace_user_identity(
        &self,
        username: &str,
        display_name: &str,
        exclude_user_id: Option<&str>,
    ) -> Result<(), AppError> {
        if username.trim().is_empty() || display_name.trim().is_empty() {
            return Err(AppError::invalid_input(
                "username and display name are required",
            ));
        }

        let users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?;
        let username_exists = users.iter().any(|user| {
            if let Some(excluded_id) = exclude_user_id {
                if user.record.id == excluded_id {
                    return false;
                }
            }
            user.record.username == username.trim()
        });
        if username_exists {
            return Err(AppError::conflict("username already exists"));
        }
        Ok(())
    }

    fn resolve_member_password(
        &self,
        password: Option<&str>,
        confirm_password: Option<&str>,
        use_default_password: bool,
    ) -> Result<(String, String), AppError> {
        if use_default_password || password.is_none() {
            return Ok((hash_password("changeme"), "reset-required".into()));
        }

        let password = password.unwrap_or_default();
        let confirm_password = confirm_password.unwrap_or_default();
        if password.len() < 8 {
            return Err(AppError::invalid_input(
                "password must be at least 8 characters",
            ));
        }
        if password != confirm_password {
            return Err(AppError::invalid_input(
                "password confirmation does not match",
            ));
        }

        Ok((hash_password(password), "set".into()))
    }
}

impl InfraAuthService {
    fn now() -> u64 {
        timestamp_now()
    }

    fn workspace_snapshot(&self) -> Result<WorkspaceSummary, AppError> {
        self.state.workspace_snapshot()
    }

    fn ensure_active_client_app(&self, client_app_id: &str) -> Result<ClientAppRecord, AppError> {
        let app = self
            .state
            .apps
            .lock()
            .map_err(|_| AppError::runtime("app registry mutex poisoned"))?
            .iter()
            .find(|app| app.id == client_app_id)
            .cloned()
            .ok_or_else(|| AppError::auth("client app is not registered"))?;
        if app.status != "active" {
            return Err(AppError::auth("client app is disabled"));
        }
        Ok(app)
    }

    fn owner_exists(&self) -> Result<bool, AppError> {
        Ok(self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .iter()
            .any(|user| {
                user.membership
                    .role_ids
                    .iter()
                    .any(|role_id| role_id == "owner")
            }))
    }

    fn persist_session(
        &self,
        user: &StoredUser,
        client_app_id: String,
    ) -> Result<SessionRecord, AppError> {
        let workspace = self.workspace_snapshot()?;
        let session = SessionRecord {
            id: format!("sess-{}", Uuid::new_v4()),
            workspace_id: workspace.id,
            user_id: user.record.id.clone(),
            client_app_id,
            token: Uuid::new_v4().to_string(),
            status: "active".into(),
            created_at: Self::now(),
            expires_at: None,
            role_ids: user.membership.role_ids.clone(),
            scope_project_ids: user.membership.scope_project_ids.clone(),
        };

        self.state
            .open_db()?
            .execute(
                "INSERT INTO sessions (id, workspace_id, user_id, client_app_id, token, status, created_at, expires_at, role_ids, scope_project_ids)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    session.id,
                    session.workspace_id,
                    session.user_id,
                    session.client_app_id,
                    session.token,
                    session.status,
                    session.created_at as i64,
                    session.expires_at.map(|value| value as i64),
                    serde_json::to_string(&session.role_ids)?,
                    serde_json::to_string(&session.scope_project_ids)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        self.state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .push(session.clone());

        Ok(session)
    }

    fn persist_avatar(
        &self,
        user_id: &str,
        avatar: &AvatarUploadPayload,
    ) -> Result<(String, String, u64, String), AppError> {
        let content_type = avatar.content_type.trim().to_ascii_lowercase();
        if !matches!(
            content_type.as_str(),
            "image/png" | "image/jpeg" | "image/jpg" | "image/webp"
        ) {
            return Err(AppError::invalid_input("avatar must be png, jpeg, or webp"));
        }
        if avatar.byte_size == 0 || avatar.byte_size > 2 * 1024 * 1024 {
            return Err(AppError::invalid_input("avatar must be 2 MiB or smaller"));
        }

        let bytes = BASE64_STANDARD
            .decode(&avatar.data_base64)
            .map_err(|_| AppError::invalid_input("avatar payload is invalid"))?;
        if bytes.len() as u64 != avatar.byte_size {
            return Err(AppError::invalid_input("avatar byte size mismatch"));
        }

        let extension = match content_type.as_str() {
            "image/png" => "png",
            "image/webp" => "webp",
            _ => "jpg",
        };
        let relative_path = format!("data/blobs/avatars/{user_id}.{extension}");
        let absolute_path = self.state.paths.root.join(&relative_path);
        fs::write(&absolute_path, &bytes)?;

        Ok((
            relative_path,
            content_type,
            avatar.byte_size,
            content_hash(&bytes),
        ))
    }
}

#[async_trait]
impl WorkspaceService for InfraWorkspaceService {
    async fn system_bootstrap(&self) -> Result<SystemBootstrapStatus, AppError> {
        let workspace = self.state.workspace_snapshot()?;
        let owner_ready = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("workspace users mutex poisoned"))?
            .iter()
            .any(|user| {
                user.membership
                    .role_ids
                    .iter()
                    .any(|role_id| role_id == "owner")
            });
        Ok(SystemBootstrapStatus {
            workspace: workspace.clone(),
            setup_required: !owner_ready && workspace.bootstrap_status == "setup_required",
            owner_ready,
            registered_apps: self
                .state
                .apps
                .lock()
                .map_err(|_| AppError::runtime("app registry mutex poisoned"))?
                .clone(),
            protocol_version: "2026-04-06".into(),
            api_base_path: "/api/v1".into(),
            transport_security: "loopback".into(),
            auth_mode: "session-token".into(),
            capabilities: octopus_core::WorkspaceCapabilitySet {
                polling: true,
                sse: true,
                idempotency: true,
                reconnect: true,
                event_replay: true,
            },
        })
    }

    async fn workspace_summary(&self) -> Result<WorkspaceSummary, AppError> {
        self.state.workspace_snapshot()
    }

    async fn list_projects(&self) -> Result<Vec<ProjectRecord>, AppError> {
        Ok(self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?
            .clone())
    }

    async fn create_project(
        &self,
        request: CreateProjectRequest,
    ) -> Result<ProjectRecord, AppError> {
        let record = ProjectRecord {
            id: format!("proj-{}", Uuid::new_v4()),
            workspace_id: self.state.workspace_id()?,
            name: Self::normalize_project_name(&request.name)?,
            status: "active".into(),
            description: Self::normalize_project_description(&request.description),
            assignments: request.assignments,
        };
        let assignments_json = record
            .assignments
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        self.state.open_db()?.execute(
            "INSERT INTO projects (id, workspace_id, name, status, description, assignments_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                record.id,
                record.workspace_id,
                record.name,
                record.status,
                record.description,
                assignments_json,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut projects = self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?;
        projects.push(record.clone());
        Ok(record)
    }

    async fn update_project(
        &self,
        project_id: &str,
        request: UpdateProjectRequest,
    ) -> Result<ProjectRecord, AppError> {
        let mut projects = self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?;
        let existing = projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("project not found"))?;

        let updated = ProjectRecord {
            id: project_id.into(),
            workspace_id: existing.workspace_id.clone(),
            name: Self::normalize_project_name(&request.name)?,
            status: Self::normalize_project_status(&request.status)?,
            description: Self::normalize_project_description(&request.description),
            assignments: request.assignments,
        };

        if existing.status != "archived" && updated.status == "archived" {
            let active_count = projects
                .iter()
                .filter(|project| project.status == "active")
                .count();
            if active_count <= 1 {
                return Err(AppError::invalid_input(
                    "cannot archive the last active project",
                ));
            }
        }

        let assignments_json = updated
            .assignments
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO projects (id, workspace_id, name, status, description, assignments_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                updated.id,
                updated.workspace_id,
                updated.name,
                updated.status,
                updated.description,
                assignments_json,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        Self::replace_or_push(&mut projects, updated.clone(), |item| item.id == project_id);

        if existing.status != updated.status && updated.status == "archived" {
            let mut workspace = self
                .state
                .workspace
                .lock()
                .map_err(|_| AppError::runtime("workspace mutex poisoned"))?;
            if workspace.default_project_id == project_id {
                workspace.default_project_id = Self::next_active_project_id(&projects, project_id)
                    .ok_or_else(|| {
                        AppError::invalid_input("cannot archive the last active project")
                    })?;
                save_workspace_config_file(&self.state.paths.workspace_config, &workspace)?;
            }
        }

        Ok(updated)
    }

    async fn list_workspace_resources(&self) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        Ok(self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?
            .clone())
    }

    async fn list_project_resources(
        &self,
        project_id: &str,
    ) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        Ok(self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id.as_deref() == Some(project_id))
            .cloned()
            .collect())
    }

    async fn create_workspace_resource(
        &self,
        workspace_id: &str,
        input: CreateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let record = WorkspaceResourceRecord {
            id: format!("res-{}", Uuid::new_v4()),
            workspace_id: workspace_id.to_string(),
            project_id: input.project_id,
            kind: input.kind,
            name: input.name,
            location: input.location,
            origin: "source".to_string(),
            status: "healthy".to_string(),
            updated_at: timestamp_now(),
            tags: input.tags,
            source_artifact_id: input.source_artifact_id,
        };

        let conn = self.state.open_db()?;
        conn.execute(
            "INSERT INTO resources (id, workspace_id, project_id, kind, name, location, origin, status, updated_at, tags, source_artifact_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.kind,
                record.name,
                record.location,
                record.origin,
                record.status,
                record.updated_at as i64,
                serde_json::to_string(&record.tags)?,
                record.source_artifact_id,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

        let mut resources = self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?;
        resources.push(record.clone());
        Ok(record)
    }

    async fn create_project_resource(
        &self,
        project_id: &str,
        input: CreateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let mut input = input;
        input.project_id = Some(project_id.to_string());
        let workspace_id = self
            .state
            .workspace
            .lock()
            .map_err(|_| AppError::runtime("workspace mutex poisoned"))?
            .id
            .clone();
        self.create_workspace_resource(&workspace_id, input).await
    }

    async fn create_project_resource_folder(
        &self,
        project_id: &str,
        input: CreateWorkspaceResourceFolderInput,
    ) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        let mut results = Vec::new();
        for entry in input.files {
            let folder_path = std::path::Path::new(&entry.relative_path);
            let name = folder_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&entry.file_name)
                .to_string();
            let location = folder_path
                .parent()
                .map(|p| p.to_string_lossy().to_string());

            let file_input = CreateWorkspaceResourceInput {
                project_id: Some(project_id.to_string()),
                kind: if entry.relative_path.ends_with('/') || entry.byte_size == 0 {
                    "folder".to_string()
                } else {
                    "file".to_string()
                },
                name,
                location,
                tags: vec![],
                source_artifact_id: None,
            };

            let record = self.create_project_resource(project_id, file_input).await?;
            results.push(record);
        }
        Ok(results)
    }

    async fn delete_workspace_resource(
        &self,
        workspace_id: &str,
        resource_id: &str,
    ) -> Result<(), AppError> {
        let conn = self.state.open_db()?;
        let affected = conn
            .execute(
                "DELETE FROM resources WHERE id = ?1 AND workspace_id = ?2",
                params![resource_id, workspace_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        if affected == 0 {
            return Err(AppError::not_found(format!(
                "resource {} not found in workspace {}",
                resource_id, workspace_id
            )));
        }

        let mut resources = self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?;
        resources.retain(|r| !(r.id == resource_id && r.workspace_id == workspace_id));
        Ok(())
    }

    async fn delete_project_resource(
        &self,
        project_id: &str,
        resource_id: &str,
    ) -> Result<(), AppError> {
        let conn = self.state.open_db()?;
        let affected = conn
            .execute(
                "DELETE FROM resources WHERE id = ?1 AND project_id = ?2",
                params![resource_id, project_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        if affected == 0 {
            return Err(AppError::not_found(format!(
                "resource {} not found in project {}",
                resource_id, project_id
            )));
        }

        let mut resources = self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?;
        resources.retain(|r| !(r.id == resource_id && r.project_id.as_deref() == Some(project_id)));
        Ok(())
    }

    async fn update_workspace_resource(
        &self,
        workspace_id: &str,
        resource_id: &str,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let mut resources = self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?;

        let record = resources
            .iter_mut()
            .find(|r| r.id == resource_id && r.workspace_id == workspace_id)
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "resource {} not found in workspace {}",
                    resource_id, workspace_id
                ))
            })?;

        if let Some(name) = input.name {
            record.name = name;
        }
        if let Some(location) = input.location {
            record.location = Some(location);
        }
        if let Some(status) = input.status {
            record.status = status;
        }
        if let Some(tags) = input.tags {
            record.tags = tags;
        }
        record.updated_at = timestamp_now();

        let conn = self.state.open_db()?;
        conn.execute(
            "UPDATE resources SET name = ?1, location = ?2, status = ?3, updated_at = ?4, tags = ?5 WHERE id = ?6 AND workspace_id = ?7",
            params![
                record.name,
                record.location,
                record.status,
                record.updated_at as i64,
                serde_json::to_string(&record.tags)?,
                resource_id,
                workspace_id,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

        Ok(record.clone())
    }

    async fn update_project_resource(
        &self,
        project_id: &str,
        resource_id: &str,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let mut resources = self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?;

        let record = resources
            .iter_mut()
            .find(|r| r.id == resource_id && r.project_id.as_deref() == Some(project_id))
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "resource {} not found in project {}",
                    resource_id, project_id
                ))
            })?;

        if let Some(name) = input.name {
            record.name = name;
        }
        if let Some(location) = input.location {
            record.location = Some(location);
        }
        if let Some(status) = input.status {
            record.status = status;
        }
        if let Some(tags) = input.tags {
            record.tags = tags;
        }
        record.updated_at = timestamp_now();

        let conn = self.state.open_db()?;
        conn.execute(
            "UPDATE resources SET name = ?1, location = ?2, status = ?3, updated_at = ?4, tags = ?5 WHERE id = ?6 AND project_id = ?7",
            params![
                record.name,
                record.location,
                record.status,
                record.updated_at as i64,
                serde_json::to_string(&record.tags)?,
                resource_id,
                project_id,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

        Ok(record.clone())
    }

    async fn list_workspace_knowledge(&self) -> Result<Vec<KnowledgeRecord>, AppError> {
        Ok(self
            .state
            .knowledge_records
            .lock()
            .map_err(|_| AppError::runtime("knowledge mutex poisoned"))?
            .clone())
    }

    async fn list_project_knowledge(
        &self,
        project_id: &str,
    ) -> Result<Vec<KnowledgeRecord>, AppError> {
        Ok(self
            .state
            .knowledge_records
            .lock()
            .map_err(|_| AppError::runtime("knowledge mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id.as_deref() == Some(project_id))
            .cloned()
            .collect())
    }

    async fn get_workspace_pet_snapshot(&self) -> Result<PetWorkspaceSnapshot, AppError> {
        self.workspace_pet_snapshot()
    }

    async fn get_project_pet_snapshot(
        &self,
        project_id: &str,
    ) -> Result<PetWorkspaceSnapshot, AppError> {
        self.project_pet_snapshot(project_id)
    }

    async fn save_workspace_pet_presence(
        &self,
        input: SavePetPresenceInput,
    ) -> Result<PetPresenceState, AppError> {
        let mut presence = self
            .state
            .workspace_pet_presence
            .lock()
            .map_err(|_| AppError::runtime("workspace pet presence mutex poisoned"))?
            .clone();
        if !input.pet_id.trim().is_empty() {
            presence.pet_id = input.pet_id;
        }
        if let Some(value) = input.is_visible {
            presence.is_visible = value;
        }
        if let Some(value) = input.chat_open {
            presence.chat_open = value;
        }
        if let Some(value) = input.motion_state {
            presence.motion_state = value;
        }
        if let Some(value) = input.unread_count {
            presence.unread_count = value;
        }
        if let Some(value) = input.last_interaction_at {
            presence.last_interaction_at = value;
        }
        if let Some(value) = input.position {
            presence.position = value;
        }
        self.persist_pet_presence(None, &presence)?;
        *self
            .state
            .workspace_pet_presence
            .lock()
            .map_err(|_| AppError::runtime("workspace pet presence mutex poisoned"))? =
            presence.clone();
        Ok(presence)
    }

    async fn save_project_pet_presence(
        &self,
        project_id: &str,
        input: SavePetPresenceInput,
    ) -> Result<PetPresenceState, AppError> {
        self.ensure_project_exists(project_id)?;
        let mut presences = self
            .state
            .project_pet_presences
            .lock()
            .map_err(|_| AppError::runtime("project pet presences mutex poisoned"))?;
        let mut presence = presences
            .iter()
            .find(|(id, _)| id == project_id)
            .map(|(_, presence)| presence.clone())
            .unwrap_or_else(default_workspace_pet_presence);
        if !input.pet_id.trim().is_empty() {
            presence.pet_id = input.pet_id;
        }
        if let Some(value) = input.is_visible {
            presence.is_visible = value;
        }
        if let Some(value) = input.chat_open {
            presence.chat_open = value;
        }
        if let Some(value) = input.motion_state {
            presence.motion_state = value;
        }
        if let Some(value) = input.unread_count {
            presence.unread_count = value;
        }
        if let Some(value) = input.last_interaction_at {
            presence.last_interaction_at = value;
        }
        if let Some(value) = input.position {
            presence.position = value;
        }
        self.persist_pet_presence(Some(project_id), &presence)?;
        Self::replace_or_push(
            &mut presences,
            (project_id.to_string(), presence.clone()),
            |item| item.0 == project_id,
        );
        Ok(presence)
    }

    async fn bind_workspace_pet_conversation(
        &self,
        input: BindPetConversationInput,
    ) -> Result<PetConversationBinding, AppError> {
        let binding = PetConversationBinding {
            pet_id: if input.pet_id.trim().is_empty() {
                "pet-octopus".into()
            } else {
                input.pet_id
            },
            workspace_id: self.state.workspace_id()?,
            project_id: String::new(),
            conversation_id: input.conversation_id,
            session_id: input.session_id,
            updated_at: Self::now(),
        };
        self.persist_pet_binding(None, &binding)?;
        *self
            .state
            .workspace_pet_binding
            .lock()
            .map_err(|_| AppError::runtime("workspace pet binding mutex poisoned"))? =
            Some(binding.clone());
        Ok(binding)
    }

    async fn bind_project_pet_conversation(
        &self,
        project_id: &str,
        input: BindPetConversationInput,
    ) -> Result<PetConversationBinding, AppError> {
        self.ensure_project_exists(project_id)?;
        let binding = PetConversationBinding {
            pet_id: if input.pet_id.trim().is_empty() {
                "pet-octopus".into()
            } else {
                input.pet_id
            },
            workspace_id: self.state.workspace_id()?,
            project_id: project_id.into(),
            conversation_id: input.conversation_id,
            session_id: input.session_id,
            updated_at: Self::now(),
        };
        self.persist_pet_binding(Some(project_id), &binding)?;
        let mut bindings = self
            .state
            .project_pet_bindings
            .lock()
            .map_err(|_| AppError::runtime("project pet bindings mutex poisoned"))?;
        Self::replace_or_push(
            &mut bindings,
            (project_id.to_string(), binding.clone()),
            |item| item.0 == project_id,
        );
        Ok(binding)
    }

    async fn list_agents(&self) -> Result<Vec<AgentRecord>, AppError> {
        Ok(self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))?
            .clone())
    }

    async fn create_agent(&self, input: UpsertAgentInput) -> Result<AgentRecord, AppError> {
        let agent_id = format!("agent-{}", Uuid::new_v4());
        let record = self.build_agent_record(&agent_id, input, None)?;

        self.state.open_db()?.execute(
            "INSERT INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut agents = self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))?;
        agents.push(record.clone());
        Ok(record)
    }

    async fn update_agent(
        &self,
        agent_id: &str,
        input: UpsertAgentInput,
    ) -> Result<AgentRecord, AppError> {
        let current = {
            self.state
                .agents
                .lock()
                .map_err(|_| AppError::runtime("agents mutex poisoned"))?
                .iter()
                .find(|item| item.id == agent_id)
                .cloned()
        };
        let record = self.build_agent_record(agent_id, input, current.as_ref())?;

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        if let Some(previous) = current.as_ref() {
            if previous.avatar_path != record.avatar_path {
                self.remove_avatar_file(previous.avatar_path.as_deref())?;
            }
        }

        let mut agents = self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))?;
        Self::replace_or_push(&mut agents, record.clone(), |item| item.id == agent_id);
        Ok(record)
    }

    async fn delete_agent(&self, agent_id: &str) -> Result<(), AppError> {
        let removed = {
            let mut agents = self
                .state
                .agents
                .lock()
                .map_err(|_| AppError::runtime("agents mutex poisoned"))?;
            let existing = agents.iter().find(|item| item.id == agent_id).cloned();
            agents.retain(|item| item.id != agent_id);
            existing
        };

        let connection = self.state.open_db()?;
        connection
            .execute("DELETE FROM agents WHERE id = ?1", params![agent_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM project_agent_links WHERE agent_id = ?1",
                params![agent_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .project_agent_links
            .lock()
            .map_err(|_| AppError::runtime("project agent links mutex poisoned"))?
            .retain(|item| item.agent_id != agent_id);

        if let Some(record) = removed {
            self.remove_avatar_file(record.avatar_path.as_deref())?;
        }
        Ok(())
    }

    async fn preview_import_agent_bundle(
        &self,
        input: ImportWorkspaceAgentBundlePreviewInput,
    ) -> Result<ImportWorkspaceAgentBundlePreview, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        agent_import::preview_agent_bundle_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            input,
        )
    }

    async fn import_agent_bundle(
        &self,
        input: ImportWorkspaceAgentBundleInput,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let result = agent_import::execute_agent_bundle_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            input,
        )?;
        let next_agents = load_agents(&connection)?;
        *self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))? = next_agents;
        Ok(result)
    }

    async fn list_project_agent_links(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectAgentLinkRecord>, AppError> {
        Ok(self
            .state
            .project_agent_links
            .lock()
            .map_err(|_| AppError::runtime("project agent links mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id)
            .cloned()
            .collect())
    }

    async fn link_project_agent(
        &self,
        input: ProjectAgentLinkInput,
    ) -> Result<ProjectAgentLinkRecord, AppError> {
        let record = ProjectAgentLinkRecord {
            workspace_id: self.state.workspace_id()?,
            project_id: input.project_id,
            agent_id: input.agent_id,
            linked_at: Self::now(),
        };
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO project_agent_links (workspace_id, project_id, agent_id, linked_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![record.workspace_id, record.project_id, record.agent_id, record.linked_at as i64],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut links = self
            .state
            .project_agent_links
            .lock()
            .map_err(|_| AppError::runtime("project agent links mutex poisoned"))?;
        Self::replace_or_push(&mut links, record.clone(), |item| {
            item.project_id == record.project_id && item.agent_id == record.agent_id
        });
        Ok(record)
    }

    async fn unlink_project_agent(&self, project_id: &str, agent_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM project_agent_links WHERE project_id = ?1 AND agent_id = ?2",
                params![project_id, agent_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .project_agent_links
            .lock()
            .map_err(|_| AppError::runtime("project agent links mutex poisoned"))?
            .retain(|item| !(item.project_id == project_id && item.agent_id == agent_id));
        Ok(())
    }

    async fn list_teams(&self) -> Result<Vec<TeamRecord>, AppError> {
        Ok(self
            .state
            .teams
            .lock()
            .map_err(|_| AppError::runtime("teams mutex poisoned"))?
            .clone())
    }

    async fn create_team(&self, input: UpsertTeamInput) -> Result<TeamRecord, AppError> {
        let team_id = format!("team-{}", Uuid::new_v4());
        let record = self.build_team_record(&team_id, input, None)?;

        write_team_record(&self.state.open_db()?, &record, false)?;

        let mut teams = self
            .state
            .teams
            .lock()
            .map_err(|_| AppError::runtime("teams mutex poisoned"))?;
        teams.push(record.clone());
        Ok(record)
    }

    async fn update_team(
        &self,
        team_id: &str,
        input: UpsertTeamInput,
    ) -> Result<TeamRecord, AppError> {
        let current = {
            self.state
                .teams
                .lock()
                .map_err(|_| AppError::runtime("teams mutex poisoned"))?
                .iter()
                .find(|item| item.id == team_id)
                .cloned()
        };
        let record = self.build_team_record(team_id, input, current.as_ref())?;

        write_team_record(&self.state.open_db()?, &record, true)?;

        if let Some(previous) = current.as_ref() {
            if previous.avatar_path != record.avatar_path {
                self.remove_avatar_file(previous.avatar_path.as_deref())?;
            }
        }

        let mut teams = self
            .state
            .teams
            .lock()
            .map_err(|_| AppError::runtime("teams mutex poisoned"))?;
        Self::replace_or_push(&mut teams, record.clone(), |item| item.id == team_id);
        Ok(record)
    }

    async fn delete_team(&self, team_id: &str) -> Result<(), AppError> {
        let removed = {
            let mut teams = self
                .state
                .teams
                .lock()
                .map_err(|_| AppError::runtime("teams mutex poisoned"))?;
            let existing = teams.iter().find(|item| item.id == team_id).cloned();
            teams.retain(|item| item.id != team_id);
            existing
        };

        let connection = self.state.open_db()?;
        connection
            .execute("DELETE FROM teams WHERE id = ?1", params![team_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM project_team_links WHERE team_id = ?1",
                params![team_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .project_team_links
            .lock()
            .map_err(|_| AppError::runtime("project team links mutex poisoned"))?
            .retain(|item| item.team_id != team_id);

        if let Some(record) = removed {
            self.remove_avatar_file(record.avatar_path.as_deref())?;
        }
        Ok(())
    }

    async fn list_project_team_links(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectTeamLinkRecord>, AppError> {
        Ok(self
            .state
            .project_team_links
            .lock()
            .map_err(|_| AppError::runtime("project team links mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id)
            .cloned()
            .collect())
    }

    async fn link_project_team(
        &self,
        input: ProjectTeamLinkInput,
    ) -> Result<ProjectTeamLinkRecord, AppError> {
        let record = ProjectTeamLinkRecord {
            workspace_id: self.state.workspace_id()?,
            project_id: input.project_id,
            team_id: input.team_id,
            linked_at: Self::now(),
        };
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO project_team_links (workspace_id, project_id, team_id, linked_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![record.workspace_id, record.project_id, record.team_id, record.linked_at as i64],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut links = self
            .state
            .project_team_links
            .lock()
            .map_err(|_| AppError::runtime("project team links mutex poisoned"))?;
        Self::replace_or_push(&mut links, record.clone(), |item| {
            item.project_id == record.project_id && item.team_id == record.team_id
        });
        Ok(record)
    }

    async fn unlink_project_team(&self, project_id: &str, team_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM project_team_links WHERE project_id = ?1 AND team_id = ?2",
                params![project_id, team_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .project_team_links
            .lock()
            .map_err(|_| AppError::runtime("project team links mutex poisoned"))?
            .retain(|item| !(item.project_id == project_id && item.team_id == team_id));
        Ok(())
    }

    async fn list_models(&self) -> Result<Vec<ModelCatalogRecord>, AppError> {
        Ok(self
            .state
            .model_catalog
            .lock()
            .map_err(|_| AppError::runtime("model catalog mutex poisoned"))?
            .clone())
    }

    async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialRecord>, AppError> {
        Ok(self
            .state
            .provider_credentials
            .lock()
            .map_err(|_| AppError::runtime("provider credentials mutex poisoned"))?
            .clone())
    }

    async fn get_tool_catalog(&self) -> Result<WorkspaceToolCatalogSnapshot, AppError> {
        self.build_tool_catalog().await
    }

    async fn set_tool_catalog_disabled(
        &self,
        patch: WorkspaceToolDisablePatch,
    ) -> Result<WorkspaceToolCatalogSnapshot, AppError> {
        let snapshot = self.build_tool_catalog().await?;
        if !snapshot
            .entries
            .iter()
            .any(|entry| entry.source_key == patch.source_key)
        {
            return Err(AppError::not_found("workspace tool catalog entry"));
        }

        let mut document = load_workspace_runtime_document(&self.state.paths)?;
        let mut disabled_keys = disabled_source_keys(&document);
        if patch.disabled {
            disabled_keys.insert(patch.source_key);
        } else {
            disabled_keys.remove(&patch.source_key);
        }
        set_disabled_source_keys(&mut document, &disabled_keys)?;
        self.save_workspace_runtime_document(document)?;
        self.build_tool_catalog().await
    }

    async fn get_workspace_skill(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        self.get_workspace_skill_document(skill_id)
    }

    async fn create_workspace_skill(
        &self,
        input: CreateWorkspaceSkillInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let slug = validate_skill_slug(&input.slug)?;
        let skill_dir = workspace_owned_skill_root(&self.state.paths).join(&slug);
        if skill_dir.exists() {
            return Err(AppError::conflict(format!(
                "workspace skill '{slug}' already exists"
            )));
        }
        fs::create_dir_all(&skill_dir)?;
        let skill_path = skill_dir.join("SKILL.md");
        fs::write(&skill_path, input.content)?;
        skill_document_from_path(
            &self.state.paths.root,
            &skill_path,
            SkillSourceOrigin::SkillsDir,
        )
    }

    async fn update_workspace_skill(
        &self,
        skill_id: &str,
        input: UpdateWorkspaceSkillInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let entry = self.ensure_workspace_owned_skill_entry(skill_id)?;
        fs::write(&entry.path, input.content)?;
        skill_document_from_path(&self.state.paths.root, &entry.path, entry.origin)
    }

    async fn get_workspace_skill_tree(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillTreeDocument, AppError> {
        self.get_workspace_skill_tree_document(skill_id)
    }

    async fn get_workspace_skill_file(
        &self,
        skill_id: &str,
        relative_path: &str,
    ) -> Result<WorkspaceSkillFileDocument, AppError> {
        self.get_workspace_skill_file_document(skill_id, relative_path)
    }

    async fn update_workspace_skill_file(
        &self,
        skill_id: &str,
        relative_path: &str,
        input: UpdateWorkspaceSkillFileInput,
    ) -> Result<WorkspaceSkillFileDocument, AppError> {
        let entry = self.ensure_workspace_owned_skill_entry(skill_id)?;
        let skill_root = skill_root_path(&entry.path, entry.origin)?;
        let path = resolve_skill_file_path(&skill_root, entry.origin, relative_path)?;
        if !path.exists() {
            return Err(AppError::not_found("workspace skill file"));
        }
        let existing = self.get_workspace_skill_file_document(skill_id, relative_path)?;
        if !existing.is_text {
            return Err(AppError::invalid_input(
                "binary skill files cannot be edited in the workspace tool page",
            ));
        }
        fs::write(&path, input.content)?;
        skill_file_document_from_path(
            &self.state.paths.root,
            skill_id,
            &skill_source_key(&entry.path, &self.state.paths.root),
            &skill_root,
            entry.origin,
            &path,
            false,
        )
    }

    async fn copy_workspace_skill_to_managed(
        &self,
        skill_id: &str,
        input: CopyWorkspaceSkillToManagedInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let entry = self.find_skill_catalog_entry(skill_id)?;
        let source_root = skill_root_path(&entry.path, entry.origin)?;
        let files = match entry.origin {
            SkillSourceOrigin::SkillsDir => {
                let mut collected = Vec::new();
                for node in build_skill_tree(&source_root, entry.origin)? {
                    collect_tree_files(&source_root, &node, &mut collected)?;
                }
                collected
            }
            SkillSourceOrigin::LegacyCommandsDir => vec![(
                source_root
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                fs::read(&source_root)?,
            )],
        };
        self.import_skill_files_to_managed_root(&input.slug, files)
    }

    async fn import_workspace_skill_archive(
        &self,
        input: ImportWorkspaceSkillArchiveInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let files = extract_archive_entries(&input)?;
        self.import_skill_files_to_managed_root(&input.slug, files)
    }

    async fn import_workspace_skill_folder(
        &self,
        input: ImportWorkspaceSkillFolderInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let files = normalize_uploaded_files(&input.files)?;
        self.import_skill_files_to_managed_root(&input.slug, files)
    }

    async fn delete_workspace_skill(&self, skill_id: &str) -> Result<(), AppError> {
        let entry = self.ensure_workspace_owned_skill_entry(skill_id)?;
        let skill_dir = entry
            .path
            .parent()
            .ok_or_else(|| AppError::invalid_input("workspace skill path is invalid"))?;
        fs::remove_dir_all(skill_dir)?;
        Ok(())
    }

    async fn get_workspace_mcp_server(
        &self,
        server_name: &str,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        self.load_mcp_server_document(server_name)
    }

    async fn create_workspace_mcp_server(
        &self,
        input: UpsertWorkspaceMcpServerInput,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        if input.server_name.trim().is_empty() {
            return Err(AppError::invalid_input("serverName is required"));
        }
        let mut document = load_workspace_runtime_document(&self.state.paths)?;
        let servers = ensure_top_level_object(&mut document, "mcpServers")?;
        if servers.contains_key(&input.server_name) {
            return Err(AppError::conflict(format!(
                "mcp server '{}' already exists",
                input.server_name
            )));
        }
        let config =
            input.config.as_object().cloned().ok_or_else(|| {
                AppError::invalid_input("mcp server config must be a JSON object")
            })?;
        servers.insert(input.server_name.clone(), serde_json::Value::Object(config));
        self.save_workspace_runtime_document(document)?;
        self.load_mcp_server_document(&input.server_name)
    }

    async fn update_workspace_mcp_server(
        &self,
        server_name: &str,
        input: UpsertWorkspaceMcpServerInput,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        if input.server_name != server_name {
            return Err(AppError::invalid_input(
                "serverName in body must match the route parameter",
            ));
        }
        let mut document = load_workspace_runtime_document(&self.state.paths)?;
        let servers = ensure_top_level_object(&mut document, "mcpServers")?;
        if !servers.contains_key(server_name) {
            return Err(AppError::not_found("workspace mcp server"));
        }
        let config =
            input.config.as_object().cloned().ok_or_else(|| {
                AppError::invalid_input("mcp server config must be a JSON object")
            })?;
        servers.insert(server_name.into(), serde_json::Value::Object(config));
        self.save_workspace_runtime_document(document)?;
        self.load_mcp_server_document(server_name)
    }

    async fn delete_workspace_mcp_server(&self, server_name: &str) -> Result<(), AppError> {
        let mut document = load_workspace_runtime_document(&self.state.paths)?;
        let servers = ensure_top_level_object(&mut document, "mcpServers")?;
        if servers.remove(server_name).is_none() {
            return Err(AppError::not_found("workspace mcp server"));
        }
        self.save_workspace_runtime_document(document)?;
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<ToolRecord>, AppError> {
        Ok(self
            .state
            .tools
            .lock()
            .map_err(|_| AppError::runtime("tools mutex poisoned"))?
            .clone())
    }

    async fn create_tool(&self, mut record: ToolRecord) -> Result<ToolRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("tool-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }
        record.updated_at = Self::now();

        self.state.open_db()?.execute(
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
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut tools = self
            .state
            .tools
            .lock()
            .map_err(|_| AppError::runtime("tools mutex poisoned"))?;
        tools.push(record.clone());
        Ok(record)
    }

    async fn update_tool(
        &self,
        tool_id: &str,
        mut record: ToolRecord,
    ) -> Result<ToolRecord, AppError> {
        record.id = tool_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }
        record.updated_at = Self::now();

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO tools (id, workspace_id, kind, name, description, status, permission_mode, updated_at)
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
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut tools = self
            .state
            .tools
            .lock()
            .map_err(|_| AppError::runtime("tools mutex poisoned"))?;
        Self::replace_or_push(&mut tools, record.clone(), |item| item.id == tool_id);
        Ok(record)
    }

    async fn delete_tool(&self, tool_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute("DELETE FROM tools WHERE id = ?1", params![tool_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .tools
            .lock()
            .map_err(|_| AppError::runtime("tools mutex poisoned"))?
            .retain(|item| item.id != tool_id);
        Ok(())
    }

    async fn list_automations(&self) -> Result<Vec<AutomationRecord>, AppError> {
        Ok(self
            .state
            .automations
            .lock()
            .map_err(|_| AppError::runtime("automations mutex poisoned"))?
            .clone())
    }

    async fn create_automation(
        &self,
        mut record: AutomationRecord,
    ) -> Result<AutomationRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("automation-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }

        self.state.open_db()?.execute(
            "INSERT INTO automations (id, workspace_id, project_id, title, description, cadence, owner_type, owner_id, status, next_run_at, last_run_at, output)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.title,
                record.description,
                record.cadence,
                record.owner_type,
                record.owner_id,
                record.status,
                record.next_run_at.map(|value| value as i64),
                record.last_run_at.map(|value| value as i64),
                record.output,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut automations = self
            .state
            .automations
            .lock()
            .map_err(|_| AppError::runtime("automations mutex poisoned"))?;
        automations.push(record.clone());
        Ok(record)
    }

    async fn update_automation(
        &self,
        automation_id: &str,
        mut record: AutomationRecord,
    ) -> Result<AutomationRecord, AppError> {
        record.id = automation_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO automations (id, workspace_id, project_id, title, description, cadence, owner_type, owner_id, status, next_run_at, last_run_at, output)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.title,
                record.description,
                record.cadence,
                record.owner_type,
                record.owner_id,
                record.status,
                record.next_run_at.map(|value| value as i64),
                record.last_run_at.map(|value| value as i64),
                record.output,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut automations = self
            .state
            .automations
            .lock()
            .map_err(|_| AppError::runtime("automations mutex poisoned"))?;
        Self::replace_or_push(&mut automations, record.clone(), |item| {
            item.id == automation_id
        });
        Ok(record)
    }

    async fn delete_automation(&self, automation_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM automations WHERE id = ?1",
                params![automation_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .automations
            .lock()
            .map_err(|_| AppError::runtime("automations mutex poisoned"))?
            .retain(|item| item.id != automation_id);
        Ok(())
    }

    async fn list_users(&self) -> Result<Vec<UserRecordSummary>, AppError> {
        Ok(self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .iter()
            .map(|user| to_user_summary(&self.state.paths, user))
            .collect())
    }

    async fn create_user(
        &self,
        request: CreateWorkspaceUserRequest,
    ) -> Result<UserRecordSummary, AppError> {
        self.validate_workspace_user_identity(&request.username, &request.display_name, None)?;

        let user_id = format!("user-{}", Uuid::new_v4());
        let now = Self::now();
        let next_avatar = if request.use_default_avatar.unwrap_or(false) {
            (None, None, None, None)
        } else if let Some(avatar) = request.avatar.as_ref() {
            let (avatar_path, avatar_content_type, avatar_byte_size, avatar_content_hash) =
                self.persist_avatar(&user_id, avatar)?;
            (
                Some(avatar_path),
                Some(avatar_content_type),
                Some(avatar_byte_size),
                Some(avatar_content_hash),
            )
        } else {
            (None, None, None, None)
        };
        let (password_hash, password_state) = self.resolve_member_password(
            request.password.as_deref(),
            request.confirm_password.as_deref(),
            request.use_default_password.unwrap_or(false),
        )?;

        let user_record = UserRecord {
            id: user_id.clone(),
            username: request.username.trim().to_string(),
            display_name: request.display_name.trim().to_string(),
            avatar_path: next_avatar.0.clone(),
            avatar_content_type: next_avatar.1.clone(),
            avatar_byte_size: next_avatar.2,
            avatar_content_hash: next_avatar.3.clone(),
            status: request.status.clone(),
            password_state: password_state.clone(),
            created_at: now,
            updated_at: now,
        };
        let membership = WorkspaceMembershipRecord {
            workspace_id: self.state.workspace_id()?,
            user_id: user_id.clone(),
            role_ids: request.role_ids.clone(),
            scope_mode: if request.scope_project_ids.is_empty() {
                "all-projects".into()
            } else {
                "selected-projects".into()
            },
            scope_project_ids: request.scope_project_ids.clone(),
        };

        self.state.open_db()?.execute(
            "INSERT INTO users (id, username, display_name, avatar_path, avatar_content_type, avatar_byte_size, avatar_content_hash, status, password_hash, password_state, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                user_record.id,
                user_record.username,
                user_record.display_name,
                user_record.avatar_path,
                user_record.avatar_content_type,
                user_record.avatar_byte_size.map(|value| value as i64),
                user_record.avatar_content_hash,
                user_record.status,
                password_hash.clone(),
                user_record.password_state,
                user_record.created_at as i64,
                user_record.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO memberships (workspace_id, user_id, role_ids, scope_mode, scope_project_ids)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                membership.workspace_id,
                membership.user_id,
                serde_json::to_string(&membership.role_ids)?,
                membership.scope_mode,
                serde_json::to_string(&membership.scope_project_ids)?,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let stored_user = StoredUser {
            record: user_record,
            password_hash,
            membership,
        };
        let summary = to_user_summary(&self.state.paths, &stored_user);
        let mut users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?;
        users.push(stored_user);
        Ok(summary)
    }

    async fn update_user(
        &self,
        user_id: &str,
        request: UpdateWorkspaceUserRequest,
    ) -> Result<UserRecordSummary, AppError> {
        self.validate_workspace_user_identity(
            &request.username,
            &request.display_name,
            Some(user_id),
        )?;

        let current_user = {
            let users = self
                .state
                .users
                .lock()
                .map_err(|_| AppError::runtime("users mutex poisoned"))?;
            users
                .iter()
                .find(|user| user.record.id == user_id)
                .cloned()
                .ok_or_else(|| AppError::not_found("workspace user"))?
        };

        let next_avatar = if let Some(avatar) = request.avatar.as_ref() {
            let (avatar_path, avatar_content_type, avatar_byte_size, avatar_content_hash) =
                self.persist_avatar(user_id, avatar)?;
            (
                Some(avatar_path),
                Some(avatar_content_type),
                Some(avatar_byte_size),
                Some(avatar_content_hash),
            )
        } else if request.remove_avatar.unwrap_or(false) {
            (None, None, None, None)
        } else {
            (
                current_user.record.avatar_path.clone(),
                current_user.record.avatar_content_type.clone(),
                current_user.record.avatar_byte_size,
                current_user.record.avatar_content_hash.clone(),
            )
        };

        let (next_password_hash, next_password_state) =
            if request.reset_password_to_default.unwrap_or(false) {
                self.resolve_member_password(None, None, true)?
            } else if request.password.is_some() || request.confirm_password.is_some() {
                self.resolve_member_password(
                    request.password.as_deref(),
                    request.confirm_password.as_deref(),
                    false,
                )?
            } else {
                (
                    current_user.password_hash.clone(),
                    current_user.record.password_state.clone(),
                )
            };

        let now = Self::now();
        self.state
            .open_db()?
            .execute(
                "UPDATE users
             SET username = ?2,
                 display_name = ?3,
                 avatar_path = ?4,
                 avatar_content_type = ?5,
                 avatar_byte_size = ?6,
                 avatar_content_hash = ?7,
                 status = ?8,
                 password_hash = ?9,
                 password_state = ?10,
                 updated_at = ?11
             WHERE id = ?1",
                params![
                    user_id,
                    request.username.trim(),
                    request.display_name.trim(),
                    next_avatar.0.clone(),
                    next_avatar.1.clone(),
                    next_avatar.2.map(|value| value as i64),
                    next_avatar.3.clone(),
                    request.status.clone(),
                    next_password_hash.clone(),
                    next_password_state.clone(),
                    now as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO memberships (workspace_id, user_id, role_ids, scope_mode, scope_project_ids)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                self.state.workspace_id()?,
                user_id,
                serde_json::to_string(&request.role_ids)?,
                if request.scope_project_ids.is_empty() { "all-projects" } else { "selected-projects" },
                serde_json::to_string(&request.scope_project_ids)?,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?;
        let existing = users
            .iter_mut()
            .find(|item| item.record.id == user_id)
            .ok_or_else(|| AppError::not_found("workspace user"))?;
        existing.record.username = request.username.trim().to_string();
        existing.record.display_name = request.display_name.trim().to_string();
        existing.record.avatar_path = next_avatar.0.clone();
        existing.record.avatar_content_type = next_avatar.1.clone();
        existing.record.avatar_byte_size = next_avatar.2;
        existing.record.avatar_content_hash = next_avatar.3.clone();
        existing.record.status = request.status.clone();
        existing.record.password_state = next_password_state.clone();
        existing.record.updated_at = now;
        existing.password_hash = next_password_hash.clone();
        existing.membership.role_ids = request.role_ids.clone();
        existing.membership.scope_project_ids = request.scope_project_ids.clone();
        existing.membership.scope_mode = if request.scope_project_ids.is_empty() {
            "all-projects".into()
        } else {
            "selected-projects".into()
        };
        let updated = to_user_summary(&self.state.paths, existing);

        if current_user.record.avatar_path != next_avatar.0 {
            self.remove_avatar_file(current_user.record.avatar_path.as_deref())?;
        }

        Ok(updated)
    }

    async fn delete_user(&self, user_id: &str) -> Result<(), AppError> {
        let existing = {
            let users = self
                .state
                .users
                .lock()
                .map_err(|_| AppError::runtime("users mutex poisoned"))?;
            users
                .iter()
                .find(|user| user.record.id == user_id)
                .cloned()
                .ok_or_else(|| AppError::not_found("workspace user"))?
        };

        self.state
            .open_db()?
            .execute(
                "DELETE FROM memberships WHERE user_id = ?1",
                params![user_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .open_db()?
            .execute("DELETE FROM sessions WHERE user_id = ?1", params![user_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .open_db()?
            .execute("DELETE FROM users WHERE id = ?1", params![user_id])
            .map_err(|error| AppError::database(error.to_string()))?;

        self.state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .retain(|user| user.record.id != user_id);
        self.state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .retain(|session| session.user_id != user_id);
        self.remove_avatar_file(existing.record.avatar_path.as_deref())?;
        Ok(())
    }

    async fn update_current_user_profile(
        &self,
        user_id: &str,
        request: UpdateCurrentUserProfileRequest,
    ) -> Result<UserRecordSummary, AppError> {
        let username = request.username.trim();
        let display_name = request.display_name.trim();
        if username.is_empty() || display_name.is_empty() {
            return Err(AppError::invalid_input(
                "username and display name are required",
            ));
        }

        let current_user = {
            let users = self
                .state
                .users
                .lock()
                .map_err(|_| AppError::runtime("users mutex poisoned"))?;
            if users
                .iter()
                .any(|user| user.record.id != user_id && user.record.username == username)
            {
                return Err(AppError::conflict("username already exists"));
            }
            users
                .iter()
                .find(|user| user.record.id == user_id)
                .cloned()
                .ok_or_else(|| AppError::not_found("workspace user"))?
        };

        let next_avatar = if let Some(avatar) = request.avatar.as_ref() {
            let (avatar_path, avatar_content_type, avatar_byte_size, avatar_content_hash) =
                self.persist_avatar(user_id, avatar)?;
            (
                Some(avatar_path),
                Some(avatar_content_type),
                Some(avatar_byte_size),
                Some(avatar_content_hash),
            )
        } else if request.remove_avatar.unwrap_or(false) {
            (None, None, None, None)
        } else {
            (
                current_user.record.avatar_path.clone(),
                current_user.record.avatar_content_type.clone(),
                current_user.record.avatar_byte_size,
                current_user.record.avatar_content_hash.clone(),
            )
        };

        let now = Self::now();
        self.state
            .open_db()?
            .execute(
                "UPDATE users
                 SET username = ?2,
                     display_name = ?3,
                     avatar_path = ?4,
                     avatar_content_type = ?5,
                     avatar_byte_size = ?6,
                     avatar_content_hash = ?7,
                     updated_at = ?8
                 WHERE id = ?1",
                params![
                    user_id,
                    username,
                    display_name,
                    next_avatar.0.clone(),
                    next_avatar.1.clone(),
                    next_avatar.2.map(|value| value as i64),
                    next_avatar.3.clone(),
                    now as i64
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        let mut users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?;
        let user = users
            .iter_mut()
            .find(|item| item.record.id == user_id)
            .ok_or_else(|| AppError::not_found("workspace user"))?;
        user.record.username = username.to_string();
        user.record.display_name = display_name.to_string();
        user.record.avatar_path = next_avatar.0.clone();
        user.record.avatar_content_type = next_avatar.1.clone();
        user.record.avatar_byte_size = next_avatar.2;
        user.record.avatar_content_hash = next_avatar.3.clone();
        user.record.updated_at = now;

        if current_user.record.avatar_path != next_avatar.0 {
            self.remove_avatar_file(current_user.record.avatar_path.as_deref())?;
        }

        Ok(to_user_summary(&self.state.paths, user))
    }

    async fn change_current_user_password(
        &self,
        user_id: &str,
        request: ChangeCurrentUserPasswordRequest,
    ) -> Result<ChangeCurrentUserPasswordResponse, AppError> {
        if request.new_password.len() < 8 {
            return Err(AppError::invalid_input(
                "new password must be at least 8 characters",
            ));
        }
        if request.new_password != request.confirm_password {
            return Err(AppError::invalid_input(
                "password confirmation does not match",
            ));
        }
        if request.new_password == request.current_password {
            return Err(AppError::invalid_input(
                "new password must be different from current password",
            ));
        }

        let mut users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?;
        let user = users
            .iter_mut()
            .find(|item| item.record.id == user_id)
            .ok_or_else(|| AppError::not_found("workspace user"))?;
        if !verify_password(&request.current_password, &user.password_hash) {
            return Err(AppError::invalid_input("current password is incorrect"));
        }
        user.password_hash = hash_password(&request.new_password);
        user.record.password_state = "set".into();
        user.record.updated_at = Self::now();
        self.state
            .open_db()?
            .execute(
                "UPDATE users SET password_hash = ?2, password_state = ?3, updated_at = ?4 WHERE id = ?1",
                params![
                    user_id,
                    user.password_hash,
                    user.record.password_state,
                    user.record.updated_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(ChangeCurrentUserPasswordResponse {
            password_state: user.record.password_state.clone(),
        })
    }

    async fn list_roles(&self) -> Result<Vec<RoleRecord>, AppError> {
        Ok(self
            .state
            .roles
            .lock()
            .map_err(|_| AppError::runtime("roles mutex poisoned"))?
            .clone())
    }

    async fn create_role(&self, mut record: RoleRecord) -> Result<RoleRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("role-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }

        self.state.open_db()?.execute(
            "INSERT INTO roles (id, workspace_id, name, code, description, status, permission_ids, menu_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.id,
                record.workspace_id,
                record.name,
                record.code,
                record.description,
                record.status,
                serde_json::to_string(&record.permission_ids)?,
                serde_json::to_string(&record.menu_ids)?,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        let mut roles = self
            .state
            .roles
            .lock()
            .map_err(|_| AppError::runtime("roles mutex poisoned"))?;
        roles.push(record.clone());
        Ok(record)
    }

    async fn update_role(
        &self,
        role_id: &str,
        mut record: RoleRecord,
    ) -> Result<RoleRecord, AppError> {
        record.id = role_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO roles (id, workspace_id, name, code, description, status, permission_ids, menu_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.id,
                record.workspace_id,
                record.name,
                record.code,
                record.description,
                record.status,
                serde_json::to_string(&record.permission_ids)?,
                serde_json::to_string(&record.menu_ids)?,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        let mut roles = self
            .state
            .roles
            .lock()
            .map_err(|_| AppError::runtime("roles mutex poisoned"))?;
        Self::replace_or_push(&mut roles, record.clone(), |item| item.id == role_id);
        Ok(record)
    }

    async fn delete_role(&self, role_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute("DELETE FROM roles WHERE id = ?1", params![role_id])
            .map_err(|error| AppError::database(error.to_string()))?;

        {
            let mut users = self
                .state
                .users
                .lock()
                .map_err(|_| AppError::runtime("users mutex poisoned"))?;
            for user in users.iter_mut() {
                if user.membership.role_ids.iter().any(|id| id == role_id) {
                    user.membership.role_ids.retain(|id| id != role_id);
                    self.state
                        .open_db()?
                        .execute(
                            "UPDATE memberships SET role_ids = ?2 WHERE workspace_id = ?1 AND user_id = ?3",
                            params![
                                self.state.workspace_id()?,
                                serde_json::to_string(&user.membership.role_ids)?,
                                user.record.id.clone(),
                            ],
                        )
                        .map_err(|error| AppError::database(error.to_string()))?;
                }
            }
        }

        {
            let mut sessions = self
                .state
                .sessions
                .lock()
                .map_err(|_| AppError::runtime("sessions mutex poisoned"))?;
            for session in sessions.iter_mut() {
                if session.role_ids.iter().any(|id| id == role_id) {
                    session.role_ids.retain(|id| id != role_id);
                    self.state
                        .open_db()?
                        .execute(
                            "UPDATE sessions SET role_ids = ?2 WHERE id = ?1",
                            params![
                                session.id.clone(),
                                serde_json::to_string(&session.role_ids)?
                            ],
                        )
                        .map_err(|error| AppError::database(error.to_string()))?;
                }
            }
        }

        self.state
            .roles
            .lock()
            .map_err(|_| AppError::runtime("roles mutex poisoned"))?
            .retain(|role| role.id != role_id);
        Ok(())
    }

    async fn list_permissions(&self) -> Result<Vec<PermissionRecord>, AppError> {
        Ok(self
            .state
            .permissions
            .lock()
            .map_err(|_| AppError::runtime("permissions mutex poisoned"))?
            .clone())
    }

    async fn create_permission(
        &self,
        mut record: PermissionRecord,
    ) -> Result<PermissionRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("permission-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }

        self.state.open_db()?.execute(
            "INSERT INTO permissions (id, workspace_id, name, code, description, status, kind, target_type, target_ids, action, member_permission_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                record.id,
                record.workspace_id,
                record.name,
                record.code,
                record.description,
                record.status,
                record.kind,
                record.target_type,
                serde_json::to_string(&record.target_ids)?,
                record.action,
                serde_json::to_string(&record.member_permission_ids)?,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        let mut permissions = self
            .state
            .permissions
            .lock()
            .map_err(|_| AppError::runtime("permissions mutex poisoned"))?;
        permissions.push(record.clone());
        Ok(record)
    }

    async fn update_permission(
        &self,
        permission_id: &str,
        mut record: PermissionRecord,
    ) -> Result<PermissionRecord, AppError> {
        record.id = permission_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO permissions (id, workspace_id, name, code, description, status, kind, target_type, target_ids, action, member_permission_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                record.id,
                record.workspace_id,
                record.name,
                record.code,
                record.description,
                record.status,
                record.kind,
                record.target_type,
                serde_json::to_string(&record.target_ids)?,
                record.action,
                serde_json::to_string(&record.member_permission_ids)?,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        let mut permissions = self
            .state
            .permissions
            .lock()
            .map_err(|_| AppError::runtime("permissions mutex poisoned"))?;
        Self::replace_or_push(&mut permissions, record.clone(), |item| {
            item.id == permission_id
        });
        Ok(record)
    }

    async fn delete_permission(&self, permission_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM permissions WHERE id = ?1",
                params![permission_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        {
            let mut roles = self
                .state
                .roles
                .lock()
                .map_err(|_| AppError::runtime("roles mutex poisoned"))?;
            for role in roles.iter_mut() {
                if role.permission_ids.iter().any(|id| id == permission_id) {
                    role.permission_ids.retain(|id| id != permission_id);
                    self.state
                        .open_db()?
                        .execute(
                            "UPDATE roles SET permission_ids = ?2 WHERE id = ?1",
                            params![
                                role.id.clone(),
                                serde_json::to_string(&role.permission_ids)?
                            ],
                        )
                        .map_err(|error| AppError::database(error.to_string()))?;
                }
            }
        }

        {
            let mut permissions = self
                .state
                .permissions
                .lock()
                .map_err(|_| AppError::runtime("permissions mutex poisoned"))?;
            for permission in permissions.iter_mut() {
                if permission
                    .member_permission_ids
                    .iter()
                    .any(|id| id == permission_id)
                {
                    permission
                        .member_permission_ids
                        .retain(|id| id != permission_id);
                    self.state
                        .open_db()?
                        .execute(
                            "UPDATE permissions SET member_permission_ids = ?2 WHERE id = ?1",
                            params![
                                permission.id.clone(),
                                serde_json::to_string(&permission.member_permission_ids)?
                            ],
                        )
                        .map_err(|error| AppError::database(error.to_string()))?;
                }
            }
            permissions.retain(|permission| permission.id != permission_id);
        }
        Ok(())
    }

    async fn list_menus(&self) -> Result<Vec<MenuRecord>, AppError> {
        Ok(self
            .state
            .menus
            .lock()
            .map_err(|_| AppError::runtime("menus mutex poisoned"))?
            .clone())
    }

    async fn create_menu(&self, mut record: MenuRecord) -> Result<MenuRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("menu-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }

        self.state.open_db()?.execute(
            "INSERT INTO menus (id, workspace_id, parent_id, source, label, route_name, status, order_value)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.id,
                record.workspace_id,
                record.parent_id,
                record.source,
                record.label,
                record.route_name,
                record.status,
                record.order,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        let mut menus = self
            .state
            .menus
            .lock()
            .map_err(|_| AppError::runtime("menus mutex poisoned"))?;
        menus.push(record.clone());
        Ok(record)
    }

    async fn update_menu(
        &self,
        menu_id: &str,
        mut record: MenuRecord,
    ) -> Result<MenuRecord, AppError> {
        record.id = menu_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO menus (id, workspace_id, parent_id, source, label, route_name, status, order_value)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.id,
                record.workspace_id,
                record.parent_id,
                record.source,
                record.label,
                record.route_name,
                record.status,
                record.order,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        let mut menus = self
            .state
            .menus
            .lock()
            .map_err(|_| AppError::runtime("menus mutex poisoned"))?;
        Self::replace_or_push(&mut menus, record.clone(), |item| item.id == menu_id);
        Ok(record)
    }
}

#[async_trait]
impl AuthService for InfraAuthService {
    async fn login(&self, request: LoginRequest) -> Result<LoginResponse, AppError> {
        self.ensure_active_client_app(&request.client_app_id)?;

        let user = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .iter()
            .find(|user| user.record.username == request.username)
            .cloned()
            .ok_or_else(|| AppError::auth("invalid credentials"))?;
        if !verify_password(&request.password, &user.password_hash) {
            return Err(AppError::auth("invalid credentials"));
        }

        let session = self.persist_session(&user, request.client_app_id)?;

        Ok(LoginResponse {
            session,
            workspace: self.workspace_snapshot()?,
        })
    }

    async fn register_owner(
        &self,
        request: RegisterWorkspaceOwnerRequest,
    ) -> Result<RegisterWorkspaceOwnerResponse, AppError> {
        self.ensure_active_client_app(&request.client_app_id)?;

        if request.username.trim().is_empty() || request.display_name.trim().is_empty() {
            return Err(AppError::invalid_input(
                "username and display name are required",
            ));
        }
        if request.password.len() < 8 {
            return Err(AppError::invalid_input(
                "password must be at least 8 characters",
            ));
        }
        if request.password != request.confirm_password {
            return Err(AppError::invalid_input(
                "password confirmation does not match",
            ));
        }
        if self.owner_exists()? {
            return Err(AppError::conflict("workspace owner already exists"));
        }

        let workspace = self.workspace_snapshot()?;
        if workspace.bootstrap_status != "setup_required" && workspace.owner_user_id.is_some() {
            return Err(AppError::conflict("workspace owner already exists"));
        }

        {
            let users = self
                .state
                .users
                .lock()
                .map_err(|_| AppError::runtime("users mutex poisoned"))?;
            if users
                .iter()
                .any(|user| user.record.username == request.username.trim())
            {
                return Err(AppError::conflict("username already exists"));
            }
        }

        let now = Self::now();
        let user_id = format!("user-{}", Uuid::new_v4());
        let (avatar_path, avatar_content_type, avatar_byte_size, avatar_content_hash) =
            self.persist_avatar(&user_id, &request.avatar)?;
        let user_record = UserRecord {
            id: user_id.clone(),
            username: request.username.trim().to_string(),
            display_name: request.display_name.trim().to_string(),
            avatar_path: Some(avatar_path.clone()),
            avatar_content_type: Some(avatar_content_type.clone()),
            avatar_byte_size: Some(avatar_byte_size),
            avatar_content_hash: Some(avatar_content_hash.clone()),
            status: "active".into(),
            password_state: "set".into(),
            created_at: now,
            updated_at: now,
        };
        let membership = WorkspaceMembershipRecord {
            workspace_id: workspace.id.clone(),
            user_id: user_id.clone(),
            role_ids: vec!["owner".into()],
            scope_mode: "all-projects".into(),
            scope_project_ids: Vec::new(),
        };

        let db = self.state.open_db()?;
        db.execute(
            "INSERT INTO users (id, username, display_name, avatar_path, avatar_content_type, avatar_byte_size, avatar_content_hash, status, password_hash, password_state, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                user_record.id,
                user_record.username,
                user_record.display_name,
                user_record.avatar_path,
                user_record.avatar_content_type,
                user_record.avatar_byte_size.map(|value| value as i64),
                user_record.avatar_content_hash,
                user_record.status,
                hash_password(&request.password),
                user_record.password_state,
                user_record.created_at as i64,
                user_record.updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
        db.execute(
            "INSERT INTO memberships (workspace_id, user_id, role_ids, scope_mode, scope_project_ids)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                membership.workspace_id,
                membership.user_id,
                serde_json::to_string(&membership.role_ids)?,
                membership.scope_mode,
                serde_json::to_string(&membership.scope_project_ids)?,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

        {
            let mut workspace_state = self
                .state
                .workspace
                .lock()
                .map_err(|_| AppError::runtime("workspace mutex poisoned"))?;
            workspace_state.bootstrap_status = "ready".into();
            workspace_state.owner_user_id = Some(user_id.clone());
        }
        self.state.save_workspace_config()?;

        let stored_user = StoredUser {
            record: user_record,
            password_hash: hash_password(&request.password),
            membership,
        };
        self.state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .push(stored_user.clone());

        let session = self.persist_session(&stored_user, request.client_app_id)?;

        Ok(RegisterWorkspaceOwnerResponse {
            session,
            workspace: self.workspace_snapshot()?,
        })
    }

    async fn logout(&self, token: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "UPDATE sessions SET status = 'revoked' WHERE token = ?1",
                params![token],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        if let Some(session) = self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .iter_mut()
            .find(|session| session.token == token)
        {
            session.status = "revoked".into();
        }

        Ok(())
    }

    async fn session(&self, token: &str) -> Result<SessionRecord, AppError> {
        self.lookup_session(token)
            .await?
            .ok_or_else(|| AppError::auth("session token is invalid"))
    }

    async fn lookup_session(&self, token: &str) -> Result<Option<SessionRecord>, AppError> {
        Ok(self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .iter()
            .find(|session| session.token == token && session.status == "active")
            .cloned())
    }
}

#[async_trait]
impl AppRegistryService for InfraAppRegistryService {
    async fn list_apps(&self) -> Result<Vec<ClientAppRecord>, AppError> {
        Ok(self
            .state
            .apps
            .lock()
            .map_err(|_| AppError::runtime("app registry mutex poisoned"))?
            .clone())
    }

    async fn register_app(&self, record: ClientAppRecord) -> Result<ClientAppRecord, AppError> {
        if !record.first_party {
            return Err(AppError::invalid_input(
                "phase one only accepts first-party client apps",
            ));
        }

        self.state
            .open_db()?
            .execute(
                "INSERT OR REPLACE INTO client_apps
                 (id, name, platform, status, first_party, allowed_origins, allowed_hosts, session_policy, default_scopes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.name,
                    record.platform,
                    record.status,
                    if record.first_party { 1 } else { 0 },
                    serde_json::to_string(&record.allowed_origins)?,
                    serde_json::to_string(&record.allowed_hosts)?,
                    record.session_policy,
                    serde_json::to_string(&record.default_scopes)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        let mut apps = self
            .state
            .apps
            .lock()
            .map_err(|_| AppError::runtime("app registry mutex poisoned"))?;
        if let Some(existing) = apps.iter_mut().find(|app| app.id == record.id) {
            *existing = record.clone();
        } else {
            apps.push(record.clone());
        }
        let registry = AppRegistryFile { apps: apps.clone() };
        fs::write(
            &self.state.paths.app_registry_config,
            toml::to_string_pretty(&registry)?,
        )?;

        Ok(record)
    }

    async fn find_app(&self, app_id: &str) -> Result<Option<ClientAppRecord>, AppError> {
        Ok(self
            .state
            .apps
            .lock()
            .map_err(|_| AppError::runtime("app registry mutex poisoned"))?
            .iter()
            .find(|app| app.id == app_id)
            .cloned())
    }
}

#[async_trait]
impl RbacService for InfraRbacService {
    async fn authorize(
        &self,
        session: &SessionRecord,
        _capability: &str,
        project_id: Option<&str>,
    ) -> Result<AuthorizationDecision, AppError> {
        if session.role_ids.iter().any(|role| role == "owner") {
            return Ok(AuthorizationDecision {
                allowed: project_id
                    .map(|project| {
                        session.scope_project_ids.is_empty()
                            || session.scope_project_ids.iter().any(|item| item == project)
                    })
                    .unwrap_or(true),
                reason: None,
            });
        }

        Ok(AuthorizationDecision {
            allowed: false,
            reason: Some("no matching role permission".into()),
        })
    }
}

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
    use octopus_platform::WorkspaceService;
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
                "INSERT INTO memberships (workspace_id, user_id, role_ids, scope_mode, scope_project_ids)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    "ws-local",
                    "user-owner",
                    "[\"owner\"]",
                    "all-projects",
                    "[]",
                ],
            )
            .expect("insert owner membership");
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
}
