/* eslint-disable */
// Generated from contracts/openapi/octopus.openapi.yaml by scripts/generate-schema.mjs.
// Source hash: d41eccd8858845037e896e816166ad8691fda12c14c95677c703a6bac8c81902

export const OCTOPUS_OPENAPI_VERSION = "3.1.0"
export const OCTOPUS_API_VERSION = "0.2.5"
export const OCTOPUS_OPENAPI_SOURCE_HASH = "d41eccd8858845037e896e816166ad8691fda12c14c95677c703a6bac8c81902"

export interface AccessAuditListResponse {
  items: AuditRecord[]
  nextCursor?: string
}

export interface AccessCapabilityBundle {
  code: string
  description: string
  name: string
  permissionCodes: string[]
}

export interface AccessExperienceCounts {
  auditEventCount: number
  customRoleCount: number
  dataPolicyCount: number
  menuPolicyCount: number
  orgUnitCount: number
  protectedResourceCount: number
  resourcePolicyCount: number
  sessionCount: number
}

export type AccessExperienceLevel = "personal" | "team" | "enterprise"

export interface AccessExperienceResponse {
  capabilityBundles: AccessCapabilityBundle[]
  counts: AccessExperienceCounts
  rolePresets: AccessRolePreset[]
  roleTemplates: AccessRoleTemplate[]
  sectionGrants: AccessSectionGrant[]
  summary: AccessExperienceSummary
}

export interface AccessExperienceSummary {
  experienceLevel: AccessExperienceLevel
  hasAdvancedPolicies: boolean
  hasCustomRoles: boolean
  hasMenuGovernance: boolean
  hasOrgStructure: boolean
  hasResourceGovernance: boolean
  memberCount: number
  recommendedLandingSection: AccessSectionCode
}

export interface AccessMemberRoleSummary {
  code: string
  id: string
  name: string
  source: AccessRoleSource
}

export interface AccessMemberSummary {
  effectiveRoleNames: string[]
  effectiveRoles: AccessMemberRoleSummary[]
  hasOrgAssignments: boolean
  primaryPresetCode: string | null
  primaryPresetName: string
  user: AccessUserRecord
}

export interface AccessRolePreset {
  capabilityBundleCodes: string[]
  code: string
  description: string
  name: string
  recommendedFor: string
  templateCodes: string[]
}

export interface AccessRoleRecord {
  code: string
  description: string
  editable: boolean
  id: string
  name: string
  permissionCodes: string[]
  source: AccessRoleSource
  status: string
}

export type AccessRoleSource = "system" | "custom"

export interface AccessRoleTemplate {
  code: string
  description: string
  editable: boolean
  managedRoleCodes: string[]
  name: string
}

export type AccessSectionCode = "members" | "access" | "governance"

export interface AccessSectionGrant {
  allowed: boolean
  section: AccessSectionCode
}

export interface AccessSessionRecord {
  clientAppId: string
  createdAt: number
  current: boolean
  displayName: string
  expiresAt?: number
  sessionId: string
  status: string
  userId: string
  username: string
}

export interface AccessUserPresetUpdateRequest {
  presetCode: string
}

export interface AccessUserRecord {
  displayName: string
  id: string
  passwordState: string
  status: string
  username: string
}

export interface AccessUserUpsertRequest {
  confirmPassword?: string
  displayName: string
  password?: string
  resetPassword?: boolean
  status: string
  username: string
}

export interface AgentRecord {
  approvalPreference: ApprovalPreference
  avatar?: string
  avatarPath?: string
  builtinToolKeys: string[]
  capabilityPolicy: CapabilityPolicy
  defaultModelStrategy: DefaultModelStrategy
  delegationPolicy: DelegationPolicy
  dependencyResolution: AssetDependencyResolution[]
  description: string
  id: string
  importMetadata: AssetImportMetadata
  integrationSource?: {
  kind: "workspace-link" | "builtin-template"
  sourceId: string
}
  manifestRevision: string
  mcpServerNames: string[]
  memoryPolicy: MemoryPolicy
  name: string
  outputContract: OutputContract
  permissionEnvelope: PermissionEnvelope
  personality: string
  projectId?: string
  prompt: string
  scope: AgentScope
  sharedCapabilityPolicy: SharedCapabilityPolicy
  skillIds: string[]
  status: AgentStatus
  tags: string[]
  taskDomains: string[]
  trustMetadata: AssetTrustMetadata
  updatedAt: number
  workspaceId: string
}

export type AgentScope = "personal" | "workspace" | "project"

export type AgentStatus = "active" | "archived"

export type ApiErrorCode = "UNAUTHENTICATED" | "SESSION_EXPIRED" | "PERMISSION_DENIED" | "AUTHORIZATION_STALE" | "FORBIDDEN" | "NOT_FOUND" | "CONFLICT" | "INVALID_INPUT" | "RATE_LIMITED" | "UNAVAILABLE" | "CAPABILITY_UNSUPPORTED" | "INTERNAL_ERROR"

export interface ApiErrorDetailEnvelope {
  code: ApiErrorCode
  details?: unknown
  message: string
  requestId: string
  retryable: boolean
}

export interface ApiErrorEnvelope {
  error: ApiErrorDetailEnvelope
}

export type ApprovalMode = "auto" | "require-approval" | "deny"

export interface ApprovalPreference {
  mcpAuth: ApprovalMode
  memoryWrite: ApprovalMode
  teamSpawn: ApprovalMode
  toolExecution: ApprovalMode
  workflowEscalation: ApprovalMode
}

export interface ArtifactHandoffPolicy {
  mode: "leader-reviewed" | "direct-handoff"
  requireLineage: boolean
  retainArtifacts: boolean
}

export type ArtifactStatus = "draft" | "review" | "approved" | "published"

export interface ArtifactVersionReference {
  artifactId: string
  contentType?: string
  previewKind: ResourcePreviewKind
  title: string
  updatedAt: number
  version: number
}

export interface AssetBundleAssetEntry {
  assetKind: "agent" | "team" | "skill" | "mcp-server" | "plugin" | "workflow-template"
  displayName: string
  manifestRevision: string
  sourceId: string
  sourcePath: string
  taskDomains: string[]
  translationMode: "native" | "translate" | "downgrade" | "reject"
}

export interface AssetBundleCompatibilityMapping {
  downgradedFeatures: string[]
  rejectedFeatures: string[]
  supportedTargets: string[]
  translatorVersion: string
}

export interface AssetBundleManifest {
  assets: AssetBundleAssetEntry[]
  bundleKind: "octopus-asset-bundle"
  bundleRoot: string
  compatibilityMapping: AssetBundleCompatibilityMapping
  dependencies: AssetDependency[]
  policyDefaults: AssetBundlePolicyDefaults
  registryMetadata?: AssetBundleRegistryMetadata
  trustMetadata: AssetTrustMetadata
  version: number
}

export interface AssetBundlePolicyDefaults {
  approvalPreference: ApprovalPreference
  defaultModelStrategy: DefaultModelStrategy
  delegationPolicy: DelegationPolicy
  memoryPolicy: MemoryPolicy
  permissionEnvelope: PermissionEnvelope
}

export interface AssetBundleRegistryMetadata {
  publisher: string
  releaseChannel: string
  revision: string
  tags: string[]
}

export interface AssetDependency {
  kind: string
  ref: string
  required: boolean
  versionRange: string
}

export interface AssetDependencyResolution {
  kind: string
  ref: string
  required: boolean
  resolutionState: "resolved" | "missing" | "version-mismatch" | "rejected"
  resolvedRef?: string
}

export interface AssetImportMetadata {
  importedAt?: number
  manifestVersion: number
  originKind: "native" | "bundle-import" | "builtin-template" | "workspace-link"
  sourceId?: string
  translationStatus: "native" | "translated" | "downgraded" | "rejected"
}

export interface AssetTranslationDiagnostic {
  assetId?: string
  assetKind?: string
  code: string
  dependencyRef?: string
  details?: Record<string, unknown>
  message: string
  severity: "info" | "warning" | "error"
  sourcePath?: string
  stage: "parse" | "validate" | "translate" | "execute"
  suggestion?: string
}

export interface AssetTranslationReport {
  dependencyResolution: AssetDependencyResolution[]
  diagnostics: AssetTranslationDiagnostic[]
  downgradedCount: number
  rejectedCount: number
  status: "native" | "translated" | "downgraded" | "rejected" | "mixed"
  translatedCount: number
  trustWarnings: string[]
  unsupportedFeatures: string[]
}

export type AssetTrustLevel = "trusted" | "reviewed" | "untrusted"

export interface AssetTrustMetadata {
  origin: string
  publisher: string
  signatureState: "verified" | "unsigned" | "invalid"
  trustLevel: AssetTrustLevel
  trustWarnings: string[]
  verifiedAt?: number
}

export interface AuditRecord {
  action: string
  actorId: string
  actorType: string
  createdAt: number
  id: string
  outcome: string
  projectId?: string
  resource: string
  workspaceId: string
}

export interface AuthorizationSnapshot {
  effectivePermissionCodes: string[]
  effectiveRoleIds: string[]
  effectiveRoles: AccessRoleRecord[]
  featureCodes: string[]
  menuGates: MenuGateResult[]
  orgAssignments: UserOrgAssignmentRecord[]
  principal: AccessUserRecord
  resourceActionGrants: ResourceActionGrant[]
  visibleMenuIds: string[]
}

export interface AvatarUploadPayload {
  byteSize: number
  contentType: string
  dataBase64: string
  fileName: string
}

export type BackendConnectionState = "ready" | "unavailable"

export type BackendTransport = "http" | "sse" | "ws"

export interface BindPetConversationInput {
  conversationId: string
  petId: string
  sessionId?: string
}

export type BudgetAccountingMode = "provider_reported" | "estimated" | "non_billable"

export type BudgetReservationStrategy = "none" | "fixed"

export interface CancelRuntimeSubrunInput {
  note?: string
}

export interface CapabilityAssetDisablePatch {
  disabled: boolean
  sourceKey: string
}

export type CapabilityAssetExportStatus = "exportable" | "readonly" | "not-exportable"

export type CapabilityAssetImportStatus = "managed" | "copy-required" | "not-importable"

export interface CapabilityAssetManifest {
  assetId: string
  description: string
  displayPath: string
  enabled: boolean
  executionKinds: CapabilityExecutionKind[]
  exportStatus: CapabilityAssetExportStatus
  health: string
  importStatus: CapabilityAssetImportStatus
  installed: boolean
  kind: "builtin" | "skill" | "mcp"
  management: WorkspaceToolManagementCapabilities
  name: string
  ownerId?: string
  ownerLabel?: string
  ownerScope?: string
  requiredPermission: string | null
  sourceKey: string
  sourceKinds: CapabilitySourceKind[]
  state: CapabilityAssetState
  workspaceId: string
}

export type CapabilityAssetState = "builtin" | "workspace" | "project" | "user" | "external" | "managed" | "shadowed" | "disabled"

export interface CapabilityDescriptor {
  capabilityId: string
  label: string
}

export type CapabilityExecutionKind = "tool" | "prompt_skill" | "resource"

export interface CapabilityManagementEntry {
  active?: boolean
  assetId: string
  availability: string
  builtinKey?: string
  capabilityId: string
  consumers?: WorkspaceToolConsumerSummary[]
  description: string
  disabled: boolean
  displayPath: string
  enabled: boolean
  endpoint?: string
  executionKind: CapabilityExecutionKind
  exportStatus: CapabilityAssetExportStatus
  health: string
  id: string
  importStatus: CapabilityAssetImportStatus
  installed: boolean
  kind: "builtin" | "skill" | "mcp"
  management: WorkspaceToolManagementCapabilities
  name: string
  ownerId?: string
  ownerLabel?: string
  ownerScope?: string
  relativePath?: string
  requiredPermission: string | null
  resourceUri?: string
  scope?: string
  serverName?: string
  shadowedBy?: string
  sourceKey: string
  sourceKind: CapabilitySourceKind
  sourceOrigin?: string
  state: CapabilityAssetState
  statusDetail?: string
  toolNames?: string[]
  workspaceId: string
  workspaceOwned?: boolean
}

export interface CapabilityManagementProjection {
  assets: CapabilityAssetManifest[]
  entries: CapabilityManagementEntry[]
  mcpServerPackages: McpServerPackageManifest[]
  skillPackages: SkillPackageManifest[]
}

export interface CapabilityPolicy {
  builtinToolKeys: string[]
  denyByDefault: boolean
  mcpServerNames: string[]
  mode: "allow-list"
  pluginCapabilityRefs: string[]
  skillIds: string[]
}

export type CapabilitySourceKind = "builtin" | "runtime_tool" | "plugin_tool" | "local_skill" | "bundled_skill" | "mcp_tool" | "mcp_prompt" | "mcp_resource" | "plugin_skill"

export interface ChangeCurrentUserPasswordRequest {
  confirmPassword: string
  currentPassword: string
  newPassword: string
}

export interface ChangeCurrentUserPasswordResponse {
  passwordState: PasswordState
}

export interface ClientAppRecord {
  allowedHosts: string[]
  allowedOrigins: string[]
  defaultScopes: string[]
  firstParty: boolean
  id: string
  name: string
  platform: string
  sessionPolicy: string
  status: string
}

export interface ConfiguredModelBudgetPolicy {
  accountingMode?: BudgetAccountingMode
  reservationStrategy?: BudgetReservationStrategy
  totalBudgetTokens?: number
  trafficClasses?: string[]
  warningThresholdPercentages?: number[]
}

export interface ConfiguredModelRecord {
  baseUrl?: string
  budgetPolicy?: ConfiguredModelBudgetPolicy
  configured: boolean
  configuredModelId: string
  credentialRef?: string
  enabled: boolean
  modelId: string
  name: string
  providerId: string
  source: string
  status: string
  tokenUsage: ConfiguredModelTokenUsage
}

export interface ConfiguredModelTokenUsage {
  exhausted: boolean
  remainingTokens?: number
  usedTokens: number
}

export type ConnectionMode = "local" | "shared" | "remote"

export interface ConnectionProfile {
  baseUrl?: string
  id: string
  label: string
  lastSyncAt?: number
  mode: ConnectionMode
  state: ConnectionState
  workspaceId: string
}

export type ConnectionState = "local-ready" | "connected" | "disconnected"

export type ConversationActorKind = "agent" | "team"

export interface ConversationRecord {
  id: string
  lastMessagePreview?: string
  projectId: string
  sessionId: string
  status: string
  title: string
  updatedAt: number
  workspaceId: string
}

export interface CopyWorkspaceSkillToManagedInput {
  slug: string
}

export interface CreateDeliverableVersionInput {
  contentType?: string
  dataBase64?: string
  parentVersion?: number
  previewKind: ResourcePreviewKind
  sourceMessageId?: string
  textContent?: string
  title?: string
}

export interface CreateHostWorkspaceConnectionInput {
  authMode: WorkspaceAuthMode
  baseUrl: string
  label: string
  transportSecurity: TransportSecurityLevel
  workspaceId: string
}

export type CreateMenuPolicyRequest = MenuPolicyUpsertRequest & {
  menuId: string
}

export interface CreateNotificationInput {
  actionLabel?: string
  body?: string
  level?: NotificationLevel
  routeTo?: string
  scopeKind: NotificationScopeKind
  scopeOwnerId?: string
  source?: string
  title?: string
  toastDurationMs?: number
}

export interface CreateProjectDeletionRequestInput {
  reason?: string
}

export interface CreateProjectPromotionRequestInput {
  assetId: string
  assetType: ProjectAssetType
}

export interface CreateProjectRequest {
  description: string
  leaderAgentId?: string
  managerUserId?: string
  memberUserIds?: string[]
  name: string
  ownerUserId?: string
  permissionOverrides?: ProjectPermissionOverrides
  presetCode?: string
  resourceDirectory: string
}

export interface CreateRuntimeSessionInput {
  conversationId: string
  executionPermissionMode: RuntimePermissionMode
  projectId?: string
  selectedActorRef: string
  selectedConfiguredModelId?: string
  sessionKind?: RuntimeSessionKind
  title: string
}

export interface CreateTaskInterventionRequest {
  approvalId?: string | null
  payload: Record<string, unknown>
  taskRunId?: string | null
  type: TaskInterventionType
}

export interface CreateTaskRequest {
  brief: string
  contextBundle: TaskContextBundle
  defaultActorRef: string
  goal: string
  scheduleSpec?: string | null
  title: string
}

export interface CreateWorkspaceResourceFolderInput {
  files: WorkspaceResourceFolderUploadEntry[]
  projectId?: string
}

export interface CreateWorkspaceResourceInput {
  kind: ProjectResourceKind
  location?: string
  name: string
  projectId?: string
  scope?: WorkspaceResourceScope
  sourceArtifactId?: string
  tags?: string[]
  visibility?: WorkspaceResourceVisibility
}

export interface CreateWorkspaceSkillInput {
  content: string
  slug: string
}

export interface CredentialBinding {
  baseUrl?: string
  configured: boolean
  credentialRef: string
  label: string
  providerId: string
  source: string
  status: "healthy" | "error" | "unconfigured" | "configured"
}

export interface DataPolicyRecord {
  classifications: string[]
  effect: string
  id: string
  name: string
  projectIds: string[]
  resourceType: string
  scopeType: string
  subjectId: string
  subjectType: string
  tags: string[]
}

export interface DataPolicyUpsertRequest {
  classifications: string[]
  effect: string
  name: string
  projectIds: string[]
  resourceType: string
  scopeType: string
  subjectId: string
  subjectType: string
  tags: string[]
}

export type DecisionAction = "approve" | "reject"

export interface DefaultModelStrategy {
  allowTurnOverride: boolean
  fallbackModelRefs: string[]
  preferredModelRef?: string
  selectionMode: "session-selected" | "actor-default" | "provider-pinned"
}

export interface DefaultSelection {
  configuredModelId?: string
  modelId: string
  providerId: string
  surface: string
}

export interface DelegationPolicy {
  allowBackgroundRuns: boolean
  allowParallelWorkers: boolean
  maxWorkerCount: number
  mode: "disabled" | "leader-orchestrated" | "single-worker" | "multi-worker"
}

export interface DeliverableDetail {
  byteSize?: number
  contentHash?: string
  contentType?: string
  conversationId: string
  id: string
  latestVersion: number
  latestVersionRef: ArtifactVersionReference
  parentArtifactId?: string
  previewKind: ResourcePreviewKind
  projectId: string
  promotionKnowledgeId?: string
  promotionState: DeliverablePromotionState
  runId: string
  sessionId: string
  sourceMessageId?: string
  status: ArtifactStatus
  storagePath?: string
  title: string
  updatedAt: number
  workspaceId: string
}

export type DeliverablePromotionState = "not-promoted" | "candidate" | "promoted"

export interface DeliverableSummary {
  contentType?: string
  conversationId: string
  id: string
  latestVersion: number
  latestVersionRef: ArtifactVersionReference
  previewKind: ResourcePreviewKind
  projectId: string
  promotionState: DeliverablePromotionState
  status: ArtifactStatus
  title: string
  updatedAt: number
  workspaceId: string
}

export interface DeliverableVersionContent {
  artifactId: string
  byteSize?: number
  contentType?: string
  dataBase64?: string
  editable: boolean
  fileName?: string
  previewKind: ResourcePreviewKind
  textContent?: string
  version: number
}

export interface DeliverableVersionSummary {
  artifactId: string
  byteSize?: number
  contentHash?: string
  contentType?: string
  parentVersion?: number
  previewKind: ResourcePreviewKind
  runId?: string
  sessionId?: string
  sourceMessageId?: string
  title: string
  updatedAt: number
  version: number
}

export interface EnterpriseAuthSuccess {
  session: EnterpriseSessionSummary
  workspace: WorkspaceSummary
}

export interface EnterpriseLoginRequest {
  clientAppId: string
  password: string
  username: string
  workspaceId?: string
}

export interface EnterprisePrincipal {
  displayName: string
  status: string
  userId: string
  username: string
}

export interface EnterpriseSessionSummary {
  clientAppId: string
  createdAt: number
  expiresAt?: number
  principal: EnterprisePrincipal
  sessionId: string
  status: string
  token: string
  workspaceId: string
}

export interface ExportWorkspaceAgentBundleInput {
  agentIds: string[]
  mode: string
  teamIds: string[]
}

export interface ExportWorkspaceAgentBundleResult {
  agentCount: number
  avatarCount: number
  bundleManifest: AssetBundleManifest
  fileCount: number
  files: WorkspaceDirectoryUploadEntry[]
  issues: ImportIssue[]
  mcpCount: number
  rootDirName: string
  skillCount: number
  teamCount: number
  translationReport: AssetTranslationReport
}

export interface FeatureDefinition {
  code: string
  id: string
  label: string
  requiredPermissionCodes: string[]
}

export interface ForkDeliverableInput {
  projectId?: string
  title?: string
}

export interface HealthcheckBackendStatus {
  state: BackendConnectionState
  transport: BackendTransport
}

export interface HealthcheckStatus {
  backend: HealthcheckBackendStatus
  cargoWorkspace: boolean
  host: HostPlatform
  mode: HostExecutionMode
  status: "ok"
}

export interface HostBackendConnection {
  authToken?: string
  baseUrl?: string
  state: BackendConnectionState
  transport: BackendTransport
}

export type HostExecutionMode = "local" | "remote"

export type HostPlatform = "tauri" | "web"

export interface HostReleaseSummary {
  channel: HostUpdateChannel
  notes?: string
  notesUrl?: string
  publishedAt: string
  version: string
}

export interface HostState {
  appVersion: string
  cargoWorkspace: boolean
  mode: HostExecutionMode
  platform: HostPlatform
  shell: string
}

export interface HostUpdateCapabilities {
  canCheck: boolean
  canDownload: boolean
  canInstall: boolean
  supportsChannels: boolean
}

export type HostUpdateChannel = "formal" | "preview"

export interface HostUpdateCheckRequest {
  channel?: HostUpdateChannel
}

export interface HostUpdateProgress {
  downloadedBytes: number
  percent: number
  totalBytes: number
}

export type HostUpdateState = "idle" | "checking" | "up_to_date" | "update_available" | "downloading" | "downloaded" | "installing" | "error"

export interface HostUpdateStatus {
  capabilities: HostUpdateCapabilities
  currentChannel: HostUpdateChannel
  currentVersion: string
  errorCode?: string
  errorMessage?: string
  lastCheckedAt?: number
  latestRelease?: HostReleaseSummary
  progress?: HostUpdateProgress
  state: HostUpdateState
}

export interface ImportedAgentPreviewItem {
  action: "create" | "update" | "skip" | "failed"
  agentId?: string
  department: string
  manifestRevision: string
  mcpServerNames: string[]
  name: string
  skillSlugs: string[]
  sourceId: string
  taskDomains: string[]
  translationMode: "native" | "translate" | "downgrade" | "reject"
}

export interface ImportedAvatarPreviewItem {
  fileName: string
  generated: boolean
  ownerKind: string
  ownerName: string
  sourceId: string
}

export interface ImportedMcpPreviewItem {
  action: "create" | "update" | "skip" | "failed"
  consumerNames: string[]
  contentHash?: string
  referencedOnly: boolean
  serverName: string
  sourceIds: string[]
}

export interface ImportedSkillPreviewItem {
  action: "create" | "update" | "skip" | "failed"
  agentNames: string[]
  contentHash: string
  departments: string[]
  fileCount: number
  name: string
  skillId: string
  slug: string
  sourceIds: string[]
}

export interface ImportedTeamPreviewItem {
  action: "create" | "update" | "skip" | "failed"
  agentSourceIds: string[]
  leaderName?: string
  manifestRevision: string
  memberNames: string[]
  name: string
  sourceId: string
  taskDomains: string[]
  teamId?: string
  translationMode: "native" | "translate" | "downgrade" | "reject"
}

export interface ImportIssue {
  assetKind?: string
  code: string
  dependencyRef?: string
  details?: Record<string, unknown>
  message: string
  scope: "bundle" | "agent" | "team" | "skill" | "mcp" | "avatar"
  severity: "info" | "warning" | "error"
  sourceId?: string
  sourcePath?: string
  stage: "parse" | "validate" | "translate" | "execute"
  suggestion?: string
}

export interface ImportWorkspaceAgentBundleInput {
  files: WorkspaceDirectoryUploadEntry[]
}

export interface ImportWorkspaceAgentBundlePreview {
  agentCount: number
  agents: ImportedAgentPreviewItem[]
  avatarCount: number
  avatars: ImportedAvatarPreviewItem[]
  bundleManifest: AssetBundleManifest
  createCount: number
  departmentCount: number
  departments: string[]
  detectedAgentCount: number
  detectedTeamCount: number
  failureCount: number
  filteredFileCount: number
  importableAgentCount: number
  importableTeamCount: number
  issues: ImportIssue[]
  mcpCount: number
  mcps: ImportedMcpPreviewItem[]
  skillCount: number
  skills: ImportedSkillPreviewItem[]
  skipCount: number
  teamCount: number
  teams: ImportedTeamPreviewItem[]
  translationReport: AssetTranslationReport
  uniqueMcpCount: number
  uniqueSkillCount: number
  updateCount: number
}

export interface ImportWorkspaceAgentBundlePreviewInput {
  files: WorkspaceDirectoryUploadEntry[]
}

export interface ImportWorkspaceAgentBundleResult {
  agentCount: number
  agents: ImportedAgentPreviewItem[]
  avatarCount: number
  avatars: ImportedAvatarPreviewItem[]
  bundleManifest: AssetBundleManifest
  createCount: number
  departmentCount: number
  departments: string[]
  detectedAgentCount: number
  detectedTeamCount: number
  failureCount: number
  filteredFileCount: number
  importableAgentCount: number
  importableTeamCount: number
  issues: ImportIssue[]
  mcpCount: number
  mcps: ImportedMcpPreviewItem[]
  skillCount: number
  skills: ImportedSkillPreviewItem[]
  skipCount: number
  teamCount: number
  teams: ImportedTeamPreviewItem[]
  translationReport: AssetTranslationReport
  uniqueMcpCount: number
  uniqueSkillCount: number
  updateCount: number
}

export interface ImportWorkspaceSkillArchiveInput {
  archive: WorkspaceFileUploadPayload
  slug: string
}

export interface ImportWorkspaceSkillFolderInput {
  files: WorkspaceDirectoryUploadEntry[]
  slug: string
}

export interface InboxItemRecord {
  actionable: boolean
  actionLabel?: string
  createdAt: number
  description: string
  id: string
  itemType: string
  priority: string
  projectId?: string
  routeTo?: string
  status: string
  targetUserId: string
  title: string
  workspaceId: string
}

export interface KnowledgeEntryRecord {
  id: string
  projectId?: string
  scope: string
  sourceRef: string
  sourceType: string
  status: string
  title: string
  updatedAt: number
  workspaceId: string
}

export type KnowledgeKind = "private" | "shared" | "candidate"

export type KnowledgePlaneScope = "personal" | "project" | "workspace"

export interface KnowledgeRecord {
  id: string
  kind: KnowledgeKind
  ownerUserId?: string
  projectId?: string
  scope?: KnowledgePlaneScope
  sourceRef: string
  sourceType: KnowledgeSourceType
  status: KnowledgeStatus
  summary: string
  title: string
  updatedAt: number
  visibility?: KnowledgeVisibilityMode
  workspaceId: string
}

export type KnowledgeSourceType = "conversation" | "artifact" | "run"

export type KnowledgeStatus = "candidate" | "reviewed" | "shared" | "archived"

export type KnowledgeVisibilityMode = "private" | "public"

export interface LaunchTaskRequest {
  actorRef?: string
}

export type Locale = "zh-CN" | "en-US"

export interface MailboxPolicy {
  allowWorkerToWorker: boolean
  mode: "leader-hub"
  retainMessages: boolean
}

export type McpServerPackageManifest = CapabilityAssetManifest & unknown

export interface MemoryPolicy {
  allowWorkspaceSharedWrite: boolean
  durableScopes: string[]
  freshnessRequired: boolean
  maxSelections: number
  writeRequiresApproval: boolean
}

export interface MenuDefinition {
  featureCode: string
  id: string
  label: string
  order: number
  parentId?: string
  routeName?: string
  source: string
  status: string
}

export interface MenuGateResult {
  allowed: boolean
  featureCode: string
  menuId: string
  reason?: string
}

export interface MenuPolicyRecord {
  enabled: boolean
  group?: string
  menuId: string
  order: number
  visibility: string
}

export interface MenuPolicyUpsertRequest {
  enabled: boolean
  group?: string
  order: number
  visibility: string
}

export type MenuSource = "main-sidebar" | "console" | "access-control"

export type MenuStatus = "active" | "disabled"

export interface MessageProcessEntry {
  detail: string
  id: string
  timestamp: number
  title: string
  toolId?: string
  type: "thinking" | "execution" | "tool" | "result"
}

export interface MessageToolCall {
  count: number
  kind: ToolCatalogKind
  label: string
  toolId: string
}

export interface MessageUsage {
  inputTokens: number
  outputTokens: number
  totalTokens: number
}

export interface ModelCatalogSnapshot {
  configuredModels: ConfiguredModelRecord[]
  credentialBindings: CredentialBinding[]
  defaultSelections: Record<string, DefaultSelection>
  diagnostics: ModelRegistryDiagnostics
  models: ModelRegistryRecord[]
  providers: ProviderRegistryRecord[]
}

export interface ModelRegistryDiagnostics {
  errors: string[]
  warnings: string[]
}

export interface ModelRegistryRecord {
  availability: string
  capabilities: CapabilityDescriptor[]
  contextWindow?: number
  defaultPermission: string
  description: string
  enabled: boolean
  family: string
  label: string
  maxOutputTokens?: number
  metadata: Record<string, unknown>
  modelId: string
  providerId: string
  recommendedFor: string
  surfaceBindings: ModelSurfaceBinding[]
  track: string
}

export interface ModelSurfaceBinding {
  enabled: boolean
  executionProfile: RuntimeExecutionProfile
  protocolFamily: string
  surface: string
}

export interface NotificationFilter {
  scope?: NotificationFilterScope
}

export type NotificationFilterScope = "all" | "app" | "workspace" | "user"

export type NotificationLevel = "info" | "success" | "warning" | "error"

export interface NotificationListResponse {
  notifications: NotificationRecord[]
  unread: NotificationUnreadSummary
}

export interface NotificationRecord {
  actionLabel?: string
  body: string
  createdAt: number
  id: string
  level: NotificationLevel
  readAt?: number
  routeTo?: string
  scopeKind: NotificationScopeKind
  scopeOwnerId?: string
  source: string
  title: string
  toastVisibleUntil?: number
}

export type NotificationScopeKind = "app" | "workspace" | "user"

export interface NotificationUnreadScopeSummary {
  app: number
  user: number
  workspace: number
}

export interface NotificationUnreadSummary {
  byScope: NotificationUnreadScopeSummary
  total: number
}

export interface OrgUnitRecord {
  code: string
  id: string
  name: string
  parentId?: string
  status: string
}

export interface OrgUnitUpsertRequest {
  code: string
  name: string
  parentId?: string
  status: string
}

export interface OutputContract {
  artifactKinds: string[]
  preserveLineage: boolean
  primaryFormat: "markdown" | "json" | "artifact" | "mixed"
  requireStructuredSummary: boolean
}

export type PasswordState = "set" | "reset-required" | "temporary"

export interface PermissionDefinition {
  actions: string[]
  category: string
  code: string
  description: string
  name: string
  resourceType: string
}

export interface PermissionEnvelope {
  allowedResourceScopes: string[]
  defaultMode: "readonly" | "workspace-write" | "danger-full-access"
  escalationAllowed: boolean
  maxMode: "readonly" | "workspace-write" | "danger-full-access"
}

export type PermissionMode = "auto" | "readonly" | "danger-full-access"

export type PetChatSender = "user" | "pet"

export type PetContextScope = "home" | "project"

export interface PetConversationBinding {
  contextScope: PetContextScope
  conversationId: string
  ownerUserId: string
  petId: string
  projectId?: string
  sessionId?: string
  updatedAt: number
  workspaceId: string
}

export interface PetDashboardSummary {
  activeConversationCount: number
  knowledgeCount: number
  lastInteractionAt?: number
  memoryCount: number
  mood: PetMood
  ownerUserId: string
  petId: string
  reminderCount: number
  resourceCount: number
  species: PetSpecies
  workspaceId: string
}

export interface PetMessage {
  content: string
  id: string
  petId: string
  sender: PetChatSender
  timestamp: number
}

export type PetMood = "curious" | "happy" | "sleepy" | "playful" | "focused"

export type PetMotionState = "idle" | "hover" | "walk" | "chat" | "sleep"

export interface PetPosition {
  x: number
  y: number
}

export interface PetPresenceState {
  chatOpen: boolean
  isVisible: boolean
  lastInteractionAt: number
  motionState: PetMotionState
  petId: string
  position: PetPosition
  unreadCount: number
}

export interface PetProfile {
  avatarLabel: string
  displayName: string
  fallbackAsset: string
  favoriteSnack: string
  greeting: string
  id: string
  mood: PetMood
  ownerUserId: string
  promptHints: string[]
  riveAsset?: string
  species: PetSpecies
  stateMachine?: string
  summary: string
}

export type PetSpecies = "duck" | "goose" | "blob" | "cat" | "dragon" | "octopus" | "owl" | "penguin" | "turtle" | "snail" | "ghost" | "axolotl" | "capybara" | "cactus" | "robot" | "rabbit" | "mushroom" | "chonk"

export interface PetWorkspaceSnapshot {
  binding?: PetConversationBinding
  contextScope: PetContextScope
  messages: PetMessage[]
  ownerUserId: string
  presence: PetPresenceState
  profile: PetProfile
  projectId?: string
  workspaceId: string
}

export interface PositionRecord {
  code: string
  id: string
  name: string
  status: string
}

export interface PositionUpsertRequest {
  code: string
  name: string
  status: string
}

export interface ProjectAgentAssignments {
  agentIds: string[]
  excludedAgentIds: string[]
  excludedTeamIds: string[]
  teamIds: string[]
}

export interface ProjectAgentLinkInput {
  agentId: string
  projectId: string
}

export interface ProjectAgentLinkRecord {
  agentId: string
  linkedAt: number
  projectId: string
  workspaceId: string
}

export type ProjectAssetType = "agent" | "resource" | "knowledge" | "tool.skill" | "tool.mcp"

export interface ProjectDashboardBreakdownItem {
  helper?: string
  id: string
  label: string
  value: number
}

export interface ProjectDashboardConversationInsight {
  approvalCount: number
  conversationId: string
  id: string
  lastMessagePreview?: string
  messageCount: number
  status: string
  title: string
  tokenCount: number
  toolCallCount: number
  updatedAt: number
}

export interface ProjectDashboardRankingItem {
  helper?: string
  id: string
  label: string
  value: number
}

export interface ProjectDashboardSnapshot {
  conversationInsights: ProjectDashboardConversationInsight[]
  metrics: WorkspaceMetricRecord[]
  modelBreakdown: ProjectDashboardBreakdownItem[]
  overview: ProjectDashboardSummary
  project: ProjectRecord
  recentActivity: WorkspaceActivityRecord[]
  recentConversations: ConversationRecord[]
  recentTasks: TaskSummary[]
  resourceBreakdown: ProjectDashboardBreakdownItem[]
  toolRanking: ProjectDashboardRankingItem[]
  trend: ProjectDashboardTrendPoint[]
  usedTokens: number
  userStats: ProjectDashboardUserStat[]
}

export interface ProjectDashboardSummary {
  activeTaskCount: number
  activeUserCount: number
  activityCount: number
  agentCount: number
  approvalCount: number
  attentionTaskCount: number
  conversationCount: number
  knowledgeCount: number
  memberCount: number
  messageCount: number
  resourceCount: number
  scheduledTaskCount: number
  taskCount: number
  teamCount: number
  tokenRecordCount: number
  toolCallCount: number
  toolCount: number
  totalTokens: number
}

export interface ProjectDashboardTrendPoint {
  approvalCount: number
  conversationCount: number
  id: string
  label: string
  messageCount: number
  timestamp: number
  tokenCount: number
  toolCallCount: number
}

export interface ProjectDashboardUserStat {
  activityCount: number
  activityTrend: number[]
  approvalCount: number
  conversationCount: number
  displayName: string
  messageCount: number
  tokenCount: number
  tokenTrend: number[]
  toolCallCount: number
  userId: string
}

export interface ProjectDefaultPermissions {
  agents: ProjectDefaultPermissionValue
  knowledge: ProjectDefaultPermissionValue
  resources: ProjectDefaultPermissionValue
  tasks: ProjectDefaultPermissionValue
  tools: ProjectDefaultPermissionValue
}

export type ProjectDefaultPermissionValue = "allow" | "deny"

export interface ProjectDeletionRequest {
  createdAt: number
  id: string
  projectId: string
  reason?: string
  requestedByUserId: string
  reviewComment?: string
  reviewedAt?: number
  reviewedByUserId?: string
  status: ProjectDeletionRequestStatus
  updatedAt: number
  workspaceId: string
}

export type ProjectDeletionRequestStatus = "pending" | "approved" | "rejected"

export interface ProjectLinkedWorkspaceAssets {
  agentIds: string[]
  knowledgeIds: string[]
  resourceIds: string[]
  toolSourceKeys: string[]
}

export interface ProjectModelAssignments {
  configuredModelIds: string[]
  defaultConfiguredModelId: string
}

export interface ProjectPermissionOverrides {
  agents: ProjectPermissionOverrideValue
  knowledge: ProjectPermissionOverrideValue
  resources: ProjectPermissionOverrideValue
  tasks: ProjectPermissionOverrideValue
  tools: ProjectPermissionOverrideValue
}

export type ProjectPermissionOverrideValue = "inherit" | "allow" | "deny"

export interface ProjectPromotionRequest {
  assetId: string
  assetType: ProjectAssetType
  createdAt: number
  id: string
  projectId: string
  requestedByUserId: string
  requiredWorkspaceCapability: string
  reviewComment?: string
  reviewedAt?: number
  reviewedByUserId?: string
  status: ProjectPromotionRequestStatus
  submittedByOwnerUserId: string
  updatedAt: number
  workspaceId: string
}

export type ProjectPromotionRequestStatus = "pending" | "approved" | "rejected"

export interface ProjectRecord {
  description: string
  id: string
  leaderAgentId?: string
  managerUserId?: string
  memberUserIds: string[]
  name: string
  ownerUserId: string
  permissionOverrides: ProjectPermissionOverrides
  presetCode?: string
  resourceDirectory: string
  status: "active" | "archived"
  workspaceId: string
}

export type ProjectResourceKind = "file" | "folder" | "artifact" | "url"

export type ProjectResourceOrigin = "source" | "generated"

export interface ProjectTeamLinkInput {
  projectId: string
  teamId: string
}

export interface ProjectTeamLinkRecord {
  linkedAt: number
  projectId: string
  teamId: string
  workspaceId: string
}

export interface ProjectTokenUsageRecord {
  projectId: string
  projectName: string
  usedTokens: number
}

export interface ProjectToolAssignments {
  excludedSourceKeys: string[]
  sourceKeys: string[]
}

export interface ProjectWorkspaceAssignments {
  agents?: ProjectAgentAssignments
  models?: ProjectModelAssignments
  tools?: ProjectToolAssignments
}

export interface PromoteDeliverableInput {
  kind?: KnowledgeKind
  summary?: string
  title?: string
}

export interface PromoteWorkspaceResourceInput {
  scope: WorkspaceResourceScope
}

export interface ProtectedResourceDescriptor {
  classification: string
  id: string
  name: string
  ownerSubjectId?: string
  ownerSubjectType?: string
  projectId?: string
  resourceSubtype?: string
  resourceType: string
  tags: string[]
}

export interface ProtectedResourceMetadataUpsertRequest {
  classification: string
  ownerSubjectId?: string
  ownerSubjectType?: string
  projectId?: string
  resourceSubtype?: string
  tags: string[]
}

export interface ProviderConfig {
  baseUrl?: string
  credentialRef?: string
  defaultModel?: string
  defaultSurface?: string
  protocolFamily?: string
  providerId: string
}

export interface ProviderRegistryRecord {
  enabled: boolean
  label: string
  metadata: Record<string, unknown>
  providerId: string
  surfaces: SurfaceDescriptor[]
}

export interface RebindRuntimeSessionConfiguredModelInput {
  selectedConfiguredModelId: string
}

export interface RegisterBootstrapAdminRequest {
  avatar: AvatarUploadPayload
  clientAppId: string
  confirmPassword: string
  displayName: string
  mappedDirectory?: string
  password: string
  username: string
  workspaceId?: string
}

export interface RerunTaskRequest {
  actorRef?: string
  sourceTaskRunId?: string | null
}

export interface ResolvedExecutionTarget {
  baseUrl?: string
  capabilities: CapabilityDescriptor[]
  configuredModelId: string
  configuredModelName: string
  credentialRef?: string
  credentialSource: string
  executionProfile: RuntimeExecutionProfile
  modelId: string
  protocolFamily: string
  providerId: string
  registryModelId: string
  requestPolicy: ResolvedRequestPolicyInput
  surface: string
}

export interface ResolvedRequestPolicyInput {
  authStrategy: string
  baseUrlPolicy: string
  configuredBaseUrl?: string
  defaultBaseUrl: string
  providerBaseUrl?: string
}

export interface ResolveRuntimeApprovalInput {
  decision: DecisionAction
}

export interface ResolveRuntimeAuthChallengeInput {
  note?: string
  resolution: "resolved" | "failed" | "cancelled"
}

export interface ResolveRuntimeMemoryProposalInput {
  decision: RuntimeMemoryProposalDecisionAction
  note?: string
}

export interface ResourceActionGrant {
  actions: string[]
  resourceType: string
}

export interface ResourcePolicyRecord {
  action: string
  effect: string
  id: string
  resourceId: string
  resourceType: string
  subjectId: string
  subjectType: string
}

export interface ResourcePolicyUpsertRequest {
  action: string
  effect: string
  resourceId: string
  resourceType: string
  subjectId: string
  subjectType: string
}

export type ResourcePreviewKind = "text" | "code" | "markdown" | "image" | "pdf" | "audio" | "video" | "folder" | "binary" | "url"

export interface ReviewProjectDeletionRequestInput {
  reviewComment?: string
}

export interface ReviewProjectPromotionRequestInput {
  approved: boolean
  reviewComment?: string
}

export type RiskLevel = "low" | "medium" | "high"

export interface RoleBindingRecord {
  effect: string
  id: string
  roleId: string
  subjectId: string
  subjectType: string
}

export interface RoleBindingUpsertRequest {
  effect: string
  roleId: string
  subjectId: string
  subjectType: string
}

export interface RoleUpsertRequest {
  code: string
  description: string
  name: string
  permissionCodes: string[]
  status: string
}

export interface RunRuntimeGenerationInput {
  configuredModelId: string
  content: string
  projectId?: string
  systemPrompt?: string
}

export type RunStatus = "idle" | "draft" | "planned" | "running" | "waiting_input" | "waiting_approval" | "blocked" | "paused" | "completed" | "failed" | "terminated"

export type RuntimeActorType = "user" | "assistant" | "system"

export interface RuntimeApprovalRequest {
  approvalLayer: string
  capabilityId?: string
  checkpointRef?: string
  conversationId: string
  createdAt: number
  detail: string
  escalationReason: string
  id: string
  providerKey?: string
  requiredPermission?: string
  requiresApproval: boolean
  requiresAuth: boolean
  riskLevel: RiskLevel
  runId: string
  sessionId: string
  status: "pending" | "approved" | "rejected"
  summary: string
  targetKind: string
  targetRef: string
  toolName: string
}

export interface RuntimeAuthChallengeSummary {
  approvalLayer: string
  capabilityId?: string
  checkpointRef?: string
  conversationId: string
  createdAt: number
  detail: string
  escalationReason: string
  id: string
  providerKey?: string
  requiredPermission?: string
  requiresApproval: boolean
  requiresAuth: boolean
  resolution?: string
  runId: string
  sessionId: string
  status: "pending" | "resolved" | "failed" | "cancelled"
  summary: string
  targetKind: string
  targetRef: string
  toolName?: string
}

export interface RuntimeAuthStateSummary {
  challengedProviderKeys: string[]
  failedProviderKeys: string[]
  lastChallengeAt?: number
  pendingChallengeCount: number
  resolvedProviderKeys: string[]
}

export interface RuntimeBackgroundRunSummary {
  backgroundCapable: boolean
  blocking?: RuntimeWorkflowBlockingSummary
  continuationState: string
  runId: string
  status: string
  updatedAt: number
  workflowRunId?: string
}

export interface RuntimeBootstrap {
  provider: ProviderConfig
  sessions: RuntimeSessionSummary[]
}

export interface RuntimeCapabilityExecutionOutcome {
  capabilityId?: string
  concurrencyPolicy?: string
  detail?: string
  dispatchKind?: string
  outcome: string
  providerKey?: string
  requiresApproval: boolean
  requiresAuth: boolean
  toolName?: string
}

export interface RuntimeCapabilityPlanSummary {
  activatedTools: string[]
  approvedTools: string[]
  authResolvedTools: string[]
  availableResources: string[]
  deferredTools: string[]
  discoverableSkills: string[]
  discoveredTools: string[]
  exposedTools: string[]
  grantedTools: string[]
  hiddenCapabilities: string[]
  pendingTools: string[]
  providerFallbacks: string[]
  visibleTools: string[]
}

export interface RuntimeCapabilityProviderState {
  degraded: boolean
  detail?: string
  providerKey: string
  state: string
}

export interface RuntimeCapabilityStateSnapshot {
  activatedTools: string[]
  approvedTools: string[]
  authResolvedTools: string[]
  discoveredTools: string[]
  effortOverride?: string
  exposedTools: string[]
  grantedToolCount: number
  grantedTools: string[]
  hiddenTools: string[]
  injectedSkillMessageCount: number
  modelOverride?: string
  pendingTools: string[]
}

export interface RuntimeCapabilitySummary {
  activatedTools: string[]
  approvedTools: string[]
  authResolvedTools: string[]
  availableResources: string[]
  deferredTools: string[]
  discoverableSkills: string[]
  discoveredTools: string[]
  exposedTools: string[]
  grantedTools: string[]
  hiddenCapabilities: string[]
  pendingTools: string[]
  providerFallbacks: string[]
  visibleTools: string[]
}

export interface RuntimeCapabilitySurface {
  availableResources: string[]
  deferredTools: string[]
  discoverableSkills: string[]
  hiddenCapabilities: string[]
  visibleTools: string[]
}

export interface RuntimeConfigPatch {
  configuredModelCredentials?: RuntimeConfiguredModelCredentialInput[]
  patch: Record<string, unknown>
  scope: RuntimeConfigScope
}

export type RuntimeConfigScope = "workspace" | "project" | "user"

export interface RuntimeConfigSource {
  contentHash?: string
  displayPath: string
  document?: Record<string, unknown>
  exists: boolean
  loaded: boolean
  ownerId?: string
  scope: RuntimeConfigScope
  sourceKey: string
}

export interface RuntimeConfiguredModelCredentialInput {
  apiKey: string
  configuredModelId: string
}

export interface RuntimeConfiguredModelProbeInput {
  apiKey?: string
  configuredModelId: string
  patch: Record<string, unknown>
  scope: RuntimeConfigScope
}

export interface RuntimeConfiguredModelProbeResult {
  configuredModelId: string
  configuredModelName?: string
  consumedTokens?: number
  errors: string[]
  reachable: boolean
  requestId?: string
  valid: boolean
  warnings: string[]
}

export interface RuntimeConfigValidationResult {
  errors: string[]
  valid: boolean
  warnings: string[]
}

export interface RuntimeEffectiveConfig {
  effectiveConfig: Record<string, unknown>
  effectiveConfigHash: string
  secretReferences: RuntimeSecretReferenceStatus[]
  sources: RuntimeConfigSource[]
  validation: RuntimeConfigValidationResult
}

export interface RuntimeEventEnvelope {
  actorRef?: string
  approval?: RuntimeApprovalRequest
  approvalLayer?: string
  authChallenge?: RuntimeAuthChallengeSummary
  capabilityPlanSummary?: RuntimeCapabilityPlanSummary
  capabilityStateRef?: string
  conversationId: string
  decision?: DecisionAction
  emittedAt: number
  error?: string
  eventType: RuntimeEventKind
  freshnessSummary?: RuntimeMemoryFreshnessSummary
  id: string
  iteration?: number
  kind?: string
  lastExecutionOutcome?: RuntimeCapabilityExecutionOutcome
  lastMediationOutcome?: RuntimeMediationOutcome
  memoryProposal?: RuntimeMemoryProposal
  memorySelectionSummary?: RuntimeMemorySelectionSummary
  message?: RuntimeMessage
  outcome?: string
  parentRunId?: string
  pendingMediation?: RuntimePendingMediation
  projectId?: string
  providerStateSummary?: RuntimeCapabilityProviderState[]
  run?: RuntimeRunSnapshot
  runId?: string
  selectedMemory?: RuntimeSelectedMemoryItem[]
  sequence: number
  sessionId: string
  summary?: RuntimeSessionSummary
  targetKind?: string
  targetRef?: string
  toolUseId?: string
  trace?: RuntimeTraceItem
  workflowRunId?: string
  workflowStepId?: string
  workspaceId: string
}

export type RuntimeEventKind = "planner.started" | "planner.completed" | "model.started" | "model.delta" | "model.tool_use" | "model.usage" | "model.completed" | "model.failed" | "tool.requested" | "tool.started" | "tool.completed" | "tool.failed" | "skill.requested" | "skill.started" | "skill.completed" | "skill.failed" | "mcp.requested" | "mcp.started" | "mcp.completed" | "mcp.failed" | "approval.requested" | "approval.resolved" | "approval.cancelled" | "auth.challenge_requested" | "auth.resolved" | "auth.failed" | "policy.exposure_denied" | "policy.surface_deferred" | "policy.session_compiled" | "trace.emitted" | "subrun.spawned" | "subrun.cancelled" | "subrun.completed" | "subrun.failed" | "workflow.started" | "workflow.step.started" | "workflow.step.completed" | "workflow.completed" | "workflow.failed" | "background.started" | "background.paused" | "background.completed" | "background.failed" | "runtime.session.started" | "runtime.message.user" | "runtime.message.assistant" | "runtime.tool.executed" | "runtime.render.block" | "runtime.ask" | "runtime.checkpoint.created" | "runtime.session.ended" | "runtime.session.plugins_snapshot" | "runtime.run.updated" | "runtime.message.created" | "runtime.trace.emitted" | "runtime.approval.requested" | "runtime.approval.resolved" | "memory.selected" | "memory.proposed" | "memory.approved" | "memory.rejected" | "memory.revalidated" | "runtime.session.updated" | "runtime.error"

export type RuntimeExecutionClass = "unsupported" | "single_shot_generation" | "agent_conversation"

export interface RuntimeExecutionProfile {
  executionClass: RuntimeExecutionClass
  toolLoop: boolean
  upstreamStreaming: boolean
}

export interface RuntimeGenerationResult {
  configuredModelId: string
  configuredModelName: string
  consumedTokens?: number
  content: string
  requestId?: string
}

export interface RuntimeHandoffSummary {
  artifactRefs: ArtifactVersionReference[]
  handoffRef: string
  mailboxRef: string
  receiverActorRef: string
  senderActorRef: string
  state: string
  updatedAt: number
}

export interface RuntimeMailboxSummary {
  channel: string
  mailboxRef: string
  pendingCount: number
  status: string
  totalMessages: number
  updatedAt: number
}

export interface RuntimeMediationOutcome {
  approvalLayer?: string
  capabilityId?: string
  checkpointRef?: string
  detail?: string
  mediationId?: string
  mediationKind: string
  outcome: string
  providerKey?: string
  reason?: string
  requiresApproval: boolean
  requiresAuth: boolean
  resolvedAt?: number
  targetKind: string
  targetRef: string
  toolName?: string
}

export type RuntimeMemoryFreshnessState = "fresh" | "revalidated" | "stale" | "unknown"

export interface RuntimeMemoryFreshnessSummary {
  freshCount: number
  freshnessRequired: boolean
  staleCount: number
}

export type RuntimeMemoryIntent = "user" | "feedback" | "project" | "reference"

export type RuntimeMemoryKind = "user" | "feedback" | "project" | "reference"

export interface RuntimeMemoryProposal {
  kind: RuntimeMemoryKind
  memoryId?: string
  proposalId: string
  proposalReason: string
  proposalState: RuntimeMemoryProposalState
  review?: RuntimeMemoryProposalReview
  scope: RuntimeMemoryScope
  sessionId: string
  sourceRunId: string
  summary: string
  title: string
}

export type RuntimeMemoryProposalDecisionAction = "approve" | "reject" | "ignore" | "revalidate"

export interface RuntimeMemoryProposalReview {
  decision: RuntimeMemoryProposalDecisionAction
  note?: string
  reviewedAt: number
  reviewerRef?: string
}

export type RuntimeMemoryProposalState = "pending" | "approved" | "rejected" | "ignored" | "revalidated"

export type RuntimeMemoryRecallMode = "default" | "skip"

export type RuntimeMemoryScope = "user" | "user-private" | "agent-private" | "project" | "project-shared" | "workspace" | "workspace-shared" | "team" | "team-shared"

export interface RuntimeMemorySelectionSummary {
  ignoredCount: number
  recallMode: RuntimeMemoryRecallMode
  selectedCount: number
  selectedMemoryIds: string[]
  totalCandidateCount: number
}

export interface RuntimeMemorySummary {
  durableMemoryCount: number
  selectedMemoryIds: string[]
  summary: string
}

export interface RuntimeMessage {
  artifacts?: ArtifactVersionReference[]
  attachments?: string[]
  configuredModelId?: string
  configuredModelName?: string
  content: string
  conversationId: string
  deliverableRefs?: ArtifactVersionReference[]
  id: string
  modelId?: string
  processEntries?: MessageProcessEntry[]
  requestedActorId?: string
  requestedActorKind?: ConversationActorKind
  resolvedActorId?: string
  resolvedActorKind?: ConversationActorKind
  resolvedActorLabel?: string
  resourceIds?: string[]
  senderLabel: string
  senderType: RuntimeActorType
  sessionId: string
  status: RunStatus
  timestamp: number
  toolCalls?: MessageToolCall[]
  usage?: MessageUsage
  usedDefaultActor?: boolean
}

export interface RuntimePendingMediation {
  approvalId?: string
  approvalLayer?: string
  authChallengeId?: string
  capabilityId?: string
  checkpointRef?: string
  detail?: string
  escalationReason?: string
  mediationId?: string
  mediationKind: string
  providerKey?: string
  reason?: string
  requiredPermission?: string
  requiresApproval: boolean
  requiresAuth: boolean
  state: string
  summary?: string
  targetKind: string
  targetRef: string
  toolName?: string
}

export type RuntimePendingMediationSummary = RuntimePendingMediation

export type RuntimePermissionMode = "read-only" | "workspace-write" | "danger-full-access"

export interface RuntimePolicyDecisionSummary {
  allowCount: number
  approvalRequiredCount: number
  authRequiredCount: number
  compiledAt?: number
  deferredCapabilityCount: number
  deniedExposureCount: number
  hiddenCapabilityCount: number
}

export interface RuntimePolicySnapshot {
  approvalPreference?: ApprovalPreference
  capabilityPolicy?: CapabilityPolicy
  configSnapshotId: string
  delegationPolicy?: DelegationPolicy
  executionPermissionMode: RuntimePermissionMode
  manifestRevision: string
  memoryPolicy?: MemoryPolicy
  selectedActorRef: string
  selectedConfiguredModelId: string
}

export interface RuntimeRunCheckpoint {
  approvalLayer?: string
  brokerDecision?: string
  capabilityId?: string
  capabilityPlanSummary: RuntimeCapabilityPlanSummary
  capabilityStateRef?: string
  checkpointArtifactRef?: string
  currentIterationIndex: number
  lastExecutionOutcome?: RuntimeCapabilityExecutionOutcome
  lastMediationOutcome?: RuntimeMediationOutcome
  pendingApproval?: RuntimeApprovalRequest
  pendingAuthChallenge?: RuntimeAuthChallengeSummary
  pendingMediation?: RuntimePendingMediation
  providerKey?: string
  reason?: string
  requiredPermission?: string
  requiresApproval?: boolean
  requiresAuth?: boolean
  targetKind?: string
  targetRef?: string
  usageSummary: RuntimeUsageSummary
}

export type RuntimeRunKind = "primary" | "subrun"

export interface RuntimeRunSnapshot {
  actorRef: string
  approvalState?: string
  approvalTarget?: RuntimeApprovalRequest
  artifactRefs: ArtifactVersionReference[]
  authTarget?: RuntimeAuthChallengeSummary
  backgroundState?: string
  capabilityPlanSummary: RuntimeCapabilityPlanSummary
  capabilityStateRef?: string
  checkpoint: RuntimeRunCheckpoint
  configSnapshotId: string
  configuredModelId?: string
  configuredModelName?: string
  consumedTokens?: number
  conversationId: string
  currentStep: string
  delegatedByToolCallId?: string
  deliverableRefs?: ArtifactVersionReference[]
  effectiveConfigHash: string
  freshnessSummary?: RuntimeMemoryFreshnessSummary
  handoffRef?: string
  id: string
  lastExecutionOutcome?: RuntimeCapabilityExecutionOutcome
  lastMediationOutcome?: RuntimeMediationOutcome
  mailboxRef?: string
  memoryStateRef?: string
  modelId?: string
  nextAction?: string
  parentRunId?: string
  pendingMediation?: RuntimePendingMediation
  pendingMemoryProposal?: RuntimeMemoryProposal
  providerStateSummary: RuntimeCapabilityProviderState[]
  requestedActorId?: string
  requestedActorKind?: ConversationActorKind
  resolvedActorId?: string
  resolvedActorKind?: ConversationActorKind
  resolvedActorLabel?: string
  resolvedTarget?: ResolvedExecutionTarget
  runKind: RuntimeRunKind
  selectedMemory: RuntimeSelectedMemoryItem[]
  sessionId: string
  startedAt: number
  startedFromScopeSet: RuntimeConfigScope[]
  status: RunStatus
  traceContext: RuntimeTraceContext
  updatedAt: number
  usageSummary: RuntimeUsageSummary
  workerDispatch?: RuntimeWorkerDispatchSummary
  workflowRun?: string
  workflowRunDetail?: RuntimeWorkflowRunDetail
}

export type RuntimeSecretReferenceState = "reference-present" | "reference-missing" | "inline-redacted" | "migration-failed" | "reference-error"

export interface RuntimeSecretReferenceStatus {
  path: string
  reference?: string
  scope: RuntimeConfigScope
  status: RuntimeSecretReferenceState
}

export interface RuntimeSelectedMemoryItem {
  freshnessState: RuntimeMemoryFreshnessState
  kind: RuntimeMemoryKind
  lastValidatedAt?: number
  memoryId: string
  ownerRef?: string
  scope: RuntimeMemoryScope
  sourceRunId?: string
  summary: string
  title: string
}

export interface RuntimeSessionDetail {
  activeRunId: string
  authStateSummary: RuntimeAuthStateSummary
  backgroundRun?: RuntimeBackgroundRunSummary
  capabilityPlanSummary: RuntimeCapabilityPlanSummary
  capabilityStateRef?: string
  handoffs: RuntimeHandoffSummary[]
  lastExecutionOutcome?: RuntimeCapabilityExecutionOutcome
  manifestRevision: string
  memorySelectionSummary: RuntimeMemorySelectionSummary
  memoryStateRef: string
  memorySummary: RuntimeMemorySummary
  messages: RuntimeMessage[]
  pendingApproval?: RuntimeApprovalRequest
  pendingMailbox?: RuntimeMailboxSummary
  pendingMediation?: RuntimePendingMediation
  pendingMemoryProposalCount: number
  policyDecisionSummary: RuntimePolicyDecisionSummary
  providerStateSummary: RuntimeCapabilityProviderState[]
  run: RuntimeRunSnapshot
  selectedActorRef: string
  sessionPolicy: RuntimePolicySnapshot
  subrunCount: number
  subruns: RuntimeSubrunSummary[]
  summary: RuntimeSessionSummary
  trace: RuntimeTraceItem[]
  workflow?: RuntimeWorkflowSummary
}

export type RuntimeSessionKind = "project" | "pet"

export interface RuntimeSessionSummary {
  activeRunId: string
  authStateSummary: RuntimeAuthStateSummary
  backgroundRun?: RuntimeBackgroundRunSummary
  capabilityPlanSummary: RuntimeCapabilityPlanSummary
  capabilityStateRef?: string
  configSnapshotId: string
  conversationId: string
  effectiveConfigHash: string
  id: string
  lastExecutionOutcome?: RuntimeCapabilityExecutionOutcome
  lastMessagePreview?: string
  manifestRevision: string
  memorySelectionSummary: RuntimeMemorySelectionSummary
  memoryStateRef: string
  memorySummary: RuntimeMemorySummary
  pendingMailbox?: RuntimeMailboxSummary
  pendingMediation?: RuntimePendingMediation
  pendingMemoryProposalCount: number
  policyDecisionSummary: RuntimePolicyDecisionSummary
  projectId: string
  providerStateSummary: RuntimeCapabilityProviderState[]
  selectedActorRef: string
  sessionKind: RuntimeSessionKind
  sessionPolicy: RuntimePolicySnapshot
  startedFromScopeSet: RuntimeConfigScope[]
  status: RunStatus
  subrunCount: number
  title: string
  updatedAt: number
  workflow?: RuntimeWorkflowSummary
}

export interface RuntimeSubrunSummary {
  actorRef: string
  delegatedByToolCallId?: string
  handoffRef?: string
  label: string
  mailboxRef?: string
  parentRunId?: string
  runId: string
  runKind: RuntimeRunKind
  startedAt: number
  status: RunStatus
  updatedAt: number
  workflowRunId?: string
}

export interface RuntimeTraceContext {
  parentRunId?: string
  sessionId: string
  traceId: string
  turnId: string
}

export interface RuntimeTraceItem {
  actor: string
  actorId?: string
  actorKind?: ConversationActorKind
  conversationId: string
  detail: string
  id: string
  kind: TraceKind
  relatedMessageId?: string
  relatedToolName?: string
  runId: string
  sessionId: string
  timestamp: number
  title: string
  tone: TraceTone
}

export interface RuntimeUsageSummary {
  inputTokens: number
  outputTokens: number
  totalTokens: number
}

export interface RuntimeWorkerDispatchSummary {
  activeSubruns: number
  completedSubruns: number
  failedSubruns: number
  totalSubruns: number
}

export interface RuntimeWorkflowBlockingSummary {
  actorRef: string
  mediationKind: string
  runId: string
  state: string
  targetKind: string
}

export interface RuntimeWorkflowRunDetail {
  backgroundCapable: boolean
  blocking?: RuntimeWorkflowBlockingSummary
  completedSteps: number
  currentStepId?: string
  currentStepLabel?: string
  status: string
  steps: RuntimeWorkflowStepSummary[]
  totalSteps: number
  workflowRunId: string
}

export interface RuntimeWorkflowStepSummary {
  actorRef: string
  delegatedByToolCallId?: string
  handoffRef?: string
  label: string
  mailboxRef?: string
  nodeKind: string
  parentRunId?: string
  runId?: string
  startedAt: number
  status: string
  stepId: string
  updatedAt: number
}

export interface RuntimeWorkflowSummary {
  backgroundCapable: boolean
  completedSteps: number
  currentStepId?: string
  currentStepLabel?: string
  label: string
  status: string
  totalSteps: number
  updatedAt: number
  workflowRunId: string
}

export interface SavePetPresenceInput {
  chatOpen?: boolean
  isVisible?: boolean
  lastInteractionAt?: number
  motionState?: PetMotionState
  petId: string
  position?: PetPosition
  unreadCount?: number
}

export interface SessionRecord {
  clientAppId: string
  createdAt: number
  expiresAt?: number
  id: string
  status: SessionStatus
  token: string
  userId: string
  workspaceId: string
}

export type SessionStatus = "active" | "revoked" | "expired"

export interface SharedCapabilityPolicy {
  allowTeamInheritedCapabilities: boolean
  denyDirectMemberEscalation: boolean
  sharedCapabilityRefs: string[]
}

export interface SharedMemoryPolicy {
  requireReviewBeforePersist: boolean
  shareMode: "isolated" | "team-shared" | "project-shared"
  writableByWorkers: boolean
}

export interface ShellBootstrap {
  backend?: HostBackendConnection
  connections: ConnectionProfile[]
  hostState: HostState
  preferences: ShellPreferences
  workspaceConnections?: WorkspaceConnectionRecord[]
  workspaceSessions?: WorkspaceSessionTokenEnvelope[]
}

export interface ShellPreferences {
  compactSidebar: boolean
  defaultWorkspaceId: string
  fontFamily: string
  fontSize: number
  fontStyle: string
  lastVisitedRoute: string
  leftSidebarCollapsed: boolean
  locale: Locale
  rightSidebarCollapsed: boolean
  theme: ThemeMode
  updateChannel: HostUpdateChannel
}

export type SkillPackageManifest = CapabilityAssetManifest & unknown

export interface SubmitRuntimeTurnInput {
  content: string
  ignoredMemoryIds?: string[]
  memoryIntent?: RuntimeMemoryIntent
  permissionMode?: RuntimePermissionMode
  recallMode?: RuntimeMemoryRecallMode
}

export interface SurfaceDescriptor {
  authStrategy: string
  baseUrl: string
  baseUrlPolicy: string
  capabilities: CapabilityDescriptor[]
  enabled: boolean
  executionProfile: RuntimeExecutionProfile
  protocolFamily: string
  surface: string
  transport: string[]
}

export interface SystemAuthStatus {
  bootstrapAdminRequired: boolean
  ownerReady: boolean
  session?: EnterpriseSessionSummary
  workspace: WorkspaceSummary
}

export interface SystemBootstrapStatus {
  apiBasePath: string
  authMode: WorkspaceAuthMode
  capabilities: WorkspaceCapabilitySet
  ownerReady: boolean
  protocolVersion: string
  registeredApps: ClientAppRecord[]
  setupRequired: boolean
  transportSecurity: TransportSecurityLevel
  workspace: WorkspaceSummary
}

export interface TaskAnalyticsSummary {
  approvalRequiredCount: number
  averageRunDurationMs: number
  completionCount: number
  failureCount: number
  lastSuccessfulRunAt?: number | null
  manualRunCount: number
  runCount: number
  scheduledRunCount: number
  takeoverCount: number
}

export type TaskAttentionReason = "updated" | "needs_approval" | "failed" | "waiting_input" | "schedule_blocked" | "takeover_recommended"

export interface TaskContextBundle {
  lastResolvedAt?: number | null
  pinnedInstructions: string
  refs: TaskContextRef[]
  resolutionMode: TaskContextResolutionMode
}

export type TaskContextPinMode = "snapshot" | "follow_latest"

export interface TaskContextRef {
  kind: TaskContextRefKind
  pinMode: TaskContextPinMode
  refId: string
  subtitle?: string
  title: string
  versionRef?: string | null
}

export type TaskContextRefKind = "resource" | "knowledge" | "deliverable"

export type TaskContextResolutionMode = "explicit_only" | "explicit_plus_project_defaults"

export interface TaskDetail {
  activeRun?: TaskRunSummary | null
  activeTaskRunId?: string | null
  analyticsSummary: TaskAnalyticsSummary
  attentionReasons: TaskAttentionReason[]
  attentionUpdatedAt?: number | null
  brief: string
  contextBundle: TaskContextBundle
  createdAt: number
  createdBy: string
  defaultActorRef: string
  goal: string
  id: string
  interventionHistory: TaskInterventionRecord[]
  lastRunAt?: number | null
  latestArtifactRefs: ArtifactVersionReference[]
  latestDeliverableRefs: ArtifactVersionReference[]
  latestFailureCategory?: TaskFailureCategory | null
  latestResultSummary?: string | null
  latestTransition?: TaskStateTransitionSummary | null
  nextRunAt?: number | null
  projectId: string
  runHistory: TaskRunSummary[]
  scheduleSpec?: string | null
  status: TaskLifecycleStatus
  title: string
  updatedAt: number
  updatedBy?: string | null
  viewStatus: ViewStatus
}

export type TaskFailureCategory = "context_unavailable" | "permission_blocked" | "approval_timeout" | "runtime_error" | "model_failure" | "user_canceled"

export interface TaskInterventionRecord {
  appliedToSessionId?: string | null
  createdAt: number
  createdBy: string
  id: string
  payload: Record<string, unknown>
  status: TaskInterventionStatus
  taskId: string
  taskRunId?: string | null
  type: TaskInterventionType
}

export type TaskInterventionStatus = "accepted" | "rejected" | "applied"

export type TaskInterventionType = "takeover" | "resume" | "comment" | "approve" | "reject" | "edit_brief" | "change_actor"

export type TaskLifecycleStatus = "draft" | "ready" | "running" | "attention" | "completed" | "archived"

export type TaskRunStatus = "queued" | "running" | "waiting_input" | "waiting_approval" | "completed" | "failed" | "canceled" | "skipped"

export interface TaskRunSummary {
  actorRef: string
  artifactRefs: ArtifactVersionReference[]
  attentionReasons: TaskAttentionReason[]
  attentionUpdatedAt?: number | null
  completedAt?: number | null
  conversationId?: string | null
  deliverableRefs: ArtifactVersionReference[]
  failureCategory?: TaskFailureCategory | null
  failureSummary?: string | null
  id: string
  latestTransition?: TaskStateTransitionSummary | null
  pendingApprovalId?: string | null
  resultSummary?: string | null
  runtimeRunId?: string | null
  sessionId?: string | null
  startedAt: number
  status: TaskRunStatus
  taskId: string
  triggerType: TaskTriggerType
  viewStatus: ViewStatus
}

export interface TaskStateTransitionSummary {
  at: number
  kind: TaskTransitionKind
  runId?: string | null
  summary: string
}

export interface TaskSummary {
  activeTaskRunId?: string | null
  analyticsSummary: TaskAnalyticsSummary
  attentionReasons: TaskAttentionReason[]
  attentionUpdatedAt?: number | null
  defaultActorRef: string
  goal: string
  id: string
  lastRunAt?: number | null
  latestFailureCategory?: TaskFailureCategory | null
  latestResultSummary?: string | null
  latestTransition?: TaskStateTransitionSummary | null
  nextRunAt?: number | null
  projectId: string
  scheduleSpec?: string | null
  status: TaskLifecycleStatus
  title: string
  updatedAt: number
  viewStatus: ViewStatus
}

export type TaskTransitionKind = "created" | "launched" | "progressed" | "waiting_approval" | "completed" | "failed" | "intervened" | "skipped"

export type TaskTriggerType = "manual" | "scheduled" | "rerun" | "takeover"

export interface TeamRecord {
  approvalPreference: ApprovalPreference
  artifactHandoffPolicy: ArtifactHandoffPolicy
  avatar?: string
  avatarPath?: string
  builtinToolKeys: string[]
  capabilityPolicy: CapabilityPolicy
  defaultModelStrategy: DefaultModelStrategy
  delegationPolicy: DelegationPolicy
  dependencyResolution: AssetDependencyResolution[]
  description: string
  id: string
  importMetadata: AssetImportMetadata
  integrationSource?: {
  kind: "workspace-link" | "builtin-template"
  sourceId: string
}
  leaderRef: string
  mailboxPolicy: MailboxPolicy
  manifestRevision: string
  mcpServerNames: string[]
  memberRefs: string[]
  memoryPolicy: MemoryPolicy
  name: string
  outputContract: OutputContract
  permissionEnvelope: PermissionEnvelope
  personality: string
  projectId?: string
  prompt: string
  scope: TeamScope
  sharedCapabilityPolicy: SharedCapabilityPolicy
  sharedMemoryPolicy: SharedMemoryPolicy
  skillIds: string[]
  status: TeamStatus
  tags: string[]
  taskDomains: string[]
  teamTopology: TeamTopology
  trustMetadata: AssetTrustMetadata
  updatedAt: number
  workerConcurrencyLimit: number
  workflowAffordance: WorkflowAffordance
  workspaceId: string
}

export type TeamScope = "workspace" | "project"

export type TeamStatus = "active" | "archived"

export interface TeamTopology {
  leaderRef: string
  memberRefs: string[]
  mode: "leader-orchestrated"
}

export type ThemeMode = "light" | "dark" | "system"

export type ToolCatalogKind = "builtin" | "skill" | "mcp"

export interface ToolRecord {
  description: string
  id: string
  kind: ToolCatalogKind
  name: string
  permissionMode: WorkspaceToolPermissionMode
  status: WorkspaceToolStatus
  updatedAt: number
  workspaceId: string
}

export type TraceKind = "step" | "tool" | "approval" | "pause" | "resume" | "artifact" | "knowledge"

export type TraceTone = "info" | "success" | "warning" | "error" | "default"

export type TransportSecurityLevel = "loopback" | "trusted" | "public"

export interface UpdateCurrentUserProfileRequest {
  avatar?: AvatarUploadPayload
  displayName: string
  removeAvatar?: boolean
  username: string
}

export interface UpdateProjectRequest {
  description: string
  leaderAgentId?: string
  managerUserId?: string
  memberUserIds?: string[]
  name: string
  ownerUserId?: string
  permissionOverrides?: ProjectPermissionOverrides
  presetCode?: string
  resourceDirectory: string
  status: "active" | "archived"
}

export interface UpdateTaskRequest {
  brief?: string
  contextBundle?: TaskContextBundle
  defaultActorRef?: string
  goal?: string
  scheduleSpec?: string | null
  title?: string
}

export interface UpdateWorkspaceRequest {
  avatar?: AvatarUploadPayload
  mappedDirectory?: string
  name?: string
  removeAvatar?: boolean
}

export interface UpdateWorkspaceResourceInput {
  location?: string
  name?: string
  status?: ViewStatus
  tags?: string[]
  visibility?: WorkspaceResourceVisibility
}

export interface UpdateWorkspaceSkillFileInput {
  content: string
}

export interface UpdateWorkspaceSkillInput {
  content: string
}

export interface UpsertAgentInput {
  approvalPreference?: ApprovalPreference
  avatar?: AvatarUploadPayload
  builtinToolKeys: string[]
  capabilityPolicy?: CapabilityPolicy
  defaultModelStrategy?: DefaultModelStrategy
  delegationPolicy?: DelegationPolicy
  description: string
  mcpServerNames: string[]
  memoryPolicy?: MemoryPolicy
  name: string
  outputContract?: OutputContract
  permissionEnvelope?: PermissionEnvelope
  personality: string
  projectId?: string
  prompt: string
  removeAvatar?: boolean
  scope: AgentScope
  sharedCapabilityPolicy?: SharedCapabilityPolicy
  skillIds: string[]
  status: AgentStatus
  tags: string[]
  taskDomains?: string[]
  workspaceId: string
}

export interface UpsertTeamInput {
  approvalPreference?: ApprovalPreference
  artifactHandoffPolicy?: ArtifactHandoffPolicy
  avatar?: AvatarUploadPayload
  builtinToolKeys: string[]
  capabilityPolicy?: CapabilityPolicy
  defaultModelStrategy?: DefaultModelStrategy
  delegationPolicy?: DelegationPolicy
  description: string
  leaderRef: string
  mailboxPolicy?: MailboxPolicy
  mcpServerNames: string[]
  memberRefs?: string[]
  memoryPolicy?: MemoryPolicy
  name: string
  outputContract?: OutputContract
  permissionEnvelope?: PermissionEnvelope
  personality: string
  projectId?: string
  prompt: string
  removeAvatar?: boolean
  scope: TeamScope
  sharedCapabilityPolicy?: SharedCapabilityPolicy
  sharedMemoryPolicy?: SharedMemoryPolicy
  skillIds: string[]
  status: TeamStatus
  tags: string[]
  taskDomains?: string[]
  teamTopology?: TeamTopology
  workerConcurrencyLimit?: number
  workflowAffordance?: WorkflowAffordance
  workspaceId: string
}

export interface UpsertWorkspaceMcpServerInput {
  config: Record<string, unknown>
  serverName: string
}

export interface UserGroupRecord {
  code: string
  id: string
  name: string
  status: string
}

export interface UserGroupUpsertRequest {
  code: string
  name: string
  status: string
}

export interface UserOrgAssignmentRecord {
  isPrimary: boolean
  orgUnitId: string
  positionIds: string[]
  userGroupIds: string[]
  userId: string
}

export interface UserOrgAssignmentUpsertRequest {
  isPrimary: boolean
  orgUnitId: string
  positionIds: string[]
  userGroupIds: string[]
  userId: string
}

export interface UserRecordSummary {
  avatar?: string
  displayName: string
  id: string
  passwordState: PasswordState
  status: UserStatus
  username: string
}

export type UserStatus = "active" | "disabled"

export type ViewStatus = "healthy" | "configured" | "attention"

export interface WorkflowAffordance {
  automationCapable: boolean
  backgroundCapable: boolean
  supportedTaskKinds: string[]
}

export interface WorkspaceActivityRecord {
  actorId?: string
  actorType?: string
  description: string
  id: string
  outcome?: string
  resource?: string
  timestamp: number
  title: string
}

export type WorkspaceAuthMode = "session-token"

export interface WorkspaceBuiltinToolCatalogEntry {
  assetId?: string
  availability: ViewStatus
  builtinKey: string
  capabilityId?: string
  description: string
  disabled: boolean
  displayPath: string
  executionKind?: CapabilityExecutionKind
  id: string
  kind: "builtin"
  management: WorkspaceToolManagementCapabilities
  name: string
  requiredPermission?: "readonly" | "workspace-write" | "danger-full-access" | "null"
  sourceKey: string
  sourceKind?: CapabilitySourceKind
  workspaceId: string
}

export interface WorkspaceCapabilitySet {
  eventReplay: boolean
  idempotency: boolean
  polling: boolean
  reconnect: boolean
  sse: boolean
}

export interface WorkspaceConnectionRecord {
  authMode: WorkspaceAuthMode
  baseUrl: string
  label: string
  lastUsedAt?: number
  status: WorkspaceConnectionStatus
  transportSecurity: TransportSecurityLevel
  workspaceConnectionId: string
  workspaceId: string
}

export type WorkspaceConnectionStatus = "connected" | "disconnected" | "expired" | "unreachable"

export interface WorkspaceDirectoryBrowserEntry {
  name: string
  path: string
}

export interface WorkspaceDirectoryBrowserResponse {
  currentPath: string
  entries: WorkspaceDirectoryBrowserEntry[]
  parentPath?: string
}

export type WorkspaceDirectoryUploadEntry = WorkspaceFileUploadPayload & {
  relativePath: string
}

export interface WorkspaceFileUploadPayload {
  byteSize: number
  contentType: string
  dataBase64: string
  fileName: string
}

export interface WorkspaceMcpServerDocument {
  config: Record<string, unknown>
  displayPath: string
  scope: "builtin" | "workspace" | "project" | "user"
  serverName: string
  sourceKey: string
}

export interface WorkspaceMcpToolCatalogEntry {
  assetId?: string
  availability: ViewStatus
  capabilityId?: string
  description: string
  disabled: boolean
  displayPath: string
  endpoint: string
  executionKind?: CapabilityExecutionKind
  id: string
  kind: "mcp"
  management: WorkspaceToolManagementCapabilities
  name: string
  requiredPermission?: "readonly" | "workspace-write" | "danger-full-access" | "null"
  resourceUri?: string
  scope: "builtin" | "workspace" | "project" | "user"
  serverName: string
  sourceKey: string
  sourceKind?: CapabilitySourceKind
  statusDetail?: string
  toolNames: string[]
  workspaceId: string
}

export interface WorkspaceMetricRecord {
  helper?: string
  id: string
  label: string
  tone?: WorkspaceMetricTone
  value: string
}

export type WorkspaceMetricTone = "default" | "success" | "warning" | "error" | "info" | "accent"

export interface WorkspaceOverviewSnapshot {
  metrics: WorkspaceMetricRecord[]
  projects: ProjectRecord[]
  projectTokenUsage: ProjectTokenUsageRecord[]
  recentActivity: WorkspaceActivityRecord[]
  recentConversations: ConversationRecord[]
  workspace: WorkspaceSummary
}

export interface WorkspaceResourceChildrenRecord {
  byteSize?: number
  contentType?: string
  kind: ProjectResourceKind
  name: string
  previewKind: ResourcePreviewKind
  relativePath: string
  updatedAt: number
}

export interface WorkspaceResourceContentDocument {
  byteSize?: number
  contentType?: string
  dataBase64?: string
  externalUrl?: string
  fileName?: string
  previewKind: ResourcePreviewKind
  resourceId: string
  textContent?: string
}

export interface WorkspaceResourceFolderUploadEntry {
  byteSize: number
  contentType: string
  dataBase64: string
  fileName: string
  relativePath: string
}

export interface WorkspaceResourceImportInput {
  files: WorkspaceResourceFolderUploadEntry[]
  name: string
  rootDirName?: string
  scope: WorkspaceResourceScope
  tags?: string[]
  visibility: WorkspaceResourceVisibility
}

export interface WorkspaceResourceRecord {
  byteSize?: number
  contentType?: string
  id: string
  kind: ProjectResourceKind
  location?: string
  name: string
  origin: ProjectResourceOrigin
  ownerUserId: string
  previewKind: ResourcePreviewKind
  projectId?: string
  scope: WorkspaceResourceScope
  sourceArtifactId?: string
  status: ViewStatus
  storagePath?: string
  tags: string[]
  updatedAt: number
  visibility: WorkspaceResourceVisibility
  workspaceId: string
}

export type WorkspaceResourceScope = "personal" | "project" | "workspace"

export type WorkspaceResourceVisibility = "private" | "public"

export interface WorkspaceSessionTokenEnvelope {
  expiresAt?: number
  issuedAt: number
  session: SessionRecord
  token: string
  workspaceConnectionId: string
}

export interface WorkspaceSkillDocument {
  content: string
  description: string
  displayPath: string
  id: string
  name: string
  relativePath?: string
  rootPath: string
  sourceKey: string
  sourceOrigin: "skills_dir" | "legacy_commands_dir"
  tree: WorkspaceSkillTreeNode[]
  workspaceOwned: boolean
}

export interface WorkspaceSkillFileDocument {
  byteSize: number
  content?: string | null
  contentType?: string
  displayPath: string
  isText: boolean
  language?: string
  path: string
  readonly: boolean
  skillId: string
  sourceKey: string
}

export interface WorkspaceSkillToolCatalogEntry {
  active: boolean
  assetId?: string
  availability: ViewStatus
  capabilityId?: string
  description: string
  disabled: boolean
  displayPath: string
  executionKind?: CapabilityExecutionKind
  id: string
  kind: "skill"
  management: WorkspaceToolManagementCapabilities
  name: string
  relativePath?: string
  requiredPermission?: "readonly" | "workspace-write" | "danger-full-access" | "null"
  shadowedBy?: string
  sourceKey: string
  sourceKind?: CapabilitySourceKind
  sourceOrigin: "skills_dir" | "legacy_commands_dir" | "builtin_bundle"
  workspaceId: string
  workspaceOwned: boolean
}

export interface WorkspaceSkillTreeDocument {
  displayPath: string
  rootPath: string
  skillId: string
  sourceKey: string
  tree: WorkspaceSkillTreeNode[]
}

export interface WorkspaceSkillTreeNode {
  byteSize?: number
  children?: WorkspaceSkillTreeNode[]
  isText?: boolean
  kind: "directory" | "file"
  name: string
  path: string
}

export interface WorkspaceSummary {
  avatar?: string
  bootstrapStatus: "setup_required" | "ready"
  defaultProjectId: string
  deployment: "local" | "remote"
  host: string
  id: string
  listenAddress: string
  mappedDirectory?: string
  mappedDirectoryDefault?: string
  name: string
  ownerUserId?: string
  projectDefaultPermissions: ProjectDefaultPermissions
  slug: string
}

export type WorkspaceToolCatalogEntry = WorkspaceBuiltinToolCatalogEntry | WorkspaceSkillToolCatalogEntry | WorkspaceMcpToolCatalogEntry

export interface WorkspaceToolConsumerSummary {
  id: string
  kind: string
  name: string
  ownerId?: string
  ownerLabel?: string
  scope: string
}

export interface WorkspaceToolManagementCapabilities {
  canDelete: boolean
  canDisable: boolean
  canEdit: boolean
}

export type WorkspaceToolPermissionMode = "allow" | "deny" | "ask" | "readonly"

export type WorkspaceToolStatus = "active" | "disabled"


export interface OctopusApiPaths {
  "/api/v1/access/audit": {
    get: { operationId: "listAccessAudit"; response: AccessAuditListResponse; error: ApiErrorEnvelope }
  }
  "/api/v1/access/authorization/current": {
    get: { operationId: "getCurrentAuthorization"; response: AuthorizationSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/access/experience": {
    get: { operationId: "getAccessExperience"; response: AccessExperienceResponse; error: ApiErrorEnvelope }
  }
  "/api/v1/access/members": {
    get: { operationId: "listAccessMembers"; response: AccessMemberSummary[]; error: ApiErrorEnvelope }
  }
  "/api/v1/access/menus/definitions": {
    get: { operationId: "listAccessMenuDefinitions"; response: MenuDefinition[]; error: ApiErrorEnvelope }
  }
  "/api/v1/access/menus/features": {
    get: { operationId: "listAccessFeatureDefinitions"; response: FeatureDefinition[]; error: ApiErrorEnvelope }
  }
  "/api/v1/access/menus/gates": {
    get: { operationId: "listAccessMenuGateResults"; response: MenuGateResult[]; error: ApiErrorEnvelope }
  }
  "/api/v1/access/menus/policies": {
    get: { operationId: "listAccessMenuPolicies"; response: MenuPolicyRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createAccessMenuPolicy"; response: MenuPolicyRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/access/menus/policies/{menuId}": {
    put: { operationId: "updateAccessMenuPolicy"; response: MenuPolicyRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteAccessMenuPolicy"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/access/org/assignments": {
    get: { operationId: "listAccessUserOrgAssignments"; response: UserOrgAssignmentRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "upsertAccessUserOrgAssignment"; response: UserOrgAssignmentRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/access/org/assignments/{userId}/{orgUnitId}": {
    delete: { operationId: "deleteAccessUserOrgAssignment"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/access/org/groups": {
    get: { operationId: "listAccessUserGroups"; response: UserGroupRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createAccessUserGroup"; response: UserGroupRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/access/org/groups/{groupId}": {
    put: { operationId: "updateAccessUserGroup"; response: UserGroupRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteAccessUserGroup"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/access/org/positions": {
    get: { operationId: "listAccessPositions"; response: PositionRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createAccessPosition"; response: PositionRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/access/org/positions/{positionId}": {
    put: { operationId: "updateAccessPosition"; response: PositionRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteAccessPosition"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/access/org/units": {
    get: { operationId: "listAccessOrgUnits"; response: OrgUnitRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createAccessOrgUnit"; response: OrgUnitRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/access/org/units/{orgUnitId}": {
    put: { operationId: "updateAccessOrgUnit"; response: OrgUnitRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteAccessOrgUnit"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/access/policies/data-policies": {
    get: { operationId: "listAccessDataPolicies"; response: DataPolicyRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createAccessDataPolicy"; response: DataPolicyRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/access/policies/data-policies/{policyId}": {
    put: { operationId: "updateAccessDataPolicy"; response: DataPolicyRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteAccessDataPolicy"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/access/policies/permission-definitions": {
    get: { operationId: "listAccessPermissionDefinitions"; response: PermissionDefinition[]; error: ApiErrorEnvelope }
  }
  "/api/v1/access/policies/resource-policies": {
    get: { operationId: "listAccessResourcePolicies"; response: ResourcePolicyRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createAccessResourcePolicy"; response: ResourcePolicyRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/access/policies/resource-policies/{policyId}": {
    put: { operationId: "updateAccessResourcePolicy"; response: ResourcePolicyRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteAccessResourcePolicy"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/access/policies/role-bindings": {
    get: { operationId: "listAccessRoleBindings"; response: RoleBindingRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createAccessRoleBinding"; response: RoleBindingRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/access/policies/role-bindings/{bindingId}": {
    put: { operationId: "updateAccessRoleBinding"; response: RoleBindingRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteAccessRoleBinding"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/access/protected-resources": {
    get: { operationId: "listAccessProtectedResources"; response: ProtectedResourceDescriptor[]; error: ApiErrorEnvelope }
  }
  "/api/v1/access/protected-resources/{resourceType}/{resourceId}": {
    put: { operationId: "upsertAccessProtectedResource"; response: ProtectedResourceDescriptor; error: ApiErrorEnvelope }
  }
  "/api/v1/access/roles": {
    get: { operationId: "listAccessRoles"; response: AccessRoleRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createAccessRole"; response: AccessRoleRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/access/roles/{roleId}": {
    put: { operationId: "updateAccessRole"; response: AccessRoleRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteAccessRole"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/access/sessions": {
    get: { operationId: "listAccessSessions"; response: AccessSessionRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/access/sessions/{sessionId}/revoke": {
    post: { operationId: "revokeAccessSession"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/access/sessions/current/revoke": {
    post: { operationId: "revokeCurrentAccessSession"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/access/users": {
    get: { operationId: "listAccessUsers"; response: AccessUserRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createAccessUser"; response: AccessUserRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/access/users/{userId}": {
    put: { operationId: "updateAccessUser"; response: AccessUserRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteAccessUser"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/access/users/{userId}/preset": {
    put: { operationId: "updateAccessUserPreset"; response: AccessMemberSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/access/users/{userId}/sessions/revoke": {
    post: { operationId: "revokeAccessUserSessions"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/apps": {
    get: { operationId: "listClientApps"; response: ClientAppRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "registerClientApp"; response: ClientAppRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/deliverables/{deliverableId}": {
    get: { operationId: "getDeliverableDetail"; response: DeliverableDetail; error: ApiErrorEnvelope }
  }
  "/api/v1/deliverables/{deliverableId}/fork": {
    post: { operationId: "forkDeliverableToConversation"; response: ConversationRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/deliverables/{deliverableId}/promote": {
    post: { operationId: "promoteDeliverable"; response: KnowledgeEntryRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/deliverables/{deliverableId}/versions": {
    get: { operationId: "listDeliverableVersions"; response: DeliverableVersionSummary[]; error: ApiErrorEnvelope }
    post: { operationId: "createDeliverableVersion"; response: DeliverableDetail; error: ApiErrorEnvelope }
  }
  "/api/v1/deliverables/{deliverableId}/versions/{version}": {
    get: { operationId: "getDeliverableVersionContent"; response: DeliverableVersionContent; error: ApiErrorEnvelope }
  }
  "/api/v1/host/bootstrap": {
    get: { operationId: "getHostBootstrap"; response: ShellBootstrap; error: ApiErrorEnvelope }
  }
  "/api/v1/host/health": {
    get: { operationId: "getHostHealthcheck"; response: HealthcheckStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/host/notifications": {
    get: { operationId: "listHostNotifications"; response: NotificationListResponse; error: ApiErrorEnvelope }
    post: { operationId: "createHostNotification"; response: NotificationRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/host/notifications/{notificationId}/dismiss-toast": {
    post: { operationId: "dismissHostNotificationToast"; response: NotificationRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/host/notifications/{notificationId}/read": {
    post: { operationId: "markHostNotificationRead"; response: NotificationRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/host/notifications/read-all": {
    post: { operationId: "markAllHostNotificationsRead"; response: NotificationUnreadSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/host/notifications/unread-summary": {
    get: { operationId: "getHostNotificationUnreadSummary"; response: NotificationUnreadSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/host/preferences": {
    get: { operationId: "getHostPreferences"; response: ShellPreferences; error: ApiErrorEnvelope }
    put: { operationId: "saveHostPreferences"; response: ShellPreferences; error: ApiErrorEnvelope }
  }
  "/api/v1/host/update-check": {
    post: { operationId: "checkHostUpdate"; response: HostUpdateStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/host/update-download": {
    post: { operationId: "downloadHostUpdate"; response: HostUpdateStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/host/update-install": {
    post: { operationId: "installHostUpdate"; response: HostUpdateStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/host/update-status": {
    get: { operationId: "getHostUpdateStatus"; response: HostUpdateStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/host/workspace-connections": {
    get: { operationId: "listHostWorkspaceConnections"; response: WorkspaceConnectionRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createHostWorkspaceConnection"; response: WorkspaceConnectionRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/host/workspace-connections/{connectionId}": {
    delete: { operationId: "deleteHostWorkspaceConnection"; response: null; error: ApiErrorEnvelope }
  }
  "/api/v1/inbox": {
    get: { operationId: "listInboxItems"; response: InboxItemRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/knowledge": {
    get: { operationId: "listKnowledgeEntries"; response: KnowledgeEntryRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/projects": {
    get: { operationId: "listProjects"; response: ProjectRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createProject"; response: ProjectRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}": {
    patch: { operationId: "updateProject"; response: ProjectRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteProject"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/agent-links": {
    get: { operationId: "listProjectAgentLinks"; response: ProjectAgentLinkRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createProjectAgentLink"; response: ProjectAgentLinkRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/agent-links/{agentId}": {
    delete: { operationId: "deleteProjectAgentLink"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/agents/{agentId}/copy-to-project": {
    post: { operationId: "copyProjectAgentToProject"; response: ImportWorkspaceAgentBundleResult; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/agents/export": {
    post: { operationId: "exportProjectAgentBundle"; response: ExportWorkspaceAgentBundleResult; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/agents/import": {
    post: { operationId: "importProjectAgentBundle"; response: ImportWorkspaceAgentBundleResult; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/agents/import-preview": {
    post: { operationId: "previewProjectAgentBundleImport"; response: ImportWorkspaceAgentBundlePreview; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/dashboard": {
    get: { operationId: "getProjectDashboard"; response: ProjectDashboardSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/deletion-requests": {
    get: { operationId: "listProjectDeletionRequests"; response: ProjectDeletionRequest[]; error: ApiErrorEnvelope }
    post: { operationId: "createProjectDeletionRequest"; response: ProjectDeletionRequest; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/deletion-requests/{requestId}/approve": {
    post: { operationId: "approveProjectDeletionRequest"; response: ProjectDeletionRequest; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/deletion-requests/{requestId}/reject": {
    post: { operationId: "rejectProjectDeletionRequest"; response: ProjectDeletionRequest; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/deliverables": {
    get: { operationId: "listProjectDeliverables"; response: DeliverableSummary[]; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/knowledge": {
    get: { operationId: "listProjectKnowledge"; response: KnowledgeRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/pet": {
    get: { operationId: "getCurrentUserProjectPetSnapshot"; response: PetWorkspaceSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/pet/conversation": {
    put: { operationId: "bindProjectPetConversation"; response: PetConversationBinding; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/pet/presence": {
    patch: { operationId: "saveProjectPetPresence"; response: PetPresenceState; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/promotion-requests": {
    get: { operationId: "listProjectPromotionRequests"; response: ProjectPromotionRequest[]; error: ApiErrorEnvelope }
    post: { operationId: "createProjectPromotionRequest"; response: ProjectPromotionRequest; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/resources": {
    get: { operationId: "listProjectResources"; response: WorkspaceResourceRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createProjectResource"; response: WorkspaceResourceRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/resources/{resourceId}": {
    patch: { operationId: "updateProjectResource"; response: WorkspaceResourceRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteProjectResource"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/resources/folder": {
    post: { operationId: "createProjectResourceFolder"; response: WorkspaceResourceRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/resources/import": {
    post: { operationId: "importProjectResource"; response: WorkspaceResourceRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/runtime-config": {
    get: { operationId: "getProjectRuntimeConfig"; response: RuntimeEffectiveConfig; error: ApiErrorEnvelope }
    patch: { operationId: "saveProjectRuntimeConfig"; response: RuntimeEffectiveConfig; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/runtime-config/validate": {
    post: { operationId: "validateProjectRuntimeConfig"; response: RuntimeConfigValidationResult; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/tasks": {
    get: { operationId: "listProjectTasks"; response: TaskSummary[]; error: ApiErrorEnvelope }
    post: { operationId: "createProjectTask"; response: TaskDetail; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/tasks/{taskId}": {
    get: { operationId: "getProjectTaskDetail"; response: TaskDetail; error: ApiErrorEnvelope }
    patch: { operationId: "updateProjectTask"; response: TaskDetail; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/tasks/{taskId}/interventions": {
    post: { operationId: "createProjectTaskIntervention"; response: TaskInterventionRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/tasks/{taskId}/launch": {
    post: { operationId: "launchProjectTask"; response: TaskRunSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/tasks/{taskId}/rerun": {
    post: { operationId: "rerunProjectTask"; response: TaskRunSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/tasks/{taskId}/runs": {
    get: { operationId: "listProjectTaskRuns"; response: TaskRunSummary[]; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/team-links": {
    get: { operationId: "listProjectTeamLinks"; response: ProjectTeamLinkRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createProjectTeamLink"; response: ProjectTeamLinkRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/team-links/{teamId}": {
    delete: { operationId: "deleteProjectTeamLink"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/teams/{teamId}/copy-to-project": {
    post: { operationId: "copyProjectTeamToProject"; response: ImportWorkspaceAgentBundleResult; error: ApiErrorEnvelope }
  }
  "/api/v1/resources/{resourceId}": {
    get: { operationId: "getWorkspaceResourceDetail"; response: WorkspaceResourceRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/resources/{resourceId}/children": {
    get: { operationId: "listWorkspaceResourceChildren"; response: WorkspaceResourceChildrenRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/resources/{resourceId}/content": {
    get: { operationId: "getWorkspaceResourceContent"; response: WorkspaceResourceContentDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/resources/{resourceId}/promote": {
    post: { operationId: "promoteWorkspaceResource"; response: WorkspaceResourceRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/bootstrap": {
    get: { operationId: "getRuntimeBootstrap"; response: RuntimeBootstrap; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/config": {
    get: { operationId: "getRuntimeConfig"; response: RuntimeEffectiveConfig; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/config/configured-models/probe": {
    post: { operationId: "probeRuntimeConfiguredModel"; response: RuntimeConfiguredModelProbeResult; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/config/scopes/{scope}": {
    patch: { operationId: "saveRuntimeConfigScope"; response: RuntimeEffectiveConfig; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/config/validate": {
    post: { operationId: "validateRuntimeConfig"; response: RuntimeConfigValidationResult; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/generations": {
    post: { operationId: "runRuntimeGeneration"; response: RuntimeGenerationResult; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions": {
    get: { operationId: "listRuntimeSessions"; response: RuntimeSessionSummary[]; error: ApiErrorEnvelope }
    post: { operationId: "createRuntimeSession"; response: RuntimeSessionDetail; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions/{sessionId}": {
    get: { operationId: "getRuntimeSession"; response: RuntimeSessionDetail; error: ApiErrorEnvelope }
    delete: { operationId: "deleteRuntimeSession"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions/{sessionId}/approvals/{approvalId}": {
    post: { operationId: "resolveRuntimeApproval"; response: RuntimeRunSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions/{sessionId}/auth-challenges/{challengeId}": {
    post: { operationId: "resolveRuntimeAuthChallenge"; response: RuntimeRunSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions/{sessionId}/configured-model": {
    post: { operationId: "rebindRuntimeSessionConfiguredModel"; response: RuntimeSessionDetail; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions/{sessionId}/events": {
    get: { operationId: "listRuntimeSessionEvents"; response: RuntimeEventEnvelope[]; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions/{sessionId}/memory-proposals/{proposalId}": {
    post: { operationId: "resolveRuntimeMemoryProposal"; response: RuntimeRunSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions/{sessionId}/subruns/{subrunId}/cancel": {
    post: { operationId: "cancelRuntimeSubrun"; response: RuntimeRunSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions/{sessionId}/turns": {
    post: { operationId: "submitRuntimeTurn"; response: RuntimeRunSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/system/auth/bootstrap-admin": {
    post: { operationId: "registerBootstrapAdmin"; response: EnterpriseAuthSuccess; error: ApiErrorEnvelope }
  }
  "/api/v1/system/auth/login": {
    post: { operationId: "systemAuthLogin"; response: EnterpriseAuthSuccess; error: ApiErrorEnvelope }
  }
  "/api/v1/system/auth/session": {
    get: { operationId: "getSystemAuthSession"; response: EnterpriseSessionSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/system/auth/status": {
    get: { operationId: "getSystemAuthStatus"; response: SystemAuthStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/system/bootstrap": {
    get: { operationId: "getSystemBootstrap"; response: SystemBootstrapStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/system/health": {
    get: { operationId: "getSystemHealthcheck"; response: HealthcheckStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace": {
    get: { operationId: "getWorkspaceSummary"; response: WorkspaceSummary; error: ApiErrorEnvelope }
    patch: { operationId: "updateWorkspace"; response: WorkspaceSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/agents": {
    get: { operationId: "listWorkspaceAgents"; response: AgentRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createWorkspaceAgent"; response: AgentRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/agents/{agentId}": {
    patch: { operationId: "updateWorkspaceAgent"; response: AgentRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceAgent"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/agents/{agentId}/copy-to-workspace": {
    post: { operationId: "copyWorkspaceAgentToWorkspace"; response: ImportWorkspaceAgentBundleResult; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/agents/export": {
    post: { operationId: "exportWorkspaceAgentBundle"; response: ExportWorkspaceAgentBundleResult; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/agents/import": {
    post: { operationId: "importWorkspaceAgentBundle"; response: ImportWorkspaceAgentBundleResult; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/agents/import-preview": {
    post: { operationId: "previewWorkspaceAgentBundleImport"; response: ImportWorkspaceAgentBundlePreview; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/management-projection": {
    get: { operationId: "getWorkspaceCapabilityManagementProjection"; response: CapabilityManagementProjection; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/management-projection/disable": {
    patch: { operationId: "setWorkspaceCapabilityAssetDisabled"; response: CapabilityManagementProjection; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/mcp-servers": {
    post: { operationId: "createWorkspaceMcpServer"; response: WorkspaceMcpServerDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/mcp-servers/{serverName}": {
    get: { operationId: "getWorkspaceMcpServer"; response: WorkspaceMcpServerDocument; error: ApiErrorEnvelope }
    patch: { operationId: "updateWorkspaceMcpServer"; response: WorkspaceMcpServerDocument; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceMcpServer"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/mcp-servers/{serverName}/copy-to-managed": {
    post: { operationId: "copyWorkspaceMcpServerToManaged"; response: WorkspaceMcpServerDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/models": {
    get: { operationId: "getWorkspaceCatalogModels"; response: ModelCatalogSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/provider-credentials": {
    get: { operationId: "listWorkspaceProviderCredentials"; response: CredentialBinding[]; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills": {
    post: { operationId: "createWorkspaceSkill"; response: WorkspaceSkillDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills/{skillId}": {
    get: { operationId: "getWorkspaceSkill"; response: WorkspaceSkillDocument; error: ApiErrorEnvelope }
    patch: { operationId: "updateWorkspaceSkill"; response: WorkspaceSkillDocument; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceSkill"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills/{skillId}/copy-to-managed": {
    post: { operationId: "copyWorkspaceSkillToManaged"; response: WorkspaceSkillDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills/{skillId}/files/{relativePath}": {
    get: { operationId: "getWorkspaceSkillFile"; response: WorkspaceSkillFileDocument; error: ApiErrorEnvelope }
    patch: { operationId: "updateWorkspaceSkillFile"; response: WorkspaceSkillFileDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills/{skillId}/tree": {
    get: { operationId: "getWorkspaceSkillTree"; response: WorkspaceSkillTreeDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills/import-archive": {
    post: { operationId: "importWorkspaceSkillArchive"; response: WorkspaceSkillDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills/import-folder": {
    post: { operationId: "importWorkspaceSkillFolder"; response: WorkspaceSkillDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/tools": {
    get: { operationId: "listWorkspaceTools"; response: ToolRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createWorkspaceTool"; response: ToolRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/tools/{toolId}": {
    patch: { operationId: "updateWorkspaceTool"; response: ToolRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceTool"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/deliverables": {
    get: { operationId: "listWorkspaceDeliverables"; response: DeliverableSummary[]; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/filesystem/directories": {
    get: { operationId: "listWorkspaceFilesystemDirectories"; response: WorkspaceDirectoryBrowserResponse; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/knowledge": {
    get: { operationId: "listWorkspaceKnowledge"; response: KnowledgeRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/overview": {
    get: { operationId: "getWorkspaceOverview"; response: WorkspaceOverviewSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/personal-center/profile": {
    get: { operationId: "getCurrentUserProfile"; response: UserRecordSummary; error: ApiErrorEnvelope }
    patch: { operationId: "updateCurrentUserProfile"; response: UserRecordSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/personal-center/profile/password": {
    post: { operationId: "changeCurrentUserPassword"; response: ChangeCurrentUserPasswordResponse; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/personal-center/profile/runtime-config": {
    get: { operationId: "getUserRuntimeConfig"; response: RuntimeEffectiveConfig; error: ApiErrorEnvelope }
    patch: { operationId: "saveUserRuntimeConfig"; response: RuntimeEffectiveConfig; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/personal-center/profile/runtime-config/validate": {
    post: { operationId: "validateUserRuntimeConfig"; response: RuntimeConfigValidationResult; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/pet": {
    get: { operationId: "getCurrentUserPetHomeSnapshot"; response: PetWorkspaceSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/pet/conversation": {
    put: { operationId: "bindWorkspacePetConversation"; response: PetConversationBinding; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/pet/dashboard": {
    get: { operationId: "getCurrentUserPetDashboardSummary"; response: PetDashboardSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/pet/presence": {
    patch: { operationId: "saveWorkspacePetPresence"; response: PetPresenceState; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/promotion-requests": {
    get: { operationId: "listWorkspacePromotionRequests"; response: ProjectPromotionRequest[]; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/promotion-requests/{requestId}/review": {
    post: { operationId: "reviewProjectPromotionRequest"; response: ProjectPromotionRequest; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/resources": {
    get: { operationId: "listWorkspaceResources"; response: WorkspaceResourceRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createWorkspaceResource"; response: WorkspaceResourceRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/resources/{resourceId}": {
    patch: { operationId: "updateWorkspaceResource"; response: WorkspaceResourceRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceResource"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/resources/import": {
    post: { operationId: "importWorkspaceResource"; response: WorkspaceResourceRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/teams": {
    get: { operationId: "listWorkspaceTeams"; response: TeamRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createWorkspaceTeam"; response: TeamRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/teams/{teamId}": {
    patch: { operationId: "updateWorkspaceTeam"; response: TeamRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceTeam"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/teams/{teamId}/copy-to-workspace": {
    post: { operationId: "copyWorkspaceTeamToWorkspace"; response: ImportWorkspaceAgentBundleResult; error: ApiErrorEnvelope }
  }
}

