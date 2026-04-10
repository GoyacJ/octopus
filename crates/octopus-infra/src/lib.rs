mod agent_assets;
#[allow(dead_code)]
mod agent_seed;
mod artifacts_inbox_knowledge;
mod auth_users;
mod bootstrap;
mod infra_state;
mod projects_teams;
mod resources_skills;
mod workspace_paths;

#[cfg(test)]
mod agent_import;

#[cfg(test)]
mod split_module_tests;

use std::{
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
    CreateWorkspaceUserRequest, ExportWorkspaceAgentBundleInput, ExportWorkspaceAgentBundleResult,
    ImportWorkspaceAgentBundleInput, ImportWorkspaceAgentBundlePreview,
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

use auth_users::*;
use infra_state::*;
use resources_skills::*;

pub use bootstrap::{build_infra_bundle, initialize_workspace};
pub use workspace_paths::WorkspacePaths;

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
