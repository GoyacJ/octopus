import type {
  AgentRecord,
  AutomationRecord,
  ChangeCurrentUserPasswordRequest,
  ChangeCurrentUserPasswordResponse,
  CopyWorkspaceSkillToManagedInput,
  CreateProjectRequest,
  CreateRuntimeSessionInput,
  CreateWorkspaceResourceFolderInput,
  CreateWorkspaceResourceInput,
  CreateWorkspaceSkillInput,
  CreateWorkspaceUserRequest,
  CredentialBinding,
  BindPetConversationInput,
  ExportWorkspaceAgentBundleInput,
  ExportWorkspaceAgentBundleResult,
  ImportWorkspaceAgentBundleInput,
  ImportWorkspaceAgentBundlePreview,
  ImportWorkspaceAgentBundlePreviewInput,
  ImportWorkspaceAgentBundleResult,
  ImportWorkspaceSkillArchiveInput,
  ImportWorkspaceSkillFolderInput,
  KnowledgeRecord,
  LoginRequest,
  LoginResponse,
  MenuRecord,
  ModelCatalogSnapshot,
  PermissionRecord,
  PetConversationBinding,
  PetPresenceState,
  PetWorkspaceSnapshot,
  ProjectAgentLinkInput,
  ProjectAgentLinkRecord,
  ProjectDashboardSnapshot,
  ProjectRecord,
  ProjectTeamLinkInput,
  ProjectTeamLinkRecord,
  RegisterWorkspaceOwnerRequest,
  RegisterWorkspaceOwnerResponse,
  ResolveRuntimeApprovalInput,
  RoleRecord,
  RuntimeBootstrap,
  RuntimeConfigPatch,
  RuntimeConfigValidationResult,
  RuntimeConfiguredModelProbeInput,
  RuntimeConfiguredModelProbeResult,
  RuntimeEventEnvelope,
  RuntimeEffectiveConfig,
  RuntimeRunSnapshot,
  RuntimeSessionDetail,
  RuntimeSessionSummary,
  SavePetPresenceInput,
  SubmitRuntimeTurnInput,
  SystemBootstrapStatus,
  TeamRecord,
  ToolRecord,
  UpdateCurrentUserProfileRequest,
  UpdateProjectRequest,
  UpdateWorkspaceResourceInput,
  UpdateWorkspaceSkillFileInput,
  UpdateWorkspaceSkillInput,
  UpdateWorkspaceUserRequest,
  UpsertAgentInput,
  UpsertTeamInput,
  UpsertWorkspaceMcpServerInput,
  PermissionCenterOverviewSnapshot,
  UserRecordSummary,
  WorkspaceConnectionRecord,
  WorkspaceMcpServerDocument,
  WorkspaceOverviewSnapshot,
  WorkspaceResourceRecord,
  WorkspaceSessionTokenEnvelope,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceSkillTreeDocument,
  WorkspaceToolCatalogSnapshot,
  WorkspaceToolDisablePatch,
  ArtifactRecord,
} from '@octopus/schema'

import { createRuntimeApi } from './runtime_api'
import { createWorkspaceApi } from './workspace_api'

export interface WorkspaceClientContext {
  connection: WorkspaceConnectionRecord
  session?: WorkspaceSessionTokenEnvelope
}

export interface RuntimeEventsPollOptions {
  after?: string
}

export interface RuntimeEventSubscription {
  mode: 'sse'
  close: () => void
}

export interface RuntimeEventSubscriptionOptions {
  lastEventId?: string
  onEvent: (event: RuntimeEventEnvelope) => void
  onError: (error: Error) => void
}

export interface WorkspaceClient {
  system: {
    bootstrap: () => Promise<SystemBootstrapStatus>
  }
  auth: {
    login: (input: LoginRequest) => Promise<LoginResponse>
    registerOwner: (input: RegisterWorkspaceOwnerRequest) => Promise<RegisterWorkspaceOwnerResponse>
    logout: () => Promise<void>
    session: () => Promise<WorkspaceSessionTokenEnvelope['session']>
  }
  workspace: {
    get: () => Promise<WorkspaceOverviewSnapshot['workspace']>
    getOverview: () => Promise<WorkspaceOverviewSnapshot>
  }
  projects: {
    list: () => Promise<ProjectRecord[]>
    create: (input: CreateProjectRequest) => Promise<ProjectRecord>
    update: (projectId: string, input: UpdateProjectRequest) => Promise<ProjectRecord>
    getDashboard: (projectId: string) => Promise<ProjectDashboardSnapshot>
  }
  resources: {
    listWorkspace: () => Promise<WorkspaceResourceRecord[]>
    listProject: (projectId: string) => Promise<WorkspaceResourceRecord[]>
    createWorkspace: (input: CreateWorkspaceResourceInput) => Promise<WorkspaceResourceRecord>
    createProject: (projectId: string, input: CreateWorkspaceResourceInput) => Promise<WorkspaceResourceRecord>
    createProjectFolder: (projectId: string, input: CreateWorkspaceResourceFolderInput) => Promise<WorkspaceResourceRecord[]>
    updateWorkspace: (resourceId: string, input: UpdateWorkspaceResourceInput) => Promise<WorkspaceResourceRecord>
    updateProject: (projectId: string, resourceId: string, input: UpdateWorkspaceResourceInput) => Promise<WorkspaceResourceRecord>
    deleteWorkspace: (resourceId: string) => Promise<void>
    deleteProject: (projectId: string, resourceId: string) => Promise<void>
  }
  artifacts: {
    listWorkspace: () => Promise<ArtifactRecord[]>
  }
  knowledge: {
    listWorkspace: () => Promise<KnowledgeRecord[]>
    listProject: (projectId: string) => Promise<KnowledgeRecord[]>
  }
  pet: {
    getSnapshot: (projectId?: string) => Promise<PetWorkspaceSnapshot>
    savePresence: (input: SavePetPresenceInput, projectId?: string) => Promise<PetPresenceState>
    bindConversation: (input: BindPetConversationInput, projectId?: string) => Promise<PetConversationBinding>
  }
  agents: {
    list: () => Promise<AgentRecord[]>
    create: (input: UpsertAgentInput) => Promise<AgentRecord>
    update: (agentId: string, input: UpsertAgentInput) => Promise<AgentRecord>
    delete: (agentId: string) => Promise<void>
    previewImportBundle: (
      input: ImportWorkspaceAgentBundlePreviewInput,
      projectId?: string,
    ) => Promise<ImportWorkspaceAgentBundlePreview>
    importBundle: (
      input: ImportWorkspaceAgentBundleInput,
      projectId?: string,
    ) => Promise<ImportWorkspaceAgentBundleResult>
    exportBundle: (
      input: ExportWorkspaceAgentBundleInput,
      projectId?: string,
    ) => Promise<ExportWorkspaceAgentBundleResult>
    listProjectLinks: (projectId: string) => Promise<ProjectAgentLinkRecord[]>
    linkProject: (input: ProjectAgentLinkInput) => Promise<ProjectAgentLinkRecord>
    unlinkProject: (projectId: string, agentId: string) => Promise<void>
  }
  teams: {
    list: () => Promise<TeamRecord[]>
    create: (input: UpsertTeamInput) => Promise<TeamRecord>
    update: (teamId: string, input: UpsertTeamInput) => Promise<TeamRecord>
    delete: (teamId: string) => Promise<void>
    listProjectLinks: (projectId: string) => Promise<ProjectTeamLinkRecord[]>
    linkProject: (input: ProjectTeamLinkInput) => Promise<ProjectTeamLinkRecord>
    unlinkProject: (projectId: string, teamId: string) => Promise<void>
  }
  catalog: {
    getSnapshot: () => Promise<ModelCatalogSnapshot>
    getToolCatalog: () => Promise<WorkspaceToolCatalogSnapshot>
    setToolDisabled: (patch: WorkspaceToolDisablePatch) => Promise<WorkspaceToolCatalogSnapshot>
    getSkill: (skillId: string) => Promise<WorkspaceSkillDocument>
    getSkillTree: (skillId: string) => Promise<WorkspaceSkillTreeDocument>
    getSkillFile: (skillId: string, relativePath: string) => Promise<WorkspaceSkillFileDocument>
    createSkill: (input: CreateWorkspaceSkillInput) => Promise<WorkspaceSkillDocument>
    updateSkill: (skillId: string, input: UpdateWorkspaceSkillInput) => Promise<WorkspaceSkillDocument>
    updateSkillFile: (
      skillId: string,
      relativePath: string,
      input: UpdateWorkspaceSkillFileInput,
    ) => Promise<WorkspaceSkillFileDocument>
    importSkillArchive: (input: ImportWorkspaceSkillArchiveInput) => Promise<WorkspaceSkillDocument>
    importSkillFolder: (input: ImportWorkspaceSkillFolderInput) => Promise<WorkspaceSkillDocument>
    copySkillToManaged: (
      skillId: string,
      input: CopyWorkspaceSkillToManagedInput,
    ) => Promise<WorkspaceSkillDocument>
    deleteSkill: (skillId: string) => Promise<void>
    getMcpServer: (serverName: string) => Promise<WorkspaceMcpServerDocument>
    createMcpServer: (input: UpsertWorkspaceMcpServerInput) => Promise<WorkspaceMcpServerDocument>
    updateMcpServer: (
      serverName: string,
      input: UpsertWorkspaceMcpServerInput,
    ) => Promise<WorkspaceMcpServerDocument>
    deleteMcpServer: (serverName: string) => Promise<void>
    listModels: () => Promise<ModelCatalogSnapshot['models']>
    listProviderCredentials: () => Promise<CredentialBinding[]>
    listTools: () => Promise<ToolRecord[]>
    createTool: (record: ToolRecord) => Promise<ToolRecord>
    updateTool: (toolId: string, record: ToolRecord) => Promise<ToolRecord>
    deleteTool: (toolId: string) => Promise<void>
  }
  automations: {
    list: () => Promise<AutomationRecord[]>
    create: (record: AutomationRecord) => Promise<AutomationRecord>
    update: (automationId: string, record: AutomationRecord) => Promise<AutomationRecord>
    delete: (automationId: string) => Promise<void>
  }
  rbac: {
    getPermissionCenterOverview: () => Promise<PermissionCenterOverviewSnapshot>
    listUsers: () => Promise<UserRecordSummary[]>
    createUser: (input: CreateWorkspaceUserRequest) => Promise<UserRecordSummary>
    updateUser: (userId: string, input: UpdateWorkspaceUserRequest) => Promise<UserRecordSummary>
    deleteUser: (userId: string) => Promise<void>
    updateCurrentUserProfile: (input: UpdateCurrentUserProfileRequest) => Promise<UserRecordSummary>
    changeCurrentUserPassword: (input: ChangeCurrentUserPasswordRequest) => Promise<ChangeCurrentUserPasswordResponse>
    listRoles: () => Promise<RoleRecord[]>
    createRole: (record: RoleRecord) => Promise<RoleRecord>
    updateRole: (roleId: string, record: RoleRecord) => Promise<RoleRecord>
    deleteRole: (roleId: string) => Promise<void>
    listPermissions: () => Promise<PermissionRecord[]>
    createPermission: (record: PermissionRecord) => Promise<PermissionRecord>
    updatePermission: (permissionId: string, record: PermissionRecord) => Promise<PermissionRecord>
    deletePermission: (permissionId: string) => Promise<void>
    listMenus: () => Promise<MenuRecord[]>
    createMenu: (record: MenuRecord) => Promise<MenuRecord>
    updateMenu: (menuId: string, record: MenuRecord) => Promise<MenuRecord>
  }
  runtime: {
    bootstrap: () => Promise<RuntimeBootstrap>
    getConfig: () => Promise<RuntimeEffectiveConfig>
    validateConfig: (patch: RuntimeConfigPatch) => Promise<RuntimeConfigValidationResult>
    validateConfiguredModel: (input: RuntimeConfiguredModelProbeInput) => Promise<RuntimeConfiguredModelProbeResult>
    saveConfig: (patch: RuntimeConfigPatch) => Promise<RuntimeEffectiveConfig>
    getProjectConfig: (projectId: string) => Promise<RuntimeEffectiveConfig>
    validateProjectConfig: (projectId: string, patch: RuntimeConfigPatch) => Promise<RuntimeConfigValidationResult>
    saveProjectConfig: (projectId: string, patch: RuntimeConfigPatch) => Promise<RuntimeEffectiveConfig>
    getUserConfig: () => Promise<RuntimeEffectiveConfig>
    validateUserConfig: (patch: RuntimeConfigPatch) => Promise<RuntimeConfigValidationResult>
    saveUserConfig: (patch: RuntimeConfigPatch) => Promise<RuntimeEffectiveConfig>
    listSessions: () => Promise<RuntimeSessionSummary[]>
    createSession: (input: CreateRuntimeSessionInput, idempotencyKey?: string) => Promise<RuntimeSessionDetail>
    loadSession: (sessionId: string) => Promise<RuntimeSessionDetail>
    deleteSession: (sessionId: string) => Promise<void>
    pollEvents: (sessionId: string, options?: RuntimeEventsPollOptions) => Promise<RuntimeEventEnvelope[]>
    subscribeEvents: (sessionId: string, options: RuntimeEventSubscriptionOptions) => Promise<RuntimeEventSubscription>
    submitUserTurn: (
      sessionId: string,
      input: SubmitRuntimeTurnInput,
      idempotencyKey?: string,
    ) => Promise<RuntimeRunSnapshot>
    resolveApproval: (
      sessionId: string,
      approvalId: string,
      input: ResolveRuntimeApprovalInput,
      idempotencyKey?: string,
    ) => Promise<void>
  }
}

export function createIdempotencyKey(scope: string): string {
  return `${scope}-${Date.now()}-${Math.random().toString(16).slice(2, 10)}`
}

export function createWorkspaceClient(context: WorkspaceClientContext): WorkspaceClient {
  return {
    ...createWorkspaceApi(context),
    runtime: createRuntimeApi(context),
  }
}
