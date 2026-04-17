mod access_control;
mod agent_assets;
mod agent_bundle;
#[allow(dead_code)]
mod agent_seed;
mod artifacts_inbox_knowledge;
mod auth_users;
mod bootstrap;
mod infra_state;
mod project_tasks;
mod projects_teams;
mod resources_skills;
#[cfg(test)]
mod split_module_tests;
mod workspace_paths;

use std::{
    env,
    ffi::OsStr,
    fs,
    hash::{Hash, Hasher},
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
    time::UNIX_EPOCH,
};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use octopus_core::{
    capability_policy_from_sources, default_agent_delegation_policy, default_agent_memory_policy,
    default_agent_shared_capability_policy, default_approval_preference,
    default_artifact_handoff_policy, default_asset_import_metadata, default_asset_trust_metadata,
    default_mailbox_policy, default_model_strategy, default_output_contract,
    default_permission_envelope, default_shared_memory_policy, default_team_delegation_policy,
    default_team_memory_policy, default_team_shared_capability_policy, normalize_task_domains,
    team_topology_from_refs, timestamp_now, workflow_affordance_from_task_domains, AgentRecord,
    AppError, ArtifactRecord, AuditRecord, AuthorizationDecision, AvatarUploadPayload,
    BindPetConversationInput, CapabilityAssetDisablePatch, ChangeCurrentUserPasswordRequest,
    ChangeCurrentUserPasswordResponse, ClientAppRecord, CopyWorkspaceSkillToManagedInput,
    CostLedgerEntry, CreateProjectPromotionRequestInput, CreateProjectRequest,
    CreateWorkspaceResourceFolderInput, CreateWorkspaceResourceInput, CreateWorkspaceSkillInput,
    ExportWorkspaceAgentBundleInput, ExportWorkspaceAgentBundleResult,
    ImportWorkspaceAgentBundleInput, ImportWorkspaceAgentBundlePreview,
    ImportWorkspaceAgentBundlePreviewInput, ImportWorkspaceAgentBundleResult,
    ImportWorkspaceSkillArchiveInput, ImportWorkspaceSkillFolderInput, InboxItemRecord,
    KnowledgeEntryRecord, KnowledgeRecord, LoginRequest, LoginResponse, ModelCatalogRecord,
    PetConversationBinding, PetMessage, PetPosition, PetPresenceState, PetProfile,
    PetWorkspaceSnapshot, ProjectAgentLinkInput, ProjectAgentLinkRecord, ProjectDefaultPermissions,
    ProjectLinkedWorkspaceAssets, ProjectPermissionOverrides, ProjectPromotionRequest,
    ProjectRecord, ProjectTaskInterventionRecord, ProjectTaskRecord, ProjectTaskRunRecord,
    ProjectTaskSchedulerClaimRecord, ProjectTeamLinkInput, ProjectTeamLinkRecord,
    ProjectWorkspaceAssignments, PromoteWorkspaceResourceInput, ProviderCredentialRecord,
    RegisterBootstrapAdminRequest, RegisterBootstrapAdminResponse,
    ReviewProjectPromotionRequestInput, SavePetPresenceInput, SessionRecord, SystemBootstrapStatus,
    TeamRecord, ToolRecord, TraceEventRecord, UpdateCurrentUserProfileRequest,
    UpdateProjectRequest, UpdateWorkspaceResourceInput, UpdateWorkspaceSkillFileInput,
    UpdateWorkspaceSkillInput, UpsertAgentInput, UpsertTeamInput, UpsertWorkspaceMcpServerInput,
    UserRecord, UserRecordSummary, WorkspaceDirectoryBrowserEntry,
    WorkspaceDirectoryBrowserResponse, WorkspaceDirectoryUploadEntry, WorkspaceMcpServerDocument,
    WorkspaceResourceChildrenRecord, WorkspaceResourceContentDocument,
    WorkspaceResourceFolderUploadEntry, WorkspaceResourceImportInput, WorkspaceResourceRecord,
    WorkspaceSkillDocument, WorkspaceSkillFileDocument, WorkspaceSkillTreeDocument,
    WorkspaceSkillTreeNode, WorkspaceSummary, WorkspaceToolManagementCapabilities,
    ASSET_MANIFEST_REVISION_V2, DEFAULT_PROJECT_ID, DEFAULT_WORKSPACE_ID,
};
use octopus_platform::{
    AccessControlService, AppRegistryService, ArtifactService, AuthService, AuthorizationService,
    InboxService, KnowledgeService, ObservationService, WorkspaceService,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zip::ZipArchive;

use access_control::*;
use auth_users::*;
use infra_state::*;
use resources_skills::*;

pub use bootstrap::{build_infra_bundle, initialize_workspace};
pub use workspace_paths::WorkspacePaths;

pub fn find_builtin_agent_template_record(
    workspace_id: &str,
    agent_id: &str,
) -> Result<Option<AgentRecord>, AppError> {
    agent_bundle::find_builtin_agent_template_record(workspace_id, agent_id)
}

pub fn find_builtin_team_template_record(
    workspace_id: &str,
    team_id: &str,
) -> Result<Option<TeamRecord>, AppError> {
    agent_bundle::find_builtin_team_template_record(workspace_id, team_id)
}

pub(crate) fn canonical_agent_ref(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        String::new()
    } else if trimmed.contains(':') {
        trimmed.to_string()
    } else {
        format!("agent:{trimmed}")
    }
}

pub(crate) fn canonical_agent_refs(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| canonical_agent_ref(value))
        .filter(|value| !value.is_empty())
        .collect()
}

#[derive(Clone)]
pub struct InfraWorkspaceService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraAccessControlService {
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
pub struct InfraAuthorizationService {
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

#[derive(Clone)]
pub struct InfraBundle {
    pub paths: WorkspacePaths,
    pub workspace: Arc<InfraWorkspaceService>,
    pub access_control: Arc<InfraAccessControlService>,
    pub auth: Arc<InfraAuthService>,
    pub app_registry: Arc<InfraAppRegistryService>,
    pub authorization: Arc<InfraAuthorizationService>,
    pub artifact: Arc<InfraArtifactService>,
    pub inbox: Arc<InfraInboxService>,
    pub knowledge: Arc<InfraKnowledgeService>,
    pub observation: Arc<InfraObservationService>,
}
