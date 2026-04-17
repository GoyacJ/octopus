import type {
  AccessExperienceResponse,
  AccessMemberSummary,
  AccessAuditListResponse,
  AccessAuditQuery,
  AccessSessionRecord,
  AccessRoleRecord,
  AccessUserRecord,
  AccessUserPresetUpdateRequest,
  AccessUserUpsertRequest,
  AgentRecord,
  AuthorizationSnapshot,
  CapabilityManagementProjection,
  CapabilityAssetDisablePatch,
  ConversationRecord,
  CreateMenuPolicyRequest,
  CreateDeliverableVersionInput,
  ChangeCurrentUserPasswordRequest,
  ChangeCurrentUserPasswordResponse,
  CopyWorkspaceSkillToManagedInput,
  DeliverableDetail,
  DeliverableSummary,
  DeliverableVersionContent,
  DeliverableVersionSummary,
  ForkDeliverableInput,
  CreateProjectRequest,
  CreateProjectPromotionRequestInput,
  CreateRuntimeSessionInput,
  CreateWorkspaceResourceFolderInput,
  CreateWorkspaceResourceInput,
  CreateWorkspaceSkillInput,
  CredentialBinding,
  DataPolicyRecord,
  DataPolicyUpsertRequest,
  EnterpriseAuthSuccess,
  EnterpriseLoginRequest,
  EnterpriseSessionSummary,
  FeatureDefinition,
  BindPetConversationInput,
  CancelRuntimeSubrunInput,
  ExportWorkspaceAgentBundleInput,
  ExportWorkspaceAgentBundleResult,
  ImportWorkspaceAgentBundleInput,
  ImportWorkspaceAgentBundlePreview,
  ImportWorkspaceAgentBundlePreviewInput,
  ImportWorkspaceAgentBundleResult,
  InboxItemRecord,
  ImportWorkspaceSkillArchiveInput,
  ImportWorkspaceSkillFolderInput,
  KnowledgeRecord,
  MenuDefinition,
  MenuGateResult,
  MenuPolicyRecord,
  MenuPolicyUpsertRequest,
  ModelCatalogSnapshot,
  OrgUnitRecord,
  OrgUnitUpsertRequest,
  PermissionDefinition,
  PetConversationBinding,
  PetDashboardSummary,
  PetPresenceState,
  PetWorkspaceSnapshot,
  PositionRecord,
  PositionUpsertRequest,
  PromoteDeliverableInput,
  PromoteWorkspaceResourceInput,
  ProtectedResourceDescriptor,
  ProtectedResourceMetadataUpsertRequest,
  ProjectAgentLinkInput,
  ProjectAgentLinkRecord,
  ProjectDashboardSnapshot,
  ProjectPromotionRequest,
  ProjectRecord,
  ProjectTeamLinkInput,
  ProjectTeamLinkRecord,
  RegisterBootstrapAdminRequest,
  ReviewProjectPromotionRequestInput,
  ResolveRuntimeApprovalInput,
  ResolveRuntimeAuthChallengeInput,
  ResolveRuntimeMemoryProposalInput,
  ResourcePolicyRecord,
  ResourcePolicyUpsertRequest,
  RoleBindingRecord,
  RoleBindingUpsertRequest,
  RoleUpsertRequest,
  KnowledgeEntryRecord,
  RuntimeBootstrap,
  RuntimeConfigPatch,
  RuntimeConfigValidationResult,
  RuntimeConfiguredModelCredentialRecord,
  RuntimeConfiguredModelCredentialUpsertInput,
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
  SystemAuthStatus,
  TeamRecord,
  ToolRecord,
  UpdateCurrentUserProfileRequest,
  UpdateProjectRequest,
  UpdateWorkspaceResourceInput,
  UpdateWorkspaceSkillFileInput,
  UpdateWorkspaceSkillInput,
  UpsertAgentInput,
  UpsertTeamInput,
  UpsertWorkspaceMcpServerInput,
  UserGroupRecord,
  UserGroupUpsertRequest,
  UserOrgAssignmentRecord,
  UserOrgAssignmentUpsertRequest,
  UserRecordSummary,
  WorkspaceConnectionRecord,
  WorkspaceDirectoryBrowserResponse,
  WorkspaceMcpServerDocument,
  WorkspaceOverviewSnapshot,
  WorkspaceResourceChildrenRecord,
  WorkspaceResourceContentDocument,
  WorkspaceResourceImportInput,
  WorkspaceResourceRecord,
  WorkspaceSessionTokenEnvelope,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceSkillTreeDocument,
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
  enterpriseAuth: {
    getStatus: () => Promise<SystemAuthStatus>
    login: (input: EnterpriseLoginRequest) => Promise<EnterpriseAuthSuccess>
    bootstrapAdmin: (input: RegisterBootstrapAdminRequest) => Promise<EnterpriseAuthSuccess>
    session: () => Promise<EnterpriseSessionSummary>
  }
  workspace: {
    get: () => Promise<WorkspaceOverviewSnapshot['workspace']>
    getOverview: () => Promise<WorkspaceOverviewSnapshot>
    listPromotionRequests: () => Promise<ProjectPromotionRequest[]>
    reviewPromotionRequest: (
      requestId: string,
      input: ReviewProjectPromotionRequestInput,
    ) => Promise<ProjectPromotionRequest>
  }
  projects: {
    list: () => Promise<ProjectRecord[]>
    create: (input: CreateProjectRequest) => Promise<ProjectRecord>
    update: (projectId: string, input: UpdateProjectRequest) => Promise<ProjectRecord>
    getDashboard: (projectId: string) => Promise<ProjectDashboardSnapshot>
    listDeliverables: (projectId: string) => Promise<DeliverableSummary[]>
    listPromotionRequests: (projectId: string) => Promise<ProjectPromotionRequest[]>
    createPromotionRequest: (
      projectId: string,
      input: CreateProjectPromotionRequestInput,
    ) => Promise<ProjectPromotionRequest>
  }
  resources: {
    listWorkspace: () => Promise<WorkspaceResourceRecord[]>
    listProject: (projectId: string) => Promise<WorkspaceResourceRecord[]>
    createWorkspace: (input: CreateWorkspaceResourceInput) => Promise<WorkspaceResourceRecord>
    createProject: (projectId: string, input: CreateWorkspaceResourceInput) => Promise<WorkspaceResourceRecord>
    createProjectFolder: (projectId: string, input: CreateWorkspaceResourceFolderInput) => Promise<WorkspaceResourceRecord[]>
    importWorkspace: (input: WorkspaceResourceImportInput) => Promise<WorkspaceResourceRecord>
    importProject: (projectId: string, input: WorkspaceResourceImportInput) => Promise<WorkspaceResourceRecord>
    getDetail: (resourceId: string) => Promise<WorkspaceResourceRecord>
    getContent: (resourceId: string) => Promise<WorkspaceResourceContentDocument>
    listChildren: (resourceId: string) => Promise<WorkspaceResourceChildrenRecord[]>
    promote: (resourceId: string, input: PromoteWorkspaceResourceInput) => Promise<WorkspaceResourceRecord>
    updateWorkspace: (resourceId: string, input: UpdateWorkspaceResourceInput) => Promise<WorkspaceResourceRecord>
    updateProject: (projectId: string, resourceId: string, input: UpdateWorkspaceResourceInput) => Promise<WorkspaceResourceRecord>
    deleteWorkspace: (resourceId: string) => Promise<void>
    deleteProject: (projectId: string, resourceId: string) => Promise<void>
  }
  filesystem: {
    listDirectories: (path?: string) => Promise<WorkspaceDirectoryBrowserResponse>
  }
  deliverables: {
    listWorkspace: () => Promise<DeliverableSummary[]>
  }
  inbox: {
    list: () => Promise<InboxItemRecord[]>
  }
  knowledge: {
    listWorkspace: () => Promise<KnowledgeRecord[]>
    listProject: (projectId: string) => Promise<KnowledgeRecord[]>
  }
  pet: {
    getDashboard: () => Promise<PetDashboardSummary>
    getSnapshot: (projectId?: string) => Promise<PetWorkspaceSnapshot>
    savePresence: (input: SavePetPresenceInput, projectId?: string) => Promise<PetPresenceState>
    bindConversation: (input: BindPetConversationInput, projectId?: string) => Promise<PetConversationBinding>
  }
  agents: {
    list: () => Promise<AgentRecord[]>
    create: (input: UpsertAgentInput) => Promise<AgentRecord>
    update: (agentId: string, input: UpsertAgentInput) => Promise<AgentRecord>
    delete: (agentId: string) => Promise<void>
    copyToWorkspace: (agentId: string) => Promise<ImportWorkspaceAgentBundleResult>
    copyToProject: (
      projectId: string,
      agentId: string,
    ) => Promise<ImportWorkspaceAgentBundleResult>
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
    copyToWorkspace: (teamId: string) => Promise<ImportWorkspaceAgentBundleResult>
    copyToProject: (
      projectId: string,
      teamId: string,
    ) => Promise<ImportWorkspaceAgentBundleResult>
    listProjectLinks: (projectId: string) => Promise<ProjectTeamLinkRecord[]>
    linkProject: (input: ProjectTeamLinkInput) => Promise<ProjectTeamLinkRecord>
    unlinkProject: (projectId: string, teamId: string) => Promise<void>
  }
  catalog: {
    getSnapshot: () => Promise<ModelCatalogSnapshot>
    getManagementProjection: () => Promise<CapabilityManagementProjection>
    setAssetDisabled: (patch: CapabilityAssetDisablePatch) => Promise<CapabilityManagementProjection>
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
    copyMcpServerToManaged: (serverName: string) => Promise<WorkspaceMcpServerDocument>
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
  profile: {
    updateCurrentUserProfile: (input: UpdateCurrentUserProfileRequest) => Promise<UserRecordSummary>
    changeCurrentUserPassword: (input: ChangeCurrentUserPasswordRequest) => Promise<ChangeCurrentUserPasswordResponse>
  }
  accessControl: {
    getCurrentAuthorization: () => Promise<AuthorizationSnapshot>
    getAccessExperience: () => Promise<AccessExperienceResponse>
    listMembers: () => Promise<AccessMemberSummary[]>
    listAudit: (query?: AccessAuditQuery) => Promise<AccessAuditListResponse>
    listSessions: () => Promise<AccessSessionRecord[]>
    revokeCurrentSession: () => Promise<void>
    revokeSession: (sessionId: string) => Promise<void>
    revokeUserSessions: (userId: string) => Promise<void>
    listUsers: () => Promise<AccessUserRecord[]>
    updateUserPreset: (
      userId: string,
      input: AccessUserPresetUpdateRequest,
    ) => Promise<AccessMemberSummary>
    createUser: (input: AccessUserUpsertRequest) => Promise<AccessUserRecord>
    updateUser: (userId: string, input: AccessUserUpsertRequest) => Promise<AccessUserRecord>
    deleteUser: (userId: string) => Promise<void>
    listOrgUnits: () => Promise<OrgUnitRecord[]>
    createOrgUnit: (input: OrgUnitUpsertRequest) => Promise<OrgUnitRecord>
    updateOrgUnit: (orgUnitId: string, input: OrgUnitUpsertRequest) => Promise<OrgUnitRecord>
    deleteOrgUnit: (orgUnitId: string) => Promise<void>
    listPositions: () => Promise<PositionRecord[]>
    createPosition: (input: PositionUpsertRequest) => Promise<PositionRecord>
    updatePosition: (positionId: string, input: PositionUpsertRequest) => Promise<PositionRecord>
    deletePosition: (positionId: string) => Promise<void>
    listUserGroups: () => Promise<UserGroupRecord[]>
    createUserGroup: (input: UserGroupUpsertRequest) => Promise<UserGroupRecord>
    updateUserGroup: (groupId: string, input: UserGroupUpsertRequest) => Promise<UserGroupRecord>
    deleteUserGroup: (groupId: string) => Promise<void>
    listUserOrgAssignments: () => Promise<UserOrgAssignmentRecord[]>
    upsertUserOrgAssignment: (input: UserOrgAssignmentUpsertRequest) => Promise<UserOrgAssignmentRecord>
    deleteUserOrgAssignment: (userId: string, orgUnitId: string) => Promise<void>
    listRoles: () => Promise<AccessRoleRecord[]>
    createRole: (input: RoleUpsertRequest) => Promise<AccessRoleRecord>
    updateRole: (roleId: string, input: RoleUpsertRequest) => Promise<AccessRoleRecord>
    deleteRole: (roleId: string) => Promise<void>
    listPermissionDefinitions: () => Promise<PermissionDefinition[]>
    listRoleBindings: () => Promise<RoleBindingRecord[]>
    createRoleBinding: (input: RoleBindingUpsertRequest) => Promise<RoleBindingRecord>
    updateRoleBinding: (bindingId: string, input: RoleBindingUpsertRequest) => Promise<RoleBindingRecord>
    deleteRoleBinding: (bindingId: string) => Promise<void>
    listDataPolicies: () => Promise<DataPolicyRecord[]>
    createDataPolicy: (input: DataPolicyUpsertRequest) => Promise<DataPolicyRecord>
    updateDataPolicy: (policyId: string, input: DataPolicyUpsertRequest) => Promise<DataPolicyRecord>
    deleteDataPolicy: (policyId: string) => Promise<void>
    listResourcePolicies: () => Promise<ResourcePolicyRecord[]>
    createResourcePolicy: (input: ResourcePolicyUpsertRequest) => Promise<ResourcePolicyRecord>
    updateResourcePolicy: (policyId: string, input: ResourcePolicyUpsertRequest) => Promise<ResourcePolicyRecord>
    deleteResourcePolicy: (policyId: string) => Promise<void>
    listMenuDefinitions: () => Promise<MenuDefinition[]>
    listFeatureDefinitions: () => Promise<FeatureDefinition[]>
    listMenuGateResults: () => Promise<MenuGateResult[]>
    listMenuPolicies: () => Promise<MenuPolicyRecord[]>
    createMenuPolicy: (input: CreateMenuPolicyRequest) => Promise<MenuPolicyRecord>
    updateMenuPolicy: (menuId: string, input: MenuPolicyUpsertRequest) => Promise<MenuPolicyRecord>
    deleteMenuPolicy: (menuId: string) => Promise<void>
    listProtectedResources: () => Promise<ProtectedResourceDescriptor[]>
    upsertProtectedResource: (
      resourceType: string,
      resourceId: string,
      input: ProtectedResourceMetadataUpsertRequest,
    ) => Promise<ProtectedResourceDescriptor>
  }
  runtime: {
    bootstrap: () => Promise<RuntimeBootstrap>
    getConfig: () => Promise<RuntimeEffectiveConfig>
    validateConfig: (patch: RuntimeConfigPatch) => Promise<RuntimeConfigValidationResult>
    upsertConfiguredModelCredential: (
      configuredModelId: string,
      input: RuntimeConfiguredModelCredentialUpsertInput,
    ) => Promise<RuntimeConfiguredModelCredentialRecord>
    deleteConfiguredModelCredential: (configuredModelId: string) => Promise<void>
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
    getDeliverableDetail: (deliverableId: string) => Promise<DeliverableDetail>
    listDeliverableVersions: (deliverableId: string) => Promise<DeliverableVersionSummary[]>
    getDeliverableVersionContent: (
      deliverableId: string,
      version: number,
    ) => Promise<DeliverableVersionContent>
    createDeliverableVersion: (
      deliverableId: string,
      input: CreateDeliverableVersionInput,
      idempotencyKey?: string,
    ) => Promise<DeliverableDetail>
    promoteDeliverable: (
      deliverableId: string,
      input: PromoteDeliverableInput,
      idempotencyKey?: string,
    ) => Promise<KnowledgeEntryRecord>
    forkDeliverable: (
      deliverableId: string,
      input: ForkDeliverableInput,
      idempotencyKey?: string,
    ) => Promise<ConversationRecord>
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
    ) => Promise<RuntimeRunSnapshot>
    resolveAuthChallenge: (
      sessionId: string,
      challengeId: string,
      input: ResolveRuntimeAuthChallengeInput,
      idempotencyKey?: string,
    ) => Promise<RuntimeRunSnapshot>
    cancelSubrun: (
      sessionId: string,
      subrunId: string,
      input: CancelRuntimeSubrunInput,
      idempotencyKey?: string,
    ) => Promise<RuntimeRunSnapshot>
    resolveMemoryProposal: (
      sessionId: string,
      proposalId: string,
      input: ResolveRuntimeMemoryProposalInput,
      idempotencyKey?: string,
    ) => Promise<RuntimeRunSnapshot>
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
