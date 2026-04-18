import type {
  AccessExperienceResponse,
  AccessMemberSummary,
  AccessUserPresetUpdateRequest,
  ConversationRecord,
  CreateDeliverableVersionInput,
  AuditRecord,
  BindPetConversationInput,
  CapabilityAssetDisablePatch,
  CapabilityManagementProjection,
  ChangeCurrentUserPasswordRequest,
  ChangeCurrentUserPasswordResponse,
  DeliverableDetail,
  DeliverableVersionContent,
  DeliverableVersionSummary,
  KnowledgeEntryRecord,
  CopyWorkspaceSkillToManagedInput,
  CreateProjectPromotionRequestInput,
  CreateTaskInterventionRequest,
  CreateTaskRequest,
  CreateWorkspaceResourceFolderInput,
  CreateWorkspaceResourceInput,
  CreateWorkspaceSkillInput,
  ExportWorkspaceAgentBundleInput,
  ExportWorkspaceAgentBundleResult,
  ImportWorkspaceAgentBundleInput,
  ImportWorkspaceAgentBundlePreview,
  ImportWorkspaceAgentBundlePreviewInput,
  ImportWorkspaceAgentBundleResult,
  KnowledgeRecord,
  LaunchTaskRequest,
  PetConversationBinding,
  ProjectAgentLinkRecord,
  ProjectPromotionRequest,
  ProjectResourceKind,
  ProjectTeamLinkRecord,
  PromoteDeliverableInput,
  PromoteWorkspaceResourceInput,
  ResourcePreviewKind,
  RerunTaskRequest,
  RuntimeApprovalRequest,
  RuntimeBootstrap,
  RuntimeConfigPatch,
  RuntimeConfigValidationResult,
  RuntimeConfiguredModelCredentialRecord,
  RuntimeConfiguredModelCredentialUpsertInput,
  RuntimeEffectiveConfig,
  RuntimeRunSnapshot,
  RuntimeSessionSummary,
  SavePetPresenceInput,
  TaskDetail,
  TaskInterventionRecord,
  TaskRunSummary,
  TaskSummary,
  UpsertAgentInput,
  UpsertTeamInput,
  UpdateCurrentUserProfileRequest,
  UpdateTaskRequest,
  UpdateProjectRequest,
  UpdateWorkspaceResourceInput,
  UpdateWorkspaceSkillFileInput,
  UpdateWorkspaceSkillInput,
  UserRecordSummary,
  WorkspaceConnectionRecord,
  WorkspaceDirectoryBrowserResponse,
  WorkspaceMcpServerDocument,
  WorkspaceResourceChildrenRecord,
  WorkspaceResourceContentDocument,
  WorkspaceResourceImportInput,
  WorkspaceResourceRecord,
  WorkspaceResourceScope,
  WorkspaceResourceVisibility,
  WorkspaceSessionTokenEnvelope,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  ProtectedResourceDescriptor,
  WorkspaceToolCatalogEntry,
  WorkspaceDirectoryUploadEntry,
  ProtectedResourceMetadataUpsertRequest,
  ReviewProjectPromotionRequestInput,
} from '@octopus/schema'
import { resolveRuntimePermissionMode } from '@octopus/schema'

import type { WorkspaceClient } from '@/tauri/workspace-client'
import { WorkspaceApiError } from '@/tauri/shared'
import { deriveCapabilityManagementProjection } from '@/stores/catalog_management'

import { clone, createWorkspaceSessionEnvelope } from './workspace-fixture-bootstrap'
import type { FixtureOptions, WorkspaceFixtureState } from './workspace-fixture-state'
import {
  cloneSkillFiles,
  createImportedSkillFiles,
  createMcpCatalogEntry,
  createSkillCatalogEntry,
  createSkillDocument,
  createSkillFileDocument,
  createSkillTemplate,
  normalizeSkillFrontmatterName,
  skillDescriptionFromContent,
  skillNameFromContent,
  skillSlugFromRelativePath,
} from './workspace-fixture-skill-helpers'
import {
  createProjectRecord,
  normalizeAgentRecord,
  normalizeTeamRecord,
  syncWorkspaceProjectState,
  updateDefaultProjectIfNeeded,
  updateProjectRecord,
} from './workspace-fixture-projects'
import {
  applyJsonMergePatch,
  createApproval,
  createEvent,
  createPetPresenceState,
  createPetSnapshot,
  createRuntimeConfigSource,
  createRuntimeMessage,
  createSessionDetail,
  createTraceItem,
  eventsAfter,
} from './workspace-fixture-runtime'
import type { RuntimeSessionState } from './workspace-fixture-runtime'

export function createWorkspaceClientFixture(
  connection: WorkspaceConnectionRecord,
  workspaceState: WorkspaceFixtureState,
  options: FixtureOptions = {},
  session?: WorkspaceSessionTokenEnvelope,
): WorkspaceClient {
  const protectedResourceKey = (resourceType: string, resourceId: string) => `${resourceType}:${resourceId}`

  const ensureRuntimeState = (sessionId: string): RuntimeSessionState => {
    const state = workspaceState.runtimeSessions.get(sessionId)
    if (!state) {
      throw new Error(`Unknown runtime session ${sessionId}`)
    }
    return state
  }

  const defaultSession = clone(
    session
    ?? createWorkspaceSessionEnvelope(
      connection,
      workspaceState.workspace.ownerUserId ?? workspaceState.users[0]?.id ?? 'user-owner',
    ),
  )
  const fixtureSessions = workspaceState.users.map((user, index) => {
    const nextSession = clone(
      session?.session.userId === user.id
        ? session
        : createWorkspaceSessionEnvelope(connection, user.id),
    )

    return {
      sessionId: nextSession.session.id,
      token: nextSession.token,
      userId: user.id,
      clientAppId: nextSession.session.clientAppId || (index === 0 ? 'octopus-desktop' : 'octopus-web'),
      status: 'active' as const,
      createdAt: nextSession.session.createdAt + index,
      expiresAt: nextSession.session.expiresAt,
    }
  })

  let accessUsers = workspaceState.users.map(user => ({
    id: user.id,
    username: user.username,
    displayName: user.displayName,
    status: user.status,
    passwordState: user.passwordState,
  }))

  let accessOrgUnits = clone(workspaceState.orgUnits)
  let accessPositions = clone(workspaceState.positions)
  let accessUserGroups = clone(workspaceState.userGroups)
  let accessUserOrgAssignments = clone(workspaceState.userOrgAssignments)
  let accessRoles = clone(workspaceState.roles)
  let accessRoleBindings = clone(workspaceState.roleBindings)
  let accessDataPolicies = clone(workspaceState.dataPolicies).map(policy => ({
    ...policy,
    classifications: clone(policy.classifications ?? []),
  }))

  let accessResourcePolicies = clone(workspaceState.resourcePolicies)

  let accessMenuPolicies = clone(workspaceState.menuPolicies)

  function ensureRuntimeProjectConfig(projectId: string): RuntimeEffectiveConfig {
    const existing = workspaceState.runtimeProjectConfigs[projectId]
    if (existing) {
      return existing
    }

    const project = workspaceState.projects.find(item => item.id === projectId)
    const ownerId = project?.ownerUserId || 'user-owner'
    const config: RuntimeEffectiveConfig = {
      effectiveConfig: {
        provider: {
          defaultModel: 'claude-sonnet-4-5',
        },
        ...clone(workspaceState.runtimeWorkspaceConfig.effectiveConfig),
        approvals: {
          defaultMode: 'manual',
        },
      },
      effectiveConfigHash: `${workspaceState.workspace.id}-${projectId}-project-cfg-hash-${Date.now()}`,
      sources: [
        createRuntimeConfigSource('user', workspaceState.workspace.id, ownerId),
        createRuntimeConfigSource('workspace', workspaceState.workspace.id),
        createRuntimeConfigSource('project', workspaceState.workspace.id, projectId),
      ],
      validation: {
        valid: true,
        errors: [],
        warnings: [],
      },
      secretReferences: [],
    }

    workspaceState.runtimeProjectConfigs[projectId] = config
    return config
  }

  const protectedResourceMetadata = new Map(
    workspaceState.protectedResourceMetadata.map(record => [
      protectedResourceKey(record.resourceType, record.id),
      clone(record),
    ] as const),
  )
  const managedConfiguredModelSecrets = new Map<string, string>()
  const auditRecords: AuditRecord[] = [
    {
      id: `audit-${connection.workspaceId}-bootstrap`,
      workspaceId: connection.workspaceId,
      actorType: 'user',
      actorId: 'user-owner',
      action: 'system.auth.login.success',
      resource: 'system.auth',
      outcome: 'success',
      createdAt: Date.now(),
    },
  ]

  const findFixtureSession = (userId: string) =>
    fixtureSessions.find(record => record.userId === userId)
    ?? fixtureSessions.find(record => record.token === activeSessionToken)
    ?? fixtureSessions[0]

  const buildSession = (
    userId: string,
    token = findFixtureSession(userId)?.token ?? defaultSession.token,
  ): WorkspaceSessionTokenEnvelope['session'] => {
    const baseSession = findFixtureSession(userId)

    return {
      ...clone(defaultSession.session),
      id: baseSession?.sessionId ?? defaultSession.session.id,
      userId,
      clientAppId: baseSession?.clientAppId ?? defaultSession.session.clientAppId,
      token,
      status: baseSession?.status ?? defaultSession.session.status,
      createdAt: baseSession?.createdAt ?? defaultSession.session.createdAt,
      expiresAt: baseSession?.expiresAt ?? defaultSession.session.expiresAt,
    }
  }

  const buildEnterpriseSession = (userId: string, token = defaultSession.token) => {
    const session = buildSession(userId, token)
    const user = workspaceState.users.find(record => record.id === userId)
      ?? workspaceState.users.find(record => record.id === workspaceState.workspace.ownerUserId)
      ?? workspaceState.users[0]

    return {
      sessionId: session.id,
      token: session.token,
      workspaceId: session.workspaceId,
      clientAppId: session.clientAppId,
      status: session.status,
      createdAt: session.createdAt,
      expiresAt: session.expiresAt,
      principal: {
        userId: user?.id ?? userId,
        username: user?.username ?? 'owner',
        displayName: user?.displayName ?? 'Workspace Owner',
        status: user?.status ?? 'active',
      },
    }
  }

  const deliverableVersionSummaries = workspaceState.deliverableVersionSummaries
  const deliverableVersionContents = workspaceState.deliverableVersionContents

  const resolveDeliverablePreviewKind = (contentType?: string, fallback?: ResourcePreviewKind): ResourcePreviewKind => {
    if (fallback) {
      return fallback
    }
    if (contentType?.includes('markdown')) {
      return 'markdown'
    }
    if (contentType?.includes('json')) {
      return 'code'
    }
    return 'text'
  }

  const resolveDeliverableSessionState = (conversationId: string) =>
    [...workspaceState.runtimeSessions.values()].find(
      state => state.detail.summary.conversationId === conversationId,
    )

  const createVersionSummaryFromRecord = (
    artifact: WorkspaceFixtureState['deliverables'][number],
    version: number,
  ): DeliverableVersionSummary => ({
    artifactId: artifact.id,
    version,
    title: version === artifact.latestVersion ? artifact.title : `${artifact.title} v${version}`,
    updatedAt: artifact.updatedAt - (artifact.latestVersion - version),
    previewKind: resolveDeliverablePreviewKind(artifact.contentType, artifact.previewKind),
    contentType: artifact.contentType,
    byteSize: 256 + version,
    contentHash: `${artifact.id}-hash-v${version}`,
    parentVersion: version > 1 ? version - 1 : undefined,
    sessionId: resolveDeliverableSessionState(artifact.conversationId)?.detail.summary.id,
    runId: resolveDeliverableSessionState(artifact.conversationId)?.detail.run.id,
    sourceMessageId: version === artifact.latestVersion ? `msg-${artifact.conversationId}-assistant-latest` : undefined,
  })

  const ensureDeliverableVersionState = (artifactId: string) => {
    if (deliverableVersionSummaries.has(artifactId)) {
      return
    }

    const artifact = workspaceState.deliverables.find(record => record.id === artifactId)
    if (!artifact) {
      return
    }

    const summaries: DeliverableVersionSummary[] = []
    for (let version = 1; version <= artifact.latestVersion; version += 1) {
      const summary = createVersionSummaryFromRecord(artifact, version)
      summaries.push(summary)
      deliverableVersionContents.set(
        `${artifactId}:${version}`,
        {
          artifactId,
          version,
          editable: true,
          fileName: `${artifact.title}.md`,
          previewKind: summary.previewKind,
          contentType: artifact.contentType,
          byteSize: summary.byteSize,
          textContent: summary.previewKind === 'markdown'
            ? `# ${summary.title}\n\nVersion ${version} content for ${artifact.id}.`
            : `${summary.title} (version ${version})`,
        },
      )
    }
    deliverableVersionSummaries.set(
      artifactId,
      summaries.sort((left, right) => right.version - left.version),
    )
  }

  const getDeliverableDetail = (artifactId: string): DeliverableDetail => {
    ensureDeliverableVersionState(artifactId)
    const artifact = workspaceState.deliverables.find(record => record.id === artifactId)
    if (!artifact) {
      throw new WorkspaceApiError({
        message: 'deliverable not found',
        status: 404,
        requestId: 'req-deliverable-not-found',
        retryable: false,
        code: 'NOT_FOUND',
      })
    }

    const sessionState = resolveDeliverableSessionState(artifact.conversationId)
    const promotedKnowledge = workspaceState.projectKnowledge[artifact.projectId]?.find(
      record => record.sourceRef === artifact.id,
    )

    return {
      id: artifact.id,
      workspaceId: artifact.workspaceId,
      projectId: artifact.projectId,
      conversationId: artifact.conversationId,
      sessionId: sessionState?.detail.summary.id ?? `session-${artifact.conversationId}`,
      runId: sessionState?.detail.run.id ?? `run-${artifact.conversationId}`,
      sourceMessageId: `msg-${artifact.conversationId}-assistant-latest`,
      parentArtifactId: undefined,
      title: artifact.title,
      status: artifact.status,
      latestVersion: artifact.latestVersion,
      latestVersionRef: artifact.latestVersionRef,
      previewKind: resolveDeliverablePreviewKind(artifact.contentType, artifact.previewKind),
      contentType: artifact.contentType,
      byteSize: 512 + artifact.latestVersion,
      contentHash: `${artifact.id}-latest-hash`,
      storagePath: `data/artifacts/${artifact.id}`,
      updatedAt: artifact.updatedAt,
      promotionState: artifact.promotionState,
      promotionKnowledgeId: promotedKnowledge?.id,
    }
  }

  const updateDeliverableRecord = (artifactId: string, nextRecord: WorkspaceFixtureState['deliverables'][number]) => {
    workspaceState.deliverables = workspaceState.deliverables.map(record =>
      record.id === artifactId ? nextRecord : record,
    )
  }

  const createKnowledgeEntryRecord = (record: {
    id: string
    workspaceId: string
    projectId?: string
    title: string
    sourceType: string
    sourceRef: string
    status: string
    updatedAt: number
  }): KnowledgeEntryRecord => ({
    id: record.id,
    workspaceId: record.workspaceId,
    projectId: record.projectId,
    scope: record.projectId ? 'project' : 'workspace',
    title: record.title,
    sourceType: record.sourceType,
    sourceRef: record.sourceRef,
    status: record.status,
    updatedAt: record.updatedAt,
  })

  const getCurrentUserId = () => workspaceState.currentUserId || defaultSession.session.userId
  const getCurrentUser = () =>
    accessUsers.find(user => user.id === getCurrentUserId())
    ?? accessUsers.find(user => user.id === workspaceState.workspace.ownerUserId)
    ?? accessUsers[0]
  const resolveSelectedActor = (selectedActorRef?: string) => {
    const [actorKind, actorId] = (selectedActorRef ?? '').split(':', 2)
    if ((actorKind === 'agent' || actorKind === 'team') && actorId) {
      return {
        actorKind,
        actorId,
      } as const
    }

    return {
      actorKind: 'agent' as const,
      actorId: 'agent-architect',
    }
  }
  const resolveActorLabel = (actorKind: 'agent' | 'team', actorId: string) => {
    const actorRecord = actorKind === 'team'
      ? workspaceState.teams.find(team => team.id === actorId)
      : workspaceState.agents.find(agent => agent.id === actorId)
    return actorRecord
      ? `${actorRecord.name} · ${actorKind === 'team' ? 'Team' : 'Agent'}`
      : '默认智能体'
  }

  const getFeatureCode = (menuId: string, routeName?: string) => `feature:${routeName ?? menuId}`
  const ROOT_ORG_UNIT_ID = 'org-root'
  const CUSTOM_ACCESS_CODE = 'custom'
  const CUSTOM_ACCESS_NAME = 'Custom access'
  const MIXED_ACCESS_CODE = 'mixed'
  const MIXED_ACCESS_NAME = 'Mixed access'
  const NO_PRESET_ASSIGNED_NAME = 'No preset assigned'
  const preciseToolResourceType = (kind: string) => {
    switch (kind) {
      case 'builtin':
        return 'tool.builtin'
      case 'mcp':
        return 'tool.mcp'
      default:
        return 'tool.skill'
    }
  }

  const appendAudit = (
    action: string,
    outcome: string,
    resource: string,
    projectId?: string,
  ) => {
    auditRecords.unshift({
      id: `audit-${Date.now()}-${Math.random().toString(16).slice(2, 8)}`,
      workspaceId: connection.workspaceId,
      actorType: 'user',
      actorId: getCurrentUserId(),
      action,
      resource,
      outcome,
      projectId,
      createdAt: Date.now(),
    })
  }

  const workspaceResourceBaseDirectory = () =>
    workspaceState.workspace.deployment === 'local'
      ? 'data/resources/workspace'
      : '/remote/workspace/resources'

  const decodeBase64Text = (value: string) => {
    if (typeof globalThis.atob === 'function') {
      return globalThis.atob(value)
    }
    return Buffer.from(value, 'base64').toString('utf-8')
  }

  const inferPreviewKind = (
    kind: ProjectResourceKind,
    contentType?: string,
    nameOrPath?: string,
  ): ResourcePreviewKind => {
    if (kind === 'folder') {
      return 'folder'
    }
    if (kind === 'url') {
      return 'url'
    }

    const lowerContentType = contentType?.toLowerCase() ?? ''
    const lowerName = nameOrPath?.toLowerCase() ?? ''

    if (lowerContentType.includes('markdown') || lowerName.endsWith('.md')) {
      return 'markdown'
    }
    if (
      lowerContentType.includes('json')
      || lowerContentType.includes('javascript')
      || lowerContentType.includes('typescript')
      || lowerName.endsWith('.json')
      || lowerName.endsWith('.js')
      || lowerName.endsWith('.ts')
      || lowerName.endsWith('.vue')
      || lowerName.endsWith('.rs')
      || lowerName.endsWith('.yaml')
      || lowerName.endsWith('.yml')
    ) {
      return 'code'
    }
    if (lowerContentType.startsWith('image/') || /\.(png|jpe?g|webp|gif|svg)$/.test(lowerName)) {
      return 'image'
    }
    if (lowerContentType === 'application/pdf' || lowerName.endsWith('.pdf')) {
      return 'pdf'
    }
    if (lowerContentType.startsWith('audio/')) {
      return 'audio'
    }
    if (lowerContentType.startsWith('video/')) {
      return 'video'
    }
    if (lowerContentType.startsWith('text/') || lowerContentType === 'application/xml' || lowerName.endsWith('.csv')) {
      return 'text'
    }
    return 'binary'
  }

  const normalizeImportedFiles = (input: WorkspaceResourceImportInput) => {
    let rootDirName = input.rootDirName?.trim() ?? ''
    let files = input.files.map(file => ({ ...file, relativePath: file.relativePath.replace(/\\/g, '/') }))

    if (!rootDirName) {
      const topLevelNames = Array.from(new Set(
        files
          .map(file => file.relativePath.split('/')[0])
          .filter((value): value is string => Boolean(value)),
      ))
      if (topLevelNames.length === 1 && files.some(file => file.relativePath.includes('/'))) {
        rootDirName = topLevelNames[0] ?? ''
      }
    }

    if (rootDirName) {
      const rootPrefix = `${rootDirName}/`
      files = files.map((file) => ({
        ...file,
        relativePath: file.relativePath.startsWith(rootPrefix)
          ? file.relativePath.slice(rootPrefix.length)
          : file.relativePath,
      }))
    }

    return {
      rootDirName,
      files,
    }
  }

  const findProjectRecord = (projectId: string) => {
    const project = workspaceState.projects.find(record => record.id === projectId)
    if (!project) {
      throw new WorkspaceApiError({
        message: 'project not found',
        status: 404,
        requestId: 'req-project-not-found',
        retryable: false,
        code: 'NOT_FOUND',
      })
    }
    return project
  }

  const ensureProjectOwner = (projectId: string) => {
    const project = findProjectRecord(projectId)
    if (project.ownerUserId !== getCurrentUserId()) {
      throw new WorkspaceApiError({
        message: 'project owner required',
        status: 403,
        requestId: 'req-project-owner-required',
        retryable: false,
        code: 'FORBIDDEN',
      })
    }
    return project
  }

  const taskDetailByKey = workspaceState.taskDetailsByKey
  const taskRunsByKey = workspaceState.taskRunsByKey
  const taskInterventionsByKey = workspaceState.taskInterventionsByKey

  const taskKey = (projectId: string, taskId: string) => `${projectId}:${taskId}`
  const taskInputError = (message: string, requestId: string) =>
    new WorkspaceApiError({
      message,
      status: 400,
      requestId,
      retryable: false,
      code: 'INVALID_INPUT',
    })
  const normalizeRequiredTaskText = (
    value: string | null | undefined,
    fieldLabel: string,
    requestId: string,
  ) => {
    const trimmed = value?.trim() ?? ''
    if (!trimmed) {
      throw taskInputError(`${fieldLabel} is required`, requestId)
    }
    return trimmed
  }
  const normalizeOptionalTaskText = (value: string | null | undefined) => {
    const trimmed = value?.trim() ?? ''
    return trimmed.length > 0 ? trimmed : null
  }
  const normalizeTaskContextBundle = (
    bundle?: TaskDetail['contextBundle'] | CreateTaskRequest['contextBundle'],
  ): TaskDetail['contextBundle'] => ({
    refs: (bundle?.refs ?? [])
      .map(reference => ({
        kind: reference.kind.trim(),
        refId: reference.refId.trim(),
        title: reference.title.trim(),
        subtitle: reference.subtitle?.trim() || undefined,
        versionRef: normalizeOptionalTaskText(reference.versionRef),
        pinMode: reference.pinMode?.trim() || 'snapshot',
      }))
      .filter(reference => reference.kind && reference.refId && reference.title),
    pinnedInstructions: bundle?.pinnedInstructions?.trim() ?? '',
    resolutionMode: normalizeOptionalTaskText(bundle?.resolutionMode) ?? 'explicit_only',
    lastResolvedAt: bundle?.lastResolvedAt ?? null,
  })
  const emptyTaskAnalytics = (): TaskDetail['analyticsSummary'] => ({
    runCount: 0,
    manualRunCount: 0,
    scheduledRunCount: 0,
    completionCount: 0,
    failureCount: 0,
    takeoverCount: 0,
    approvalRequiredCount: 0,
    averageRunDurationMs: 0,
    lastSuccessfulRunAt: null,
  })
  const updateTaskAnalyticsForRun = (
    analytics: TaskDetail['analyticsSummary'],
    triggerType: TaskRunSummary['triggerType'],
  ): TaskDetail['analyticsSummary'] => {
    const updated = {
      ...clone(analytics),
      runCount: analytics.runCount + 1,
    }
    if (triggerType === 'manual') {
      updated.manualRunCount += 1
    } else if (triggerType === 'scheduled') {
      updated.scheduledRunCount += 1
    } else if (triggerType === 'takeover') {
      updated.takeoverCount += 1
    }
    return updated
  }

  const taskSummaryFromDetail = (detail: TaskDetail): TaskSummary => ({
    id: detail.id,
    projectId: detail.projectId,
    title: detail.title,
    goal: detail.goal,
    defaultActorRef: detail.defaultActorRef,
    status: detail.status,
    scheduleSpec: detail.scheduleSpec ?? null,
    nextRunAt: detail.nextRunAt ?? null,
    lastRunAt: detail.lastRunAt ?? null,
    latestResultSummary: detail.latestResultSummary ?? null,
    latestFailureCategory: detail.latestFailureCategory ?? null,
    latestTransition: detail.latestTransition ?? null,
    viewStatus: detail.viewStatus,
    attentionReasons: clone(detail.attentionReasons),
    attentionUpdatedAt: detail.attentionUpdatedAt ?? null,
    activeTaskRunId: detail.activeTaskRunId ?? null,
    analyticsSummary: clone(detail.analyticsSummary),
    updatedAt: detail.updatedAt,
  })

  const syncProjectTaskProjection = (projectId: string) => {
    const dashboard = workspaceState.dashboards[projectId]
    if (!dashboard) {
      return
    }

    const recentTasks = [...(dashboard.recentTasks ?? [])].sort((left, right) =>
      right.updatedAt - left.updatedAt || left.id.localeCompare(right.id))

    dashboard.recentTasks = recentTasks
    dashboard.overview = {
      ...dashboard.overview,
      taskCount: recentTasks.length,
      activeTaskCount: recentTasks.filter(task => task.status === 'running').length,
      attentionTaskCount: recentTasks.filter(task => task.viewStatus === 'attention').length,
      scheduledTaskCount: recentTasks.filter(task => typeof task.scheduleSpec === 'string' && task.scheduleSpec.length > 0).length,
    }
  }

  const storeTaskDetail = (detail: TaskDetail) => {
    const key = taskKey(detail.projectId, detail.id)
    taskDetailByKey.set(key, clone(detail))
    taskRunsByKey.set(key, clone(detail.runHistory))
    taskInterventionsByKey.set(key, clone(detail.interventionHistory))

    const dashboard = workspaceState.dashboards[detail.projectId]
    if (!dashboard) {
      return
    }

    const summary = taskSummaryFromDetail(detail)
    const existingIndex = (dashboard.recentTasks ?? []).findIndex(task => task.id === detail.id)
    if (existingIndex >= 0) {
      dashboard.recentTasks = dashboard.recentTasks.map(task =>
        task.id === detail.id ? summary : task)
    } else {
      dashboard.recentTasks = [summary, ...(dashboard.recentTasks ?? [])]
    }
    syncProjectTaskProjection(detail.projectId)
  }

  const findTaskSummary = (projectId: string, taskId: string) => {
    findProjectRecord(projectId)
    const task = (workspaceState.dashboards[projectId]?.recentTasks ?? [])
      .find(record => record.id === taskId)
    if (!task) {
      throw new WorkspaceApiError({
        message: 'task not found',
        status: 404,
        requestId: 'req-task-not-found',
        retryable: false,
        code: 'NOT_FOUND',
      })
    }
    return task
  }

  const ensureTaskDetail = (projectId: string, taskId: string): TaskDetail => {
    const key = taskKey(projectId, taskId)
    const existing = taskDetailByKey.get(key)
    if (existing) {
      return clone(existing)
    }

    const summary = findTaskSummary(projectId, taskId)
    const detail: TaskDetail = {
      id: summary.id,
      projectId: summary.projectId,
      title: summary.title,
      goal: summary.goal,
      brief: '',
      defaultActorRef: summary.defaultActorRef,
      status: summary.status,
      scheduleSpec: summary.scheduleSpec ?? null,
      nextRunAt: summary.nextRunAt ?? null,
      lastRunAt: summary.lastRunAt ?? null,
      latestResultSummary: summary.latestResultSummary ?? null,
      latestFailureCategory: summary.latestFailureCategory ?? null,
      latestTransition: summary.latestTransition ?? null,
      viewStatus: summary.viewStatus,
      attentionReasons: clone(summary.attentionReasons),
      attentionUpdatedAt: summary.attentionUpdatedAt ?? null,
      activeTaskRunId: summary.activeTaskRunId ?? null,
      analyticsSummary: clone(summary.analyticsSummary),
      contextBundle: {
        refs: [],
        pinnedInstructions: '',
        resolutionMode: 'explicit_only',
        lastResolvedAt: null,
      },
      latestDeliverableRefs: [],
      latestArtifactRefs: [],
      runHistory: [],
      interventionHistory: [],
      activeRun: null,
      createdBy: getCurrentUserId(),
      updatedBy: null,
      createdAt: summary.updatedAt,
      updatedAt: summary.updatedAt,
    }
    storeTaskDetail(detail)
    return clone(detail)
  }
  const buildTaskRunSummary = (
    detail: TaskDetail,
    triggerType: TaskRunSummary['triggerType'],
    actorRef: string,
  ): TaskRunSummary => {
    workspaceState.taskRunIdSequence += 1
    const startedAt = Date.now()
    const runtimeRunId = `runtime-run-${workspaceState.taskRunIdSequence}`

    return {
      id: `task-run-${workspaceState.taskRunIdSequence}`,
      taskId: detail.id,
      triggerType,
      status: 'running',
      sessionId: `task-session-${workspaceState.taskRunIdSequence}`,
      conversationId: `task-conversation-${workspaceState.taskRunIdSequence}`,
      runtimeRunId,
      actorRef,
      startedAt,
      completedAt: null,
      resultSummary: null,
      failureCategory: null,
      failureSummary: null,
      viewStatus: 'healthy',
      attentionReasons: [],
      attentionUpdatedAt: null,
      deliverableRefs: [],
      artifactRefs: [],
      latestTransition: {
        kind: 'launched',
        summary: 'Task run started in the runtime.',
        at: startedAt,
        runId: runtimeRunId,
      },
    }
  }
  const applyTaskRunToDetail = (
    detail: TaskDetail,
    run: TaskRunSummary,
  ): TaskDetail => ({
    ...clone(detail),
    status: 'running',
    lastRunAt: run.startedAt,
    activeTaskRunId: run.id,
    latestResultSummary: run.resultSummary ?? null,
    latestFailureCategory: run.failureCategory ?? null,
    latestTransition: clone(run.latestTransition),
    viewStatus: 'healthy',
    attentionReasons: [],
    attentionUpdatedAt: null,
    analyticsSummary: updateTaskAnalyticsForRun(detail.analyticsSummary, run.triggerType),
    latestDeliverableRefs: clone(run.deliverableRefs),
    latestArtifactRefs: clone(run.artifactRefs),
    runHistory: [clone(run), ...detail.runHistory.filter(existing => existing.id !== run.id)],
    activeRun: clone(run),
    updatedBy: getCurrentUserId(),
    updatedAt: run.startedAt,
  })

  const listWorkspaceResources = () => [
    ...workspaceState.workspaceResources,
    ...Object.values(workspaceState.projectResources).flat(),
  ]

  const knowledgeVisibleToCurrentUser = (record: KnowledgeRecord) => {
    if (record.scope === 'personal') {
      return record.ownerUserId === getCurrentUserId()
    }

    if (record.visibility === 'private') {
      return record.ownerUserId === getCurrentUserId()
    }

    return true
  }

  const knowledgeRelevantToProjectContext = (record: KnowledgeRecord, projectId: string) =>
    record.projectId === projectId
    || record.scope === 'workspace'
    || record.scope === 'personal'

  const listAllKnowledgeRecords = () => {
    const records = [
      ...workspaceState.workspaceKnowledge,
      ...Object.values(workspaceState.projectKnowledge).flat(),
    ]

    const seen = new Set<string>()
    return records.filter((record) => {
      if (seen.has(record.id)) {
        return false
      }
      seen.add(record.id)
      return true
    })
  }

  const listWorkspaceKnowledge = () =>
    listAllKnowledgeRecords().filter(knowledgeVisibleToCurrentUser)

  const listProjectKnowledge = (projectId: string) =>
    listAllKnowledgeRecords().filter(record =>
      knowledgeRelevantToProjectContext(record, projectId)
      && knowledgeVisibleToCurrentUser(record),
    )

  const findResourceLocation = (resourceId: string) => {
    const workspaceIndex = workspaceState.workspaceResources.findIndex(record => record.id === resourceId)
    if (workspaceIndex >= 0) {
      return {
        container: 'workspace' as const,
        index: workspaceIndex,
        record: workspaceState.workspaceResources[workspaceIndex]!,
      }
    }

    for (const [projectId, resources] of Object.entries(workspaceState.projectResources)) {
      const index = resources.findIndex(record => record.id === resourceId)
      if (index >= 0) {
        return {
          container: 'project' as const,
          projectId,
          index,
          record: resources[index]!,
        }
      }
    }

    return null
  }

  const storeResourceRecord = (record: WorkspaceResourceRecord) => {
    const located = findResourceLocation(record.id)
    if (!located) {
      if (record.projectId) {
        workspaceState.projectResources[record.projectId] = [
          ...(workspaceState.projectResources[record.projectId] ?? []),
          record,
        ]
      } else {
        workspaceState.workspaceResources = [...workspaceState.workspaceResources, record]
      }
      return
    }

    if (located.container === 'workspace') {
      workspaceState.workspaceResources = workspaceState.workspaceResources.map((item, index) =>
        index === located.index ? record : item)
      return
    }

    workspaceState.projectResources[located.projectId] =
      (workspaceState.projectResources[located.projectId] ?? []).map((item, index) =>
        index === located.index ? record : item)
  }

  const removeResourceRecord = (resourceId: string) => {
    const located = findResourceLocation(resourceId)
    if (!located) {
      return
    }

    if (located.container === 'workspace') {
      workspaceState.workspaceResources = workspaceState.workspaceResources.filter(record => record.id !== resourceId)
    } else {
      workspaceState.projectResources[located.projectId] =
        (workspaceState.projectResources[located.projectId] ?? []).filter(record => record.id !== resourceId)
    }

    delete workspaceState.resourceContents[resourceId]
    delete workspaceState.resourceChildren[resourceId]
  }

  const buildResourceContent = (
    resourceId: string,
    payload: {
      fileName?: string
      contentType?: string
      dataBase64?: string
      externalUrl?: string
      previewKind: ResourcePreviewKind
    },
  ): WorkspaceResourceContentDocument => {
    const content: WorkspaceResourceContentDocument = {
      resourceId,
      previewKind: payload.previewKind,
      fileName: payload.fileName,
      contentType: payload.contentType,
      externalUrl: payload.externalUrl,
    }

    if (!payload.dataBase64) {
      return content
    }

    if (payload.previewKind === 'markdown' || payload.previewKind === 'code' || payload.previewKind === 'text') {
      content.textContent = decodeBase64Text(payload.dataBase64)
      content.byteSize = payload.dataBase64.length
      return content
    }

    content.dataBase64 = payload.dataBase64
    content.byteSize = payload.dataBase64.length
    return content
  }

  const createResourceRecord = (
    input: {
      projectId?: string
      kind: ProjectResourceKind
      name: string
      origin?: WorkspaceResourceRecord['origin']
      location?: string
      scope?: WorkspaceResourceScope
      visibility?: WorkspaceResourceVisibility
      sourceArtifactId?: string
      tags?: string[]
      storagePath?: string
      contentType?: string
      byteSize?: number
      previewKind?: ResourcePreviewKind
      status?: WorkspaceResourceRecord['status']
    },
  ): WorkspaceResourceRecord => {
    const project = input.projectId ? findProjectRecord(input.projectId) : null
    const previewKind = input.previewKind ?? inferPreviewKind(input.kind, input.contentType, input.location || input.name)
    const storagePath = input.storagePath
      ?? (input.kind === 'url'
        ? undefined
        : `${project?.resourceDirectory ?? workspaceResourceBaseDirectory()}/${input.name}`)

    return {
      id: `res-${Date.now()}-${Math.random().toString(16).slice(2, 8)}`,
      workspaceId: workspaceState.workspace.id,
      projectId: input.projectId,
      kind: input.kind,
      name: input.name,
      location: input.location ?? storagePath,
      origin: input.origin ?? 'source',
      scope: input.scope ?? (input.projectId ? 'project' : 'workspace'),
      visibility: input.visibility ?? 'public',
      ownerUserId: getCurrentUserId(),
      storagePath,
      contentType: input.contentType,
      byteSize: input.byteSize,
      previewKind,
      sourceArtifactId: input.sourceArtifactId,
      status: input.status ?? 'healthy',
      updatedAt: Date.now(),
      tags: [...(input.tags ?? [])],
    }
  }

  const importResourceRecord = (
    input: WorkspaceResourceImportInput,
    projectId?: string,
  ) => {
    const { rootDirName, files } = normalizeImportedFiles(input)
    const isFolder = Boolean(rootDirName) || files.length > 1 || files.some(file => file.relativePath.includes('/'))
    const project = projectId ? findProjectRecord(projectId) : null
    const baseDirectory = project?.resourceDirectory ?? workspaceResourceBaseDirectory()

    if (isFolder) {
      const folderName = input.name.trim() || rootDirName || 'imported-folder'
      const record = createResourceRecord({
        projectId,
        kind: 'folder',
        name: folderName,
        scope: input.scope,
        visibility: input.visibility,
        tags: input.tags,
        storagePath: `${baseDirectory}/${rootDirName || folderName}`,
        previewKind: 'folder',
      })
      workspaceState.resourceChildren[record.id] = files.map((file) => ({
        name: file.fileName,
        relativePath: file.relativePath,
        kind: 'file',
        previewKind: inferPreviewKind('file', file.contentType, file.relativePath),
        contentType: file.contentType,
        byteSize: file.byteSize,
        updatedAt: record.updatedAt,
      }))
      return record
    }

    const file = files[0]
    if (!file) {
      throw new WorkspaceApiError({
        message: 'resource import files are required',
        status: 400,
        requestId: 'req-resource-import-files',
        retryable: false,
        code: 'INVALID_INPUT',
      })
    }

    const record = createResourceRecord({
      projectId,
      kind: 'file',
      name: input.name.trim() || file.fileName,
      scope: input.scope,
      visibility: input.visibility,
      tags: input.tags,
      storagePath: `${baseDirectory}/${file.relativePath}`,
      contentType: file.contentType,
      byteSize: file.byteSize,
      previewKind: inferPreviewKind('file', file.contentType, file.fileName),
    })
    workspaceState.resourceContents[record.id] = buildResourceContent(record.id, {
      fileName: file.fileName,
      contentType: file.contentType,
      dataBase64: file.dataBase64,
      previewKind: record.previewKind,
    })
    return record
  }

  const getMenuRequiredPermissionCodes = (menuId: string) => {
    switch (menuId) {
      case 'menu-workspace-access-control':
        return ['access.users.read']
      default:
        return ['workspace.overview.read']
    }
  }

  const getCurrentUserSubjects = (userId: string) => {
    const assignments = accessUserOrgAssignments.filter(record => record.userId === userId)

    return {
      userId,
      orgUnitIds: Array.from(new Set(assignments.map(record => record.orgUnitId))),
      positionIds: Array.from(new Set(assignments.flatMap(record => record.positionIds))),
      userGroupIds: Array.from(new Set(assignments.flatMap(record => record.userGroupIds))),
    }
  }

  const roleBindingMatchesUser = (
    binding: {
      subjectType: string
      subjectId: string
    },
    userId: string,
  ) => {
    const subjects = getCurrentUserSubjects(userId)
    switch (binding.subjectType) {
      case 'user':
        return binding.subjectId === subjects.userId
      case 'org_unit':
      case 'org-unit':
        return subjects.orgUnitIds.includes(binding.subjectId)
      case 'position':
        return subjects.positionIds.includes(binding.subjectId)
      case 'user_group':
      case 'user-group':
        return subjects.userGroupIds.includes(binding.subjectId)
      default:
        return false
    }
  }

  const getEffectiveRoleRecords = (userId: string) => {
    if (!accessUsers.find(record => record.id === userId)) {
      return []
    }

    const matchedBindings = accessRoleBindings.filter(binding => roleBindingMatchesUser(binding, userId))
    const deniedRoleIds = new Set(
      matchedBindings
        .filter(binding => binding.effect === 'deny')
        .map(binding => binding.roleId),
    )
    const allowedRoleIds = new Set(
      matchedBindings
        .filter(binding => binding.effect !== 'deny')
        .map(binding => binding.roleId),
    )

    return accessRoles.filter(role => allowedRoleIds.has(role.id) && !deniedRoleIds.has(role.id))
  }

  const getEffectivePermissionCodes = (userId: string) => {
    return Array.from(new Set(getEffectiveRoleRecords(userId).flatMap(role => role.permissionCodes)))
  }

  const getVisibleMenuIds = (userId: string) => {
    const featureCodes = new Set(getFeatureCodes(userId))

    return workspaceState.menus
      .filter((menu) => {
        const policy = accessMenuPolicies.find(record => record.menuId === menu.id)
        const enabled = policy?.enabled ?? (menu.status === 'active')
        const visibility = policy?.visibility ?? 'inherit'
        const featureAllowed = featureCodes.has(getFeatureCode(menu.id, menu.routeName))
        if (!enabled || visibility === 'hidden') {
          return false
        }
        if (visibility === 'visible') {
          return true
        }
        return featureAllowed
      })
      .map(menu => menu.id)
  }

  const getFeatureCodes = (userId: string) => {
    const effectivePermissionCodes = new Set(getEffectivePermissionCodes(userId))
    return workspaceState.menus
      .filter(menu => getMenuRequiredPermissionCodes(menu.id).every(code => effectivePermissionCodes.has(code)))
      .map(menu => getFeatureCode(menu.id, menu.routeName))
  }

  const buildMenuGateResults = (userId: string) => {
    const featureCodes = new Set(getFeatureCodes(userId))
    return workspaceState.menus.map((menu) => {
      const policy = accessMenuPolicies.find(record => record.menuId === menu.id)
      const enabled = policy?.enabled ?? (menu.status === 'active')
      const visibility = policy?.visibility ?? 'inherit'
      const featureCode = getFeatureCode(menu.id, menu.routeName)
      const featureAllowed = featureCodes.has(featureCode)
      const allowed = enabled && (visibility === 'visible' || (visibility !== 'hidden' && featureAllowed))

      return {
        menuId: menu.id,
        featureCode,
        allowed,
        reason: allowed
          ? undefined
          : !enabled
              ? 'menu disabled by policy'
              : visibility === 'hidden'
                  ? 'menu hidden by policy'
                  : 'required permission missing',
      }
    })
  }

  const buildProtectedResourceCatalog = (): ProtectedResourceDescriptor[] => [
      ...workspaceState.agents.map(agent => ({
        id: agent.id,
        resourceType: 'agent',
        resourceSubtype: agent.scope,
        name: agent.name,
        projectId: agent.projectId,
        ownerSubjectType: undefined,
        ownerSubjectId: undefined,
        tags: clone(agent.tags),
        classification: 'internal',
      })),
      ...workspaceState.workspaceResources.map(resource => ({
        id: resource.id,
        resourceType: 'resource',
        resourceSubtype: resource.kind,
        name: resource.name,
        projectId: resource.projectId,
        ownerSubjectType: undefined,
        ownerSubjectId: undefined,
        tags: clone(resource.tags),
        classification: 'internal',
      })),
      ...workspaceState.workspaceKnowledge.map(entry => ({
        id: entry.id,
        resourceType: 'knowledge',
        resourceSubtype: entry.sourceType,
        name: entry.title,
        projectId: entry.projectId,
        ownerSubjectType: undefined,
        ownerSubjectId: undefined,
        tags: [],
        classification: 'internal',
      })),
      ...workspaceState.tools.map(tool => ({
        id: tool.id,
        resourceType: preciseToolResourceType(tool.kind),
        resourceSubtype: tool.kind,
        name: tool.name,
        projectId: undefined,
        ownerSubjectType: undefined,
        ownerSubjectId: undefined,
        tags: [],
        classification: 'internal',
      })),
    ]

  const buildProtectedResources = () =>
    buildProtectedResourceCatalog().flatMap((descriptor) => {
      const metadata = protectedResourceMetadata.get(protectedResourceKey(descriptor.resourceType, descriptor.id))
      if (!metadata) {
        return []
      }

      return [{
        ...descriptor,
        resourceSubtype: metadata.resourceSubtype ?? descriptor.resourceSubtype,
        projectId: metadata.projectId ?? descriptor.projectId,
        ownerSubjectType: metadata.ownerSubjectType ?? descriptor.ownerSubjectType,
        ownerSubjectId: metadata.ownerSubjectId ?? descriptor.ownerSubjectId,
        tags: metadata.tags.length ? clone(metadata.tags) : descriptor.tags,
        classification: metadata.classification || descriptor.classification,
      }]
    })

  const resolveProtectedResourceDescriptor = (resourceType: string, resourceId: string) => {
    const descriptor = buildProtectedResourceCatalog().find(record =>
      record.resourceType === resourceType && record.id === resourceId,
    )
    if (!descriptor) {
      return null
    }

    const metadata = protectedResourceMetadata.get(protectedResourceKey(descriptor.resourceType, descriptor.id))
    if (!metadata) {
      return descriptor
    }

    return {
      ...descriptor,
      resourceSubtype: metadata.resourceSubtype ?? descriptor.resourceSubtype,
      projectId: metadata.projectId ?? descriptor.projectId,
      ownerSubjectType: metadata.ownerSubjectType ?? descriptor.ownerSubjectType,
      ownerSubjectId: metadata.ownerSubjectId ?? descriptor.ownerSubjectId,
      tags: metadata.tags.length ? clone(metadata.tags) : descriptor.tags,
      classification: metadata.classification || descriptor.classification,
    }
  }

  const buildResourceActionGrants = (permissionCodes: string[]) => {
    const grants = new Map<string, Set<string>>()
    const appendGrant = (resourceType: string, actions: string[]) => {
      const current = grants.get(resourceType) ?? new Set<string>()
      actions.forEach(action => current.add(action))
      grants.set(resourceType, current)
    }

    permissionCodes.forEach((code) => {
      if (code === 'workspace.overview.read') {
        appendGrant('workspace', ['overview.read'])
        return
      }

      if (code.startsWith('access.')) {
        const segments = code.split('.')
        if (segments.length === 3) {
          appendGrant(`${segments[0]}.${segments[1]}`, [segments[2]])
        }
        return
      }

      if (code.startsWith('tool.')) {
        const segments = code.split('.')
        if (segments.length >= 3) {
          appendGrant(`${segments[0]}.${segments[1]}`, [segments.slice(2).join('.')])
        }
        return
      }

      if (code.startsWith('runtime.config.')) {
        const segments = code.split('.')
        if (segments.length >= 4) {
          appendGrant(segments.slice(0, 3).join('.'), [segments.slice(3).join('.')])
        }
        return
      }

      if (code.startsWith('runtime.')) {
        const segments = code.split('.')
        if (segments.length >= 3) {
          appendGrant(segments.slice(0, 2).join('.'), [segments.slice(2).join('.')])
        }
        return
      }

      if (code.startsWith('provider-credential.')) {
        const segments = code.split('.')
        if (segments.length >= 2) {
          appendGrant('provider-credential', [segments.slice(1).join('.')])
        }
        return
      }

      const [resourceType, ...actions] = code.split('.')
      if (resourceType && actions.length > 0) {
        appendGrant(resourceType, [actions.join('.')])
      }
    })

    return Array.from(grants.entries()).map(([resourceType, actions]) => ({
      resourceType,
      actions: Array.from(actions),
    }))
  }

  const buildAuthorizationSnapshot = () => {
    const user = getCurrentUser()
    if (!user) {
      throw new Error('Expected current user in workspace fixture')
    }

    const effectiveRoles = getEffectiveRoleRecords(user.id)
    const effectivePermissionCodes = getEffectivePermissionCodes(user.id)
    const featureCodes = getFeatureCodes(user.id)
    const menuGates = buildMenuGateResults(user.id)
    const visibleMenuIds = menuGates.filter(menu => menu.allowed).map(menu => menu.menuId)

    return {
      principal: clone(user),
      orgAssignments: accessUserOrgAssignments.filter(assignment => assignment.userId === user.id),
      effectiveRoleIds: effectiveRoles.map(role => role.id),
      effectiveRoles: effectiveRoles.map(role => ({
        id: role.id,
        code: role.code,
        name: role.name,
        description: role.description,
        source: role.source,
        editable: role.editable,
        status: role.status,
        permissionCodes: clone(role.permissionCodes),
      })),
      effectivePermissionCodes,
      featureCodes,
      visibleMenuIds,
      menuGates,
      resourceActionGrants: buildResourceActionGrants(effectivePermissionCodes),
    }
  }

  const hasAnyPermission = (permissionCodes: Set<string>, candidates: string[]) =>
    candidates.some(code => permissionCodes.has(code))

  const accessRoleTemplates = [
    {
      code: 'owner',
      name: 'Owner',
      description: 'Manage the workspace and govern its policies.',
      managedRoleCodes: ['system.owner'],
      editable: false,
    },
    {
      code: 'admin',
      name: 'Admin',
      description: 'Manage members, roles, and day-to-day workspace operations.',
      managedRoleCodes: ['system.admin'],
      editable: false,
    },
  ] as const

  const accessRolePresets = [
    {
      code: 'owner',
      name: 'Owner',
      description: 'Full workspace control for the accountable owner.',
      recommendedFor: 'Workspace owners',
      templateCodes: ['owner'],
      capabilityBundleCodes: ['workspace_governance', 'member_management', 'security_and_audit'],
    },
    {
      code: 'admin',
      name: 'Admin',
      description: 'Operate the workspace without exposing low-level policy detail by default.',
      recommendedFor: 'Workspace operators',
      templateCodes: ['admin'],
      capabilityBundleCodes: ['member_management', 'project_and_resource_access', 'automation_and_tools'],
    },
  ] as const

  const accessCapabilityBundles = [
    {
      code: 'workspace_governance',
      name: 'Workspace governance',
      description: 'Manage organization structure, roles, and governance settings.',
      permissionCodes: ['access.roles.manage', 'access.org.read', 'access.policies.read', 'access.menus.read'],
    },
    {
      code: 'member_management',
      name: 'Member management',
      description: 'Invite, update, and organize workspace members.',
      permissionCodes: ['access.users.read', 'access.users.manage'],
    },
    {
      code: 'project_and_resource_access',
      name: 'Project and resource access',
      description: 'Grant access to projects, resources, and protected content.',
      permissionCodes: ['access.roles.read', 'access.policies.read'],
    },
    {
      code: 'automation_and_tools',
      name: 'Automation and tools',
      description: 'Operate tools and automation capabilities for the workspace.',
      permissionCodes: ['tool.catalog.view'],
    },
    {
      code: 'security_and_audit',
      name: 'Security and audit',
      description: 'Inspect sessions and audit activity when tighter control is needed.',
      permissionCodes: ['access.sessions.read'],
    },
  ] as const

  const presetNameByCode = new Map(accessRolePresets.map(preset => [preset.code, preset.name]))
  const presetCodeByRoleCode = new Map(accessRoleTemplates.flatMap(template =>
    template.managedRoleCodes.map(roleCode => [roleCode, template.code] as const),
  ))
  const isAdvancedDataPolicy = (policy: typeof accessDataPolicies[number]) =>
    policy.resourceType !== 'project'
    || policy.scopeType !== 'selected-projects'
    || policy.effect !== 'allow'

  const buildAccessMemberSummary = (userId: string): AccessMemberSummary => {
    const user = accessUsers.find(record => record.id === userId)
    if (!user) {
      throw new Error(`Unknown access user ${userId}`)
    }

    const directUserRoles = accessRoleBindings
      .filter(binding => binding.subjectType === 'user' && binding.subjectId === userId && binding.effect !== 'deny')
      .map(binding => accessRoles.find(role => role.id === binding.roleId))
      .filter((role): role is NonNullable<typeof accessRoles[number]> => Boolean(role))
    const directPresetCodes = Array.from(new Set(
      directUserRoles
        .map(role => presetCodeByRoleCode.get(role.code))
        .filter((code): code is string => Boolean(code)),
    ))
    const effectiveRoles = getEffectiveRoleRecords(userId)
    const effectiveRoleCodes = new Set(effectiveRoles.map(role => role.code))
    const hasExtraEffectiveRoles = directPresetCodes.length > 0
      && (
        effectiveRoleCodes.size > 1
        || (directPresetCodes[0] && !effectiveRoleCodes.has(`system.${directPresetCodes[0]}`))
      )

    let primaryPresetCode: AccessMemberSummary['primaryPresetCode'] = null
    let primaryPresetName = NO_PRESET_ASSIGNED_NAME

    if (directPresetCodes.length === 1 && !hasExtraEffectiveRoles && effectiveRoles.length > 0) {
      primaryPresetCode = directPresetCodes[0]
      primaryPresetName = presetNameByCode.get(primaryPresetCode) ?? directPresetCodes[0]
    } else if (directPresetCodes.length > 0 || effectiveRoles.length > 0) {
      primaryPresetCode = hasExtraEffectiveRoles || directPresetCodes.length > 1
        ? MIXED_ACCESS_CODE
        : CUSTOM_ACCESS_CODE
      primaryPresetName = primaryPresetCode === MIXED_ACCESS_CODE ? MIXED_ACCESS_NAME : CUSTOM_ACCESS_NAME
    }

    const hasOrgAssignments = accessUserOrgAssignments.some(assignment =>
      assignment.userId === userId
      && (
        assignment.orgUnitId !== ROOT_ORG_UNIT_ID
        || assignment.positionIds.length > 0
        || assignment.userGroupIds.length > 0
      ),
    )

    return {
      user: clone(user),
      primaryPresetCode,
      primaryPresetName,
      effectiveRoles: effectiveRoles.map(role => ({
        id: role.id,
        code: role.code,
        name: role.name,
        source: role.source,
      })),
      effectiveRoleNames: effectiveRoles.map(role => role.name),
      hasOrgAssignments,
    }
  }

  const buildAccessMembers = () => accessUsers.map(user => buildAccessMemberSummary(user.id))

  const buildAccessExperience = (): AccessExperienceResponse => {
    const user = getCurrentUser()
    if (!user) {
      throw new Error('Expected current user in workspace fixture')
    }

    const permissionCodes = new Set(getEffectivePermissionCodes(user.id))
    const memberCount = accessUsers.length
    const hasOrgStructure = accessOrgUnits.some(unit => unit.id !== ROOT_ORG_UNIT_ID)
      || accessPositions.length > 0
      || accessUserGroups.length > 0
      || accessUserOrgAssignments.some(assignment =>
        assignment.orgUnitId !== ROOT_ORG_UNIT_ID
        || assignment.positionIds.length > 0
        || assignment.userGroupIds.length > 0,
      )
    const hasCustomRoles = accessRoles.some(role => role.source === 'custom')
    const hasAdvancedPolicies = accessDataPolicies.some(policy => isAdvancedDataPolicy(policy))
    const hasMenuGovernance = accessMenuPolicies.length > 0
    const hasResourceGovernance = accessResourcePolicies.length > 0 || protectedResourceMetadata.size > 0
    const enterpriseLike = hasOrgStructure || hasCustomRoles || hasAdvancedPolicies || hasMenuGovernance || hasResourceGovernance
    const membersAllowed = hasAnyPermission(permissionCodes, ['access.users.read', 'access.users.manage'])
    const sectionGrants = [
      {
        section: 'members' as const,
        allowed: membersAllowed,
      },
      {
        section: 'access' as const,
        allowed: membersAllowed,
      },
      {
        section: 'governance' as const,
        allowed: hasAnyPermission(permissionCodes, [
          'access.org.read',
          'access.org.manage',
          'access.policies.read',
          'access.policies.manage',
          'access.menus.read',
          'access.menus.manage',
          'access.sessions.read',
          'access.sessions.manage',
          'audit.read',
        ]),
      },
    ]
    const firstAllowedSection = sectionGrants.find(grant => grant.allowed)?.section ?? 'members'
    const recommendedLandingSection = sectionGrants[0].allowed && memberCount > 1
      ? 'members'
      : sectionGrants[1].allowed
        ? 'access'
        : enterpriseLike && sectionGrants[2].allowed
          ? 'governance'
          : firstAllowedSection

    return {
      summary: {
        experienceLevel: enterpriseLike ? 'enterprise' : (memberCount > 1 ? 'team' : 'personal'),
        memberCount,
        hasOrgStructure,
        hasCustomRoles,
        hasAdvancedPolicies,
        hasMenuGovernance,
        hasResourceGovernance,
        recommendedLandingSection,
      },
      sectionGrants,
      roleTemplates: accessRoleTemplates.map(template => ({
        ...template,
        managedRoleCodes: [...template.managedRoleCodes],
      })),
      rolePresets: accessRolePresets.map(preset => ({
        ...preset,
        templateCodes: [...preset.templateCodes],
        capabilityBundleCodes: [...preset.capabilityBundleCodes],
      })),
      capabilityBundles: accessCapabilityBundles.map(bundle => ({
        ...bundle,
        permissionCodes: [...bundle.permissionCodes],
      })),
      counts: {
        auditEventCount: auditRecords.length,
        customRoleCount: accessRoles.filter(role => role.source === 'custom').length,
        dataPolicyCount: accessDataPolicies.length,
        menuPolicyCount: accessMenuPolicies.length,
        orgUnitCount: accessOrgUnits.length,
        protectedResourceCount: protectedResourceMetadata.size,
        resourcePolicyCount: accessResourcePolicies.length,
        sessionCount: fixtureSessions.length,
      },
    }
  }

  const buildAccessSessionRecords = () =>
    fixtureSessions.map((session) => {
        const user = accessUsers.find(record => record.id === session.userId)
        ?? accessUsers.find(record => record.id === workspaceState.workspace.ownerUserId)
        ?? accessUsers[0]

      return {
        sessionId: session.sessionId,
        userId: session.userId,
        username: user?.username ?? session.userId,
        displayName: user?.displayName ?? session.userId,
        clientAppId: session.clientAppId,
        createdAt: session.createdAt,
        expiresAt: session.expiresAt,
        status: session.status,
        current: session.sessionId === defaultSession.session.id,
      }
    })

  const assertCustomRoleCode = (code: string) => {
    if (!code.startsWith('system.')) {
      return
    }

    throw new WorkspaceApiError({
      message: 'reserved managed role namespace',
      status: 400,
      requestId: 'req-access-role-reserved-code',
      retryable: false,
      code: 'INVALID_INPUT',
    })
  }

  const registerBootstrapAdmin = (request: {
    username: string
    displayName: string
    avatar: {
      contentType: string
      dataBase64: string
    }
  }) => {
    const ownerId = 'user-owner'
    const ownerAvatar = `data:${request.avatar.contentType};base64,${request.avatar.dataBase64}`
    const ownerRecord: UserRecordSummary = {
      id: ownerId,
      username: request.username,
      displayName: request.displayName,
      avatar: ownerAvatar,
      status: 'active',
      passwordState: 'set',
    }

    workspaceState.workspace = {
      ...workspaceState.workspace,
      bootstrapStatus: 'ready',
      ownerUserId: ownerId,
    }
    workspaceState.systemBootstrap = {
      ...workspaceState.systemBootstrap,
      workspace: clone(workspaceState.workspace),
      setupRequired: false,
      ownerReady: true,
    }
    workspaceState.overview = {
      ...workspaceState.overview,
      workspace: clone(workspaceState.workspace),
    }
    workspaceState.users = [
      ownerRecord,
      ...workspaceState.users.filter(record => record.id !== ownerId),
    ]
    workspaceState.currentUserId = ownerId
    accessUsers = [
      {
        id: ownerRecord.id,
        username: ownerRecord.username,
        displayName: ownerRecord.displayName,
        status: ownerRecord.status,
        passwordState: ownerRecord.passwordState,
      },
      ...accessUsers.filter(record => record.id !== ownerId),
    ]
    accessUserOrgAssignments = [
      {
        userId: ownerId,
        orgUnitId: 'org-root',
        isPrimary: true,
        positionIds: [],
        userGroupIds: [],
      },
      ...accessUserOrgAssignments.filter(record => record.userId !== ownerId),
    ]
    accessRoleBindings = [
      {
        id: 'binding-user-owner-role-owner',
        roleId: 'role-owner',
        subjectType: 'user',
        subjectId: ownerId,
        effect: 'allow',
      },
      ...accessRoleBindings.filter(record => record.subjectId !== ownerId),
    ]

    return {
      session: buildEnterpriseSession(ownerId),
      workspace: clone(workspaceState.workspace),
    }
  }

  const getWorkspaceRuntimeDocument = () => {
    const source = workspaceState.runtimeWorkspaceConfig.sources.find(item => item.scope === 'workspace')
    const current = source?.document && typeof source.document === 'object' && !Array.isArray(source.document)
      ? clone(source.document)
      : {}
    return {
      source,
      current,
    }
  }

  const updateWorkspaceRuntimeDocument = (
    transform: (current: Record<string, any>) => Record<string, any>,
  ) => {
    const { source, current } = getWorkspaceRuntimeDocument()
    const next = transform(current)
    if (source) {
      source.document = clone(next)
      source.exists = true
      source.loaded = true
    }
    workspaceState.runtimeWorkspaceConfig = {
      ...workspaceState.runtimeWorkspaceConfig,
      effectiveConfig: clone(next),
      effectiveConfigHash: `${workspaceState.workspace.id}-cfg-hash-${Date.now()}`,
      validation: {
        valid: true,
        errors: [],
        warnings: [],
      },
    }
  }

  const syncManagedToolConfig = () => {
    syncManagementProjection()
    updateWorkspaceRuntimeDocument((current) => {
      const currentToolCatalog = typeof current.toolCatalog === 'object' && current.toolCatalog && !Array.isArray(current.toolCatalog)
        ? current.toolCatalog
        : {}
      return {
        ...current,
        toolCatalog: {
          ...currentToolCatalog,
          disabledSourceKeys: workspaceState.toolCatalog.entries
            .filter(entry => entry.disabled)
            .map(entry => entry.sourceKey),
        },
        mcpServers: Object.fromEntries(
          Object.values(workspaceState.mcpDocuments).map(document => [document.serverName, clone(document.config)]),
        ),
      }
    })
  }

  const syncManagementProjection = () => {
    workspaceState.managementProjection = deriveCapabilityManagementProjection(
      workspaceState.toolCatalog.entries,
    )
  }

  const findToolCatalogEntry = (sourceKey: string) =>
    workspaceState.toolCatalog.entries.find(entry => entry.sourceKey === sourceKey)

  const replaceToolCatalogEntry = (entry: WorkspaceToolCatalogEntry) => {
    const index = workspaceState.toolCatalog.entries.findIndex(item => item.id === entry.id)
    if (index === -1) {
      workspaceState.toolCatalog.entries = [...workspaceState.toolCatalog.entries, entry]
      return
    }
    workspaceState.toolCatalog.entries = workspaceState.toolCatalog.entries.map((item, itemIndex) => itemIndex === index ? entry : item)
  }

  const ensureSkillDocument = (skillId: string) => {
    const document = workspaceState.skillDocuments[skillId]
    if (!document) {
      throw new WorkspaceApiError({
        message: 'skill not found',
        status: 404,
        requestId: 'req-skill-not-found',
        retryable: false,
        code: 'NOT_FOUND',
      })
    }
    return document
  }

  const ensureSkillFiles = (skillId: string) => {
    const files = workspaceState.skillFiles[skillId]
    if (!files) {
      throw new WorkspaceApiError({
        message: 'skill files not found',
        status: 404,
        requestId: 'req-skill-files-not-found',
        retryable: false,
        code: 'NOT_FOUND',
      })
    }
    return files
  }

  const replaceSkillState = (
    document: WorkspaceSkillDocument,
    files: Record<string, WorkspaceSkillFileDocument>,
    disabled?: boolean,
  ) => {
    workspaceState.skillDocuments = {
      ...workspaceState.skillDocuments,
      [document.id]: clone(document),
    }
    workspaceState.skillFiles = {
      ...workspaceState.skillFiles,
      [document.id]: cloneSkillFiles(files),
    }
    const currentEntry = findToolCatalogEntry(document.sourceKey)
    replaceToolCatalogEntry(
      createSkillCatalogEntry(
        workspaceState.workspace.id,
        document,
        disabled ?? currentEntry?.disabled ?? false,
      ),
    )
  }

  const createManagedSkill = (
    slug: string,
    files?: Record<string, WorkspaceSkillFileDocument>,
  ) => {
    const normalizedSlug = slug.trim()
    const relativePath = `data/skills/${normalizedSlug}/SKILL.md`
    const sourceKey = `skill:${relativePath}`
    if (!normalizedSlug) {
      throw new WorkspaceApiError({
        message: 'skill slug is required',
        status: 400,
        requestId: 'req-skill-slug-required',
        retryable: false,
        code: 'INVALID_INPUT',
      })
    }
    if (findToolCatalogEntry(sourceKey)) {
      throw new WorkspaceApiError({
        message: 'skill already exists',
        status: 409,
        requestId: 'req-skill-exists',
        retryable: false,
        code: 'CONFLICT',
      })
    }

    const skillId = `skill-workspace-${normalizedSlug}`
    const rootPath = `data/skills/${normalizedSlug}`
    const nextFiles = files
      ? cloneSkillFiles(files)
      : {
          'SKILL.md': createSkillFileDocument(skillId, sourceKey, rootPath, 'SKILL.md', {
            content: createSkillTemplate(normalizedSlug),
          }),
        }

    const document = createSkillDocument(
      skillId,
      sourceKey,
      skillNameFromContent(nextFiles['SKILL.md']?.content ?? '', normalizedSlug),
      skillDescriptionFromContent(nextFiles['SKILL.md']?.content ?? '', `Workspace skill ${normalizedSlug}`),
      relativePath,
      true,
      nextFiles,
      relativePath,
    )
    replaceSkillState(document, nextFiles, false)
    syncManagedToolConfig()
    return document
  }

  const ensureWorkspaceOwnedSkillDocument = (skillId: string) => {
    const document = ensureSkillDocument(skillId)
    if (!document.workspaceOwned) {
      throw new WorkspaceApiError({
        message: 'external skills are read-only',
        status: 400,
        requestId: 'req-skill-readonly',
        retryable: false,
        code: 'INVALID_INPUT',
      })
    }
    return document
  }

  const normalizeImportedDirectoryEntries = (files: WorkspaceDirectoryUploadEntry[]) =>
    files.map(file => ({
      ...file,
      relativePath: file.relativePath.replace(/^\/+/, ''),
    }))

  const activeSessionToken = session?.token ?? ''

  const requireAuthenticatedSession = () => {
    if (!session?.token) {
      throw new Error(`Workspace session is unavailable for ${connection.workspaceConnectionId}`)
    }

    const currentSession = fixtureSessions.find(record => record.token === session.token)
    if (!currentSession || currentSession.status !== 'active') {
      throw new WorkspaceApiError({
        message: `Workspace session expired for ${connection.workspaceConnectionId}`,
        status: 401,
        requestId: 'req-fixture-session-expired',
        retryable: false,
        code: 'SESSION_EXPIRED',
      })
    }

    return currentSession
  }

  const shouldRequireSession = (domain: string, method: string) => {
    if (domain === 'system') {
      return false
    }
    if (domain === 'auth') {
      return method === 'session'
    }
    if (domain === 'enterpriseAuth') {
      return method === 'session'
    }
    return true
  }

  const applySessionGuards = (client: WorkspaceClient): WorkspaceClient => {
    const wrapped = client as Record<string, any>
    for (const [domain, api] of Object.entries(wrapped)) {
      if (!api || typeof api !== 'object') {
        continue
      }
      for (const [method, handler] of Object.entries(api)) {
        if (typeof handler !== 'function' || !shouldRequireSession(domain, method)) {
          continue
        }
        api[method] = async (...args: unknown[]) => {
          requireAuthenticatedSession()
          return await handler(...args)
        }
      }
    }
    return client
  }

  const buildAgentBundlePreview = (
    input: ImportWorkspaceAgentBundlePreviewInput | ImportWorkspaceAgentBundleInput,
    targetProjectId?: string,
  ): ImportWorkspaceAgentBundlePreview => {
    const files = normalizeImportedDirectoryEntries(input.files)
    const projectScoped = Boolean(targetProjectId)
    return {
      departments: ['Imported Bundle'],
      departmentCount: 1,
      detectedAgentCount: 1,
      importableAgentCount: 1,
      detectedTeamCount: 1,
      importableTeamCount: 1,
      createCount: 2,
      updateCount: 0,
      skipCount: 0,
      failureCount: 0,
      uniqueSkillCount: 1,
      uniqueMcpCount: 1,
      avatarCount: 2,
      filteredFileCount: files.length,
      agents: [
        {
          sourceId: projectScoped ? 'imported-project-agent' : 'imported-workspace-agent',
          name: projectScoped ? 'Imported Project Agent' : 'Imported Workspace Agent',
          department: 'Imported Bundle',
          action: 'create',
          skillSlugs: ['imported-skill'],
          mcpServerNames: ['imported-mcp'],
        },
      ],
      teams: [
        {
          sourceId: projectScoped ? 'imported-project-team' : 'imported-workspace-team',
          name: projectScoped ? 'Imported Project Team' : 'Imported Workspace Team',
          action: 'create',
          leaderName: projectScoped ? 'Imported Project Agent' : 'Imported Workspace Agent',
          memberNames: [projectScoped ? 'Imported Project Agent' : 'Imported Workspace Agent'],
          agentSourceIds: [projectScoped ? 'imported-project-agent' : 'imported-workspace-agent'],
        },
      ],
      skills: [
        {
          slug: 'imported-skill',
          skillId: 'skill-imported-skill',
          name: 'imported-skill',
          action: 'create',
          contentHash: 'hash-imported-skill',
          fileCount: Math.max(1, files.length),
          sourceIds: [projectScoped ? 'imported-project-agent' : 'imported-workspace-agent'],
          departments: ['Imported Bundle'],
          agentNames: [projectScoped ? 'Imported Project Agent' : 'Imported Workspace Agent'],
        },
      ],
      mcps: [
        {
          serverName: 'imported-mcp',
          action: 'create',
          contentHash: 'hash-imported-mcp',
          sourceIds: [projectScoped ? 'imported-project-agent' : 'imported-workspace-agent'],
          consumerNames: [projectScoped ? 'Imported Project Agent' : 'Imported Workspace Agent'],
          referencedOnly: false,
        },
      ],
      avatars: [
        {
          sourceId: projectScoped ? 'imported-project-agent' : 'imported-workspace-agent',
          ownerKind: 'agent',
          ownerName: projectScoped ? 'Imported Project Agent' : 'Imported Workspace Agent',
          fileName: 'avatar.png',
          generated: false,
        },
        {
          sourceId: projectScoped ? 'imported-project-team' : 'imported-workspace-team',
          ownerKind: 'team',
          ownerName: projectScoped ? 'Imported Project Team' : 'Imported Workspace Team',
          fileName: 'team-avatar.png',
          generated: true,
        },
      ],
      issues: [],
    }
  }

  const buildAgentBundleExport = (
    input: ExportWorkspaceAgentBundleInput,
  ): ExportWorkspaceAgentBundleResult => {
    const rootDirName = input.mode === 'single'
      ? input.agentIds[0] || input.teamIds[0] || 'templates'
      : 'templates'
    const files = [
      ...input.agentIds.map((agentId) => ({
        fileName: `${agentId}.md`,
        contentType: 'text/markdown',
        byteSize: 64,
        dataBase64: btoa(`# ${agentId}\n`),
        relativePath: input.mode === 'single'
          ? `${rootDirName}/${agentId}.md`
          : `templates/${agentId}/${agentId}.md`,
      })),
      ...input.teamIds.map((teamId) => ({
        fileName: `${teamId}说明.md`,
        contentType: 'text/markdown',
        byteSize: 64,
        dataBase64: btoa(`# ${teamId}\n`),
        relativePath: input.mode === 'single'
          ? `${rootDirName}/${teamId}说明.md`
          : `templates/${teamId}/${teamId}说明.md`,
      })),
    ]

    return {
      rootDirName,
      fileCount: files.length,
      agentCount: input.agentIds.length,
      teamCount: input.teamIds.length,
      skillCount: Math.max(0, input.agentIds.length + input.teamIds.length),
      mcpCount: input.agentIds.length ? 1 : 0,
      avatarCount: input.agentIds.length + input.teamIds.length,
      files,
      issues: [],
    }
  }

  const normalizeCopySlug = (name: string) => name.toLowerCase().replace(/[^a-z0-9]+/g, '-')

  const upsertCopiedAgent = (record: ReturnType<typeof normalizeAgentRecord>) => {
    workspaceState.agents = [...workspaceState.agents.filter(item => item.id !== record.id), record]
  }

  const upsertCopiedTeam = (record: ReturnType<typeof normalizeTeamRecord>) => {
    workspaceState.teams = [...workspaceState.teams.filter(item => item.id !== record.id), record]
  }

  const buildCopyResult = (
    agentIds: string[],
    teamIds: string[],
  ): ImportWorkspaceAgentBundleResult => ({
    departments: [],
    departmentCount: 0,
    detectedAgentCount: agentIds.length,
    importableAgentCount: agentIds.length,
    detectedTeamCount: teamIds.length,
    importableTeamCount: teamIds.length,
    createCount: agentIds.length + teamIds.length,
    updateCount: 0,
    skipCount: 0,
    failureCount: 0,
    uniqueSkillCount: 0,
    uniqueMcpCount: 0,
    agentCount: agentIds.length,
    teamCount: teamIds.length,
    skillCount: 0,
    mcpCount: 0,
    avatarCount: agentIds.length + teamIds.length,
    filteredFileCount: 0,
    agents: [],
    teams: [],
    skills: [],
    mcps: [],
    avatars: [],
    issues: [],
  })

  const copyAgentToScope = (agentId: string, projectId?: string) => {
    const source = workspaceState.agents.find(record => record.id === agentId)
    if (!source) {
      throw new WorkspaceApiError({
        message: 'agent not found',
        status: 404,
        requestId: 'req-agent-not-found',
        retryable: false,
        code: 'NOT_FOUND',
      })
    }

    const copiedId = projectId
      ? `agent-project-${normalizeCopySlug(source.name)}-copy`
      : `agent-workspace-${normalizeCopySlug(source.name)}-copy`
    const copied = normalizeAgentRecord(
      {
        workspaceId: workspaceState.workspace.id,
        projectId,
        scope: projectId ? 'project' : 'workspace',
        name: source.name,
        avatar: undefined,
        removeAvatar: false,
        personality: source.personality,
        tags: clone(source.tags),
        prompt: source.prompt,
        builtinToolKeys: clone(source.builtinToolKeys),
        skillIds: clone(source.skillIds),
        mcpServerNames: clone(source.mcpServerNames),
        description: source.description,
        status: source.status,
      },
      undefined,
      copiedId,
    )
    upsertCopiedAgent(copied)
    return copied
  }

  const copyTeamToScope = (teamId: string, projectId?: string) => {
    const source = workspaceState.teams.find(record => record.id === teamId)
    if (!source) {
      throw new WorkspaceApiError({
        message: 'team not found',
        status: 404,
        requestId: 'req-team-not-found',
        retryable: false,
        code: 'NOT_FOUND',
      })
    }

    const referencedAgentIds = Array.from(new Set([
      ...(source.leaderAgentId ? [source.leaderAgentId] : []),
      ...source.memberAgentIds,
    ]))
    const agentIdMap = new Map<string, string>()
    for (const referencedAgentId of referencedAgentIds) {
      const copiedAgent = copyAgentToScope(referencedAgentId, projectId)
      agentIdMap.set(referencedAgentId, copiedAgent.id)
    }

    const copiedId = projectId
      ? `team-project-${normalizeCopySlug(source.name)}-copy`
      : `team-workspace-${normalizeCopySlug(source.name)}-copy`
    const copied = normalizeTeamRecord(
      {
        workspaceId: workspaceState.workspace.id,
        projectId,
        scope: projectId ? 'project' : 'workspace',
        name: source.name,
        avatar: undefined,
        removeAvatar: false,
        personality: source.personality,
        tags: clone(source.tags),
        prompt: source.prompt,
        builtinToolKeys: clone(source.builtinToolKeys),
        skillIds: clone(source.skillIds),
        mcpServerNames: clone(source.mcpServerNames),
        leaderAgentId: source.leaderAgentId ? agentIdMap.get(source.leaderAgentId) : undefined,
        memberAgentIds: source.memberAgentIds
          .map(agentId => agentIdMap.get(agentId) ?? agentId)
          .filter((agentId, index, list) => list.indexOf(agentId) === index),
        description: source.description,
        status: source.status,
      },
      undefined,
      copiedId,
    )
    upsertCopiedTeam(copied)
    return {
      copied,
      copiedAgentIds: Array.from(agentIdMap.values()),
    }
  }

  const client: WorkspaceClient = {
    system: {
      async bootstrap() {
        return clone(workspaceState.systemBootstrap)
      },
    },
    enterpriseAuth: {
      async getStatus() {
        return {
          workspace: clone(workspaceState.workspace),
          bootstrapAdminRequired: !workspaceState.systemBootstrap.ownerReady,
          ownerReady: workspaceState.systemBootstrap.ownerReady,
        }
      },
      async login(request) {
        const user = workspaceState.users.find(record => record.username === request.username)
          ?? workspaceState.users.find(record => record.id === workspaceState.workspace.ownerUserId)
          ?? workspaceState.users[0]

        return {
          session: buildEnterpriseSession(user?.id ?? 'user-owner'),
          workspace: clone(workspaceState.workspace),
        }
      },
      async bootstrapAdmin(request) {
        return registerBootstrapAdmin(request)
      },
      async session() {
        if (connection.workspaceId === 'ws-local' && options.localSessionValid === false) {
          throw new WorkspaceApiError({
            message: 'session expired',
            status: 401,
            requestId: 'req-fixture-session-expired',
            retryable: false,
            code: 'SESSION_EXPIRED',
          })
        }

        const currentSession = activeSessionToken
          ? fixtureSessions.find(record => record.token === activeSessionToken)
          : findFixtureSession(getCurrentUserId())
        const nextUserId = currentSession?.userId ?? getCurrentUserId()
        workspaceState.currentUserId = nextUserId
        return buildEnterpriseSession(nextUserId, currentSession?.token)
      },
    },
    workspace: {
      async get() {
        return clone(workspaceState.workspace)
      },
      async getOverview() {
        return clone(workspaceState.overview)
      },
      async listPromotionRequests() {
        return clone(workspaceState.projectPromotionRequests)
      },
      async reviewPromotionRequest(requestId: string, input: ReviewProjectPromotionRequestInput) {
        const existing = workspaceState.projectPromotionRequests.find(record => record.id === requestId)
        if (!existing) {
          throw new WorkspaceApiError({
            message: 'promotion request not found',
            status: 404,
            requestId: 'req-promotion-request-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        const updated: ProjectPromotionRequest = {
          ...existing,
          status: input.approved ? 'approved' : 'rejected',
          reviewedByUserId: getCurrentUserId(),
          reviewComment: input.reviewComment?.trim() || undefined,
          reviewedAt: Date.now(),
          updatedAt: Date.now(),
        }
        workspaceState.projectPromotionRequests = workspaceState.projectPromotionRequests
          .map(record => record.id === requestId ? updated : record)

        if (input.approved && existing.assetType === 'resource') {
          const located = findResourceLocation(existing.assetId)
          if (located) {
            storeResourceRecord({
              ...located.record,
              scope: 'workspace',
              updatedAt: Date.now(),
            })
          }
        }

        return clone(updated)
      },
    },
    projects: {
      async list() {
        return clone(workspaceState.projects)
      },
      async create(input) {
        const project = createProjectRecord(workspaceState.workspace.id, input)
        workspaceState.projects = [...workspaceState.projects, project]
        ensureRuntimeProjectConfig(project.id)
        workspaceState.dashboards[project.id] = {
          project: clone(project),
          usedTokens: 0,
          metrics: [],
          overview: {
            memberCount: project.memberUserIds.length,
            activeUserCount: project.memberUserIds.length,
            agentCount: 0,
            teamCount: 0,
            conversationCount: 0,
            messageCount: 0,
            toolCallCount: 0,
            approvalCount: 0,
            resourceCount: 0,
            knowledgeCount: 0,
            toolCount: 0,
            tokenRecordCount: 0,
            totalTokens: 0,
            activityCount: 0,
            taskCount: 0,
            activeTaskCount: 0,
            attentionTaskCount: 0,
            scheduledTaskCount: 0,
          },
          trend: [],
          userStats: [],
          conversationInsights: [],
          toolRanking: [],
          resourceBreakdown: [],
          modelBreakdown: [],
          recentConversations: [],
          recentActivity: [],
          recentTasks: [],
        }
        workspaceState.projectResources[project.id] = []
        workspaceState.projectKnowledge[project.id] = []
        if (!workspaceState.projects.some(item => item.status === 'active' && item.id !== project.id)) {
          workspaceState.workspace = {
            ...workspaceState.workspace,
            defaultProjectId: project.id,
          }
        }
        syncWorkspaceProjectState(workspaceState)
        return clone(project)
      },
      async update(projectId, input: UpdateProjectRequest) {
        const current = workspaceState.projects.find(project => project.id === projectId)
        if (!current) {
          throw new WorkspaceApiError({
            message: 'project not found',
            status: 404,
            requestId: 'req-project-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        if (!input.name.trim()) {
          throw new WorkspaceApiError({
            message: 'project name is required',
            status: 400,
            requestId: 'req-project-name-required',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }

        if (current.status === 'active' && input.status === 'archived' && workspaceState.projects.filter(project => project.status === 'active').length <= 1) {
          throw new WorkspaceApiError({
            message: 'cannot archive the last active project',
            status: 400,
            requestId: 'req-project-last-active',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }

        const updated = updateProjectRecord(current, input)
        workspaceState.projects = workspaceState.projects.map(project => project.id === projectId ? updated : project)
        if (workspaceState.dashboards[projectId]) {
          workspaceState.dashboards[projectId] = {
            ...workspaceState.dashboards[projectId],
            project: clone(updated),
          }
        }
        if (updated.status === 'archived') {
          updateDefaultProjectIfNeeded(workspaceState, projectId)
        }
        syncWorkspaceProjectState(workspaceState)
        return clone(updated)
      },
      async getDashboard(projectId) {
        return clone(workspaceState.dashboards[projectId])
      },
      async listDeliverables(projectId) {
        findProjectRecord(projectId)
        return clone(
          workspaceState.deliverables
            .filter(record => record.projectId === projectId)
            .sort((left, right) => right.updatedAt - left.updatedAt),
        )
      },
      async listPromotionRequests(projectId) {
        findProjectRecord(projectId)
        return clone(
          workspaceState.projectPromotionRequests.filter(record => record.projectId === projectId),
        )
      },
      async createPromotionRequest(projectId, input: CreateProjectPromotionRequestInput) {
        const project = ensureProjectOwner(projectId)
        const record: ProjectPromotionRequest = {
          id: `promotion-${projectId}-${Date.now()}`,
          workspaceId: workspaceState.workspace.id,
          projectId,
          assetType: input.assetType,
          assetId: input.assetId,
          requestedByUserId: getCurrentUserId(),
          submittedByOwnerUserId: project.ownerUserId,
          requiredWorkspaceCapability: input.assetType === 'resource' ? 'resource.publish' : `${input.assetType}.publish`,
          status: 'pending',
          createdAt: Date.now(),
          updatedAt: Date.now(),
        }
        workspaceState.projectPromotionRequests = [record, ...workspaceState.projectPromotionRequests]
        return clone(record)
      },
    },
    tasks: {
      async listProject(projectId: string) {
        findProjectRecord(projectId)
        return clone(workspaceState.dashboards[projectId]?.recentTasks ?? [])
      },
      async createProject(projectId: string, input: CreateTaskRequest) {
        findProjectRecord(projectId)
        workspaceState.taskIdSequence += 1
        const now = Date.now()
        const detail: TaskDetail = {
          id: `task-${workspaceState.taskIdSequence}`,
          projectId,
          title: normalizeRequiredTaskText(input.title, 'task title', 'req-task-title-required'),
          goal: normalizeRequiredTaskText(input.goal, 'task goal', 'req-task-goal-required'),
          brief: normalizeRequiredTaskText(input.brief, 'task brief', 'req-task-brief-required'),
          defaultActorRef: normalizeRequiredTaskText(
            input.defaultActorRef,
            'default actor',
            'req-task-actor-required',
          ),
          status: 'ready',
          scheduleSpec: normalizeOptionalTaskText(input.scheduleSpec),
          nextRunAt: null,
          lastRunAt: null,
          latestResultSummary: null,
          latestFailureCategory: null,
          latestTransition: null,
          viewStatus: 'configured',
          attentionReasons: [],
          attentionUpdatedAt: null,
          activeTaskRunId: null,
          analyticsSummary: emptyTaskAnalytics(),
          contextBundle: normalizeTaskContextBundle(input.contextBundle),
          latestDeliverableRefs: [],
          latestArtifactRefs: [],
          runHistory: [],
          interventionHistory: [],
          activeRun: null,
          createdBy: getCurrentUserId(),
          updatedBy: null,
          createdAt: now,
          updatedAt: now,
        }
        storeTaskDetail(detail)
        return clone(detail)
      },
      async getDetail(projectId: string, taskId: string) {
        return ensureTaskDetail(projectId, taskId)
      },
      async updateProject(projectId: string, taskId: string, input: UpdateTaskRequest) {
        const detail = ensureTaskDetail(projectId, taskId)
        const now = Date.now()
        const updated: TaskDetail = {
          ...clone(detail),
          title: input.title === undefined
            ? detail.title
            : normalizeRequiredTaskText(input.title, 'task title', 'req-task-title-empty'),
          goal: input.goal === undefined
            ? detail.goal
            : normalizeRequiredTaskText(input.goal, 'task goal', 'req-task-goal-empty'),
          brief: input.brief === undefined
            ? detail.brief
            : normalizeRequiredTaskText(input.brief, 'task brief', 'req-task-brief-empty'),
          defaultActorRef: input.defaultActorRef === undefined
            ? detail.defaultActorRef
            : normalizeRequiredTaskText(
                input.defaultActorRef,
                'default actor',
                'req-task-actor-empty',
              ),
          scheduleSpec: input.scheduleSpec === undefined
            ? detail.scheduleSpec ?? null
            : normalizeOptionalTaskText(input.scheduleSpec),
          contextBundle: input.contextBundle === undefined
            ? clone(detail.contextBundle)
            : normalizeTaskContextBundle(input.contextBundle),
          updatedBy: getCurrentUserId(),
          updatedAt: now,
        }
        storeTaskDetail(updated)
        return clone(updated)
      },
      async launch(projectId: string, taskId: string, input: LaunchTaskRequest) {
        const detail = ensureTaskDetail(projectId, taskId)
        const actorRef = normalizeOptionalTaskText(input.actorRef) ?? detail.defaultActorRef
        const run = buildTaskRunSummary(detail, 'manual', actorRef)
        storeTaskDetail(applyTaskRunToDetail(detail, run))
        return clone(run)
      },
      async rerun(projectId: string, taskId: string, input: RerunTaskRequest) {
        const detail = ensureTaskDetail(projectId, taskId)
        const actorRef = normalizeOptionalTaskText(input.actorRef) ?? detail.defaultActorRef
        const run = buildTaskRunSummary(detail, 'rerun', actorRef)
        storeTaskDetail(applyTaskRunToDetail(detail, run))
        return clone(run)
      },
      async listRuns(projectId: string, taskId: string) {
        const key = taskKey(projectId, taskId)
        ensureTaskDetail(projectId, taskId)
        return clone(taskRunsByKey.get(key) ?? [])
      },
      async createIntervention(projectId: string, taskId: string, input: CreateTaskInterventionRequest) {
        const detail = ensureTaskDetail(projectId, taskId)
        const type = normalizeRequiredTaskText(input.type, 'task intervention type', 'req-task-intervention-type')
        workspaceState.taskInterventionIdSequence += 1
        const createdAt = Date.now()
        const appliesRunState = type === 'approve' || type === 'reject' || type === 'resume'
        const intervention: TaskInterventionRecord = {
          id: `task-intervention-${workspaceState.taskInterventionIdSequence}`,
          taskId: detail.id,
          taskRunId: normalizeOptionalTaskText(input.taskRunId),
          type,
          payload: clone(input.payload ?? {}),
          createdBy: getCurrentUserId(),
          createdAt,
          appliedToSessionId: null,
          status: appliesRunState ? 'applied' : 'accepted',
        }
        const payloadBrief = intervention.payload && typeof intervention.payload === 'object'
          ? intervention.payload.brief
          : undefined
        const payloadActorRef = intervention.payload && typeof intervention.payload === 'object'
          ? normalizeOptionalTaskText(
              typeof intervention.payload.actorRef === 'string' ? intervention.payload.actorRef : null,
            )
          : null
        const nextActorRef = type === 'change_actor' ? payloadActorRef : null
        const targetRunId = intervention.taskRunId ?? detail.activeTaskRunId ?? detail.activeRun?.id ?? null
        const transitionSummary = type === 'approve'
          ? 'Approval received. Task run resumed.'
          : type === 'reject'
            ? 'Approval rejected. Task run is waiting for updated guidance.'
            : type === 'resume'
              ? 'Task run resumed after updated guidance.'
              : `Task intervention recorded: ${type}.`
        const latestResultSummary = type === 'approve'
          ? 'Approval received. Continuing the active run.'
          : type === 'reject'
            ? 'Approval rejected. Waiting for updated guidance.'
            : type === 'resume'
              ? 'Updated guidance received. Continuing the active run.'
              : detail.latestResultSummary
        const runTransition = (runId: string | null, runtimeRunId: string | null) => ({
          kind: 'intervened' as const,
          summary: transitionSummary,
          at: createdAt,
          runId: runtimeRunId ?? runId,
        })
        const updateTargetRun = (run: TaskRunSummary) => {
          if (run.id !== targetRunId) {
            return clone(run)
          }

          if (nextActorRef) {
            return {
              ...clone(run),
              actorRef: nextActorRef,
            }
          }

          if (type === 'approve' || type === 'resume') {
            return {
              ...clone(run),
              status: 'running',
              resultSummary: latestResultSummary,
              pendingApprovalId: null,
              failureCategory: null,
              failureSummary: null,
              viewStatus: 'healthy',
              attentionReasons: [],
              attentionUpdatedAt: null,
              latestTransition: runTransition(run.id, run.runtimeRunId ?? null),
            }
          }

          if (type === 'reject') {
            return {
              ...clone(run),
              status: 'waiting_input',
              resultSummary: latestResultSummary,
              pendingApprovalId: null,
              failureCategory: null,
              failureSummary: null,
              viewStatus: 'attention',
              attentionReasons: ['waiting_input'],
              attentionUpdatedAt: createdAt,
              latestTransition: runTransition(run.id, run.runtimeRunId ?? null),
            }
          }

          return clone(run)
        }
        const nextActiveRun = detail.activeRun
          ? updateTargetRun(detail.activeRun)
          : null
        const nextRunHistory = detail.runHistory.map(run => updateTargetRun(run))
        const nextAttentionReasons = type === 'takeover'
          ? ['takeover_recommended']
          : type === 'reject'
            ? ['waiting_input']
            : type === 'approve' || type === 'resume'
              ? []
              : detail.attentionReasons
        const nextStatus = type === 'approve' || type === 'resume'
          ? 'running'
          : type === 'reject'
            ? 'attention'
            : detail.status
        const nextViewStatus = nextAttentionReasons.length > 0
          ? 'attention'
          : type === 'approve' || type === 'resume'
            ? 'healthy'
            : detail.viewStatus
        const updated: TaskDetail = {
          ...clone(detail),
          brief: type === 'edit_brief' && typeof payloadBrief === 'string' && payloadBrief.trim()
            ? payloadBrief.trim()
            : detail.brief,
          defaultActorRef: nextActorRef
            ? nextActorRef
            : detail.defaultActorRef,
          status: nextStatus,
          latestResultSummary,
          latestFailureCategory: type === 'approve' || type === 'reject' || type === 'resume'
            ? null
            : detail.latestFailureCategory,
          latestTransition: {
            kind: 'intervened',
            summary: transitionSummary,
            at: createdAt,
            runId: targetRunId,
          },
          viewStatus: nextViewStatus,
          attentionReasons: clone(nextAttentionReasons),
          attentionUpdatedAt: nextAttentionReasons.length > 0 ? createdAt : null,
          runHistory: nextRunHistory,
          interventionHistory: [
            clone(intervention),
            ...detail.interventionHistory.filter(record => record.id !== intervention.id),
          ],
          activeRun: nextActiveRun ? clone(nextActiveRun) : null,
          updatedBy: getCurrentUserId(),
          updatedAt: createdAt,
        }
        storeTaskDetail(updated)
        return clone(intervention)
      },
    },
    resources: {
      async listWorkspace() {
        return clone(listWorkspaceResources())
      },
      async listProject(projectId) {
        return clone(workspaceState.projectResources[projectId] ?? [])
      },
      async createWorkspace(input: CreateWorkspaceResourceInput) {
        const record = createResourceRecord({
          kind: input.kind,
          name: input.name.trim(),
          location: input.location,
          scope: input.scope ?? 'workspace',
          visibility: input.visibility ?? 'public',
          sourceArtifactId: input.sourceArtifactId,
          tags: input.tags,
          origin: input.kind === 'url' ? 'generated' : 'source',
        })
        if (record.previewKind === 'url' && record.location) {
          workspaceState.resourceContents[record.id] = buildResourceContent(record.id, {
            previewKind: 'url',
            externalUrl: record.location,
          })
        }
        storeResourceRecord(record)
        return clone(record)
      },
      async createProject(projectId: string, input: CreateWorkspaceResourceInput) {
        const record = createResourceRecord({
          projectId,
          kind: input.kind,
          name: input.name.trim(),
          location: input.location,
          scope: input.scope ?? 'project',
          visibility: input.visibility ?? 'public',
          sourceArtifactId: input.sourceArtifactId,
          tags: input.tags,
          origin: input.kind === 'url' ? 'generated' : 'source',
        })
        if (record.previewKind === 'url' && record.location) {
          workspaceState.resourceContents[record.id] = buildResourceContent(record.id, {
            previewKind: 'url',
            externalUrl: record.location,
          })
        }
        storeResourceRecord(record)
        return clone(record)
      },
      async createProjectFolder(projectId: string, input: CreateWorkspaceResourceFolderInput) {
        const record = importResourceRecord({
          name: 'uploaded-folder',
          rootDirName: input.files[0]?.relativePath.split('/')[0] || 'uploaded-folder',
          scope: 'project',
          visibility: 'public',
          files: input.files,
        }, projectId)
        storeResourceRecord(record)
        return clone([record])
      },
      async importWorkspace(input: WorkspaceResourceImportInput) {
        const record = importResourceRecord(input)
        storeResourceRecord(record)
        return clone(record)
      },
      async importProject(projectId: string, input: WorkspaceResourceImportInput) {
        const record = importResourceRecord(input, projectId)
        storeResourceRecord(record)
        return clone(record)
      },
      async getDetail(resourceId: string) {
        const located = findResourceLocation(resourceId)
        if (!located) {
          throw new WorkspaceApiError({
            message: 'resource not found',
            status: 404,
            requestId: 'req-resource-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        return clone(located.record)
      },
      async getContent(resourceId: string) {
        const located = findResourceLocation(resourceId)
        if (!located) {
          throw new WorkspaceApiError({
            message: 'resource not found',
            status: 404,
            requestId: 'req-resource-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        const content = workspaceState.resourceContents[resourceId]
          ?? (located.record.previewKind === 'url' && located.record.location
            ? buildResourceContent(resourceId, {
                previewKind: 'url',
                externalUrl: located.record.location,
              })
            : {
                resourceId,
                previewKind: located.record.previewKind,
                fileName: located.record.name,
                contentType: located.record.contentType,
                byteSize: located.record.byteSize,
              })
        return clone(content)
      },
      async listChildren(resourceId: string) {
        return clone(workspaceState.resourceChildren[resourceId] ?? [])
      },
      async promote(resourceId: string, input: PromoteWorkspaceResourceInput) {
        const located = findResourceLocation(resourceId)
        if (!located) {
          throw new WorkspaceApiError({
            message: 'resource not found',
            status: 404,
            requestId: 'req-resource-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        const updated: WorkspaceResourceRecord = {
          ...located.record,
          scope: input.scope,
          updatedAt: Date.now(),
        }
        storeResourceRecord(updated)
        return clone(updated)
      },
      async updateWorkspace(resourceId: string, input: UpdateWorkspaceResourceInput) {
        const located = findResourceLocation(resourceId)
        if (!located || located.container !== 'workspace') {
          throw new WorkspaceApiError({
            message: 'workspace resource not found',
            status: 404,
            requestId: 'req-workspace-resource-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        const updated: WorkspaceResourceRecord = {
          ...located.record,
          name: input.name ?? located.record.name,
          location: input.location ?? located.record.location,
          visibility: input.visibility ?? located.record.visibility,
          status: input.status ?? located.record.status,
          tags: input.tags ?? located.record.tags,
          updatedAt: Date.now(),
        }
        storeResourceRecord(updated)
        return clone(updated)
      },
      async updateProject(projectId: string, resourceId: string, input: UpdateWorkspaceResourceInput) {
        const located = findResourceLocation(resourceId)
        if (!located || located.container !== 'project' || located.projectId !== projectId) {
          throw new WorkspaceApiError({
            message: 'project resource not found',
            status: 404,
            requestId: 'req-project-resource-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        const updated: WorkspaceResourceRecord = {
          ...located.record,
          name: input.name ?? located.record.name,
          location: input.location ?? located.record.location,
          visibility: input.visibility ?? located.record.visibility,
          status: input.status ?? located.record.status,
          tags: input.tags ?? located.record.tags,
          updatedAt: Date.now(),
        }
        storeResourceRecord(updated)
        return clone(updated)
      },
      async deleteWorkspace(resourceId: string) {
        removeResourceRecord(resourceId)
      },
      async deleteProject(projectId: string, resourceId: string) {
        const located = findResourceLocation(resourceId)
        if (located?.container === 'project' && located.projectId === projectId) {
          removeResourceRecord(resourceId)
        }
      },
    },
    filesystem: {
      async listDirectories(path?: string) {
        const key = path?.trim() || ''
        const payload = workspaceState.remoteDirectories[key]
          ?? workspaceState.remoteDirectories['']
          ?? {
            currentPath: key || '/remote',
            entries: [],
          } satisfies WorkspaceDirectoryBrowserResponse
        return clone(payload)
      },
    },
    deliverables: {
      async listWorkspace() {
        return clone(workspaceState.deliverables)
      },
    },
    inbox: {
      async list() {
        return clone(workspaceState.inboxItems)
      },
    },
    knowledge: {
      async listWorkspace() {
        return clone(listWorkspaceKnowledge())
      },
      async listProject(projectId) {
        findProjectRecord(projectId)
        return clone(listProjectKnowledge(projectId))
      },
    },
    pet: {
      async getDashboard() {
        const hasHomePetSession = Boolean(workspaceState.workspacePetBinding?.sessionId)
        return clone({
          petId: workspaceState.petProfile.id,
          workspaceId: workspaceState.workspace.id,
          ownerUserId: workspaceState.petProfile.ownerUserId,
          species: workspaceState.petProfile.species,
          mood: workspaceState.petProfile.mood,
          memoryCount: hasHomePetSession ? 1 : 0,
          knowledgeCount: workspaceState.workspaceKnowledge.length,
          resourceCount: workspaceState.workspaceResources.length,
          reminderCount: workspaceState.inboxItems.length,
          activeConversationCount: workspaceState.workspacePetBinding ? 1 : 0,
          lastInteractionAt: workspaceState.workspacePetPresence.lastInteractionAt,
        })
      },
      async getSnapshot(projectId) {
        return clone(createPetSnapshot(workspaceState, projectId))
      },
      async savePresence(input: SavePetPresenceInput, projectId) {
        const current = projectId
          ? workspaceState.projectPetPresences[projectId] ?? createPetPresenceState(input.petId || workspaceState.petProfile.id)
          : workspaceState.workspacePetPresence
        const next = {
          ...current,
          petId: input.petId || current.petId,
          isVisible: input.isVisible ?? current.isVisible,
          chatOpen: input.chatOpen ?? current.chatOpen,
          motionState: input.motionState ?? current.motionState,
          unreadCount: input.unreadCount ?? current.unreadCount,
          lastInteractionAt: input.lastInteractionAt ?? current.lastInteractionAt,
          position: input.position ?? current.position,
        }
        if (projectId) {
          workspaceState.projectPetPresences[projectId] = next
        } else {
          workspaceState.workspacePetPresence = next
        }
        return clone(next)
      },
      async bindConversation(input: BindPetConversationInput, projectId) {
        const binding: PetConversationBinding = {
          petId: input.petId,
          workspaceId: workspaceState.workspace.id,
          conversationId: input.conversationId,
          sessionId: input.sessionId,
          ownerUserId: workspaceState.petProfile.ownerUserId,
          contextScope: projectId ? 'project' : 'home',
          projectId,
          updatedAt: Date.now(),
        }
        if (projectId) {
          workspaceState.projectPetBindings[projectId] = binding
        } else {
          workspaceState.workspacePetBinding = binding
        }
        return clone(binding)
      },
    },
    agents: {
      async list() {
        return clone(workspaceState.agents)
      },
      async copyToWorkspace(agentId) {
        const copied = copyAgentToScope(agentId)
        return clone(buildCopyResult([copied.id], []))
      },
      async copyToProject(projectId, agentId) {
        const copied = copyAgentToScope(agentId, projectId)
        return clone(buildCopyResult([copied.id], []))
      },
      async previewImportBundle(input, projectId) {
        return clone(buildAgentBundlePreview(input, projectId))
      },
      async importBundle(input, projectId) {
        const preview = buildAgentBundlePreview(input, projectId)
        const importedAgentId = projectId ? 'agent-imported-project' : 'agent-imported-workspace'
        const importedTeamId = projectId ? 'team-imported-project' : 'team-imported-workspace'
        const importedAgent = normalizeAgentRecord(
          {
            workspaceId: workspaceState.workspace.id,
            projectId,
            scope: projectId ? 'project' : 'workspace',
            name: projectId ? 'Imported Project Agent' : 'Imported Workspace Agent',
            builtinToolKeys: ['bash'],
            skillIds: [],
            mcpServerNames: [],
            description: 'Imported from an agent bundle.',
            personality: 'Imported fixture persona',
            tags: ['imported'],
            prompt: 'Imported fixture prompt',
            status: 'active',
          },
          undefined,
          importedAgentId,
        )
        const importedTeam = normalizeTeamRecord(
          {
            workspaceId: workspaceState.workspace.id,
            projectId,
            scope: projectId ? 'project' : 'workspace',
            name: projectId ? 'Imported Project Team' : 'Imported Workspace Team',
            builtinToolKeys: ['bash'],
            skillIds: [],
            mcpServerNames: [],
            leaderAgentId: importedAgentId,
            memberAgentIds: [importedAgentId],
            description: 'Imported from an agent bundle.',
            personality: 'Imported fixture team',
            tags: ['imported'],
            prompt: 'Imported fixture prompt',
            status: 'active',
          },
          undefined,
          importedTeamId,
        )

        workspaceState.agents = [...workspaceState.agents, importedAgent]
        workspaceState.teams = [...workspaceState.teams, importedTeam]
        return clone(preview) as ImportWorkspaceAgentBundleResult
      },
      async exportBundle(input) {
        return clone(buildAgentBundleExport(input))
      },
      async create(input) {
        const id = `agent-${Date.now()}`
        const record = normalizeAgentRecord(input as UpsertAgentInput, undefined, id)
        workspaceState.agents = [...workspaceState.agents, record]
        return clone(record)
      },
      async update(agentId, input) {
        const current = workspaceState.agents.find(item => item.id === agentId)
        const record = normalizeAgentRecord(input as UpsertAgentInput, current, agentId)
        workspaceState.agents = workspaceState.agents.map(item => item.id === agentId ? record : item)
        return clone(record)
      },
      async delete(agentId) {
        workspaceState.agents = workspaceState.agents.filter(item => item.id !== agentId)
        workspaceState.projectAgentLinks = Object.fromEntries(
          Object.entries(workspaceState.projectAgentLinks).map(([projectId, links]) => [
            projectId,
            links.filter(link => link.agentId !== agentId),
          ]),
        )
      },
      async listProjectLinks(projectId) {
        return clone(workspaceState.projectAgentLinks[projectId] ?? [])
      },
      async linkProject(input) {
        const created: ProjectAgentLinkRecord = {
          workspaceId: workspaceState.workspace.id,
          projectId: input.projectId,
          agentId: input.agentId,
          linkedAt: Date.now(),
        }
        workspaceState.projectAgentLinks[input.projectId] = [
          ...(workspaceState.projectAgentLinks[input.projectId] ?? []).filter(link => link.agentId !== input.agentId),
          created,
        ]
        return clone(created)
      },
      async unlinkProject(projectId, agentId) {
        workspaceState.projectAgentLinks[projectId] = (workspaceState.projectAgentLinks[projectId] ?? [])
          .filter(link => link.agentId !== agentId)
      },
    },
    teams: {
      async list() {
        return clone(workspaceState.teams)
      },
      async copyToWorkspace(teamId) {
        const copied = copyTeamToScope(teamId)
        return clone(buildCopyResult(copied.copiedAgentIds, [copied.copied.id]))
      },
      async copyToProject(projectId, teamId) {
        const copied = copyTeamToScope(teamId, projectId)
        return clone(buildCopyResult(copied.copiedAgentIds, [copied.copied.id]))
      },
      async create(input) {
        const id = `team-${Date.now()}`
        const record = normalizeTeamRecord(input as UpsertTeamInput, undefined, id)
        workspaceState.teams = [...workspaceState.teams, record]
        return clone(record)
      },
      async update(teamId, input) {
        const current = workspaceState.teams.find(item => item.id === teamId)
        const record = normalizeTeamRecord(input as UpsertTeamInput, current, teamId)
        workspaceState.teams = workspaceState.teams.map(item => item.id === teamId ? record : item)
        return clone(record)
      },
      async delete(teamId) {
        workspaceState.teams = workspaceState.teams.filter(item => item.id !== teamId)
        workspaceState.projectTeamLinks = Object.fromEntries(
          Object.entries(workspaceState.projectTeamLinks).map(([projectId, links]) => [
            projectId,
            links.filter(link => link.teamId !== teamId),
          ]),
        )
      },
      async listProjectLinks(projectId) {
        return clone(workspaceState.projectTeamLinks[projectId] ?? [])
      },
      async linkProject(input) {
        const created: ProjectTeamLinkRecord = {
          workspaceId: workspaceState.workspace.id,
          projectId: input.projectId,
          teamId: input.teamId,
          linkedAt: Date.now(),
        }
        workspaceState.projectTeamLinks[input.projectId] = [
          ...(workspaceState.projectTeamLinks[input.projectId] ?? []).filter(link => link.teamId !== input.teamId),
          created,
        ]
        return clone(created)
      },
      async unlinkProject(projectId, teamId) {
        workspaceState.projectTeamLinks[projectId] = (workspaceState.projectTeamLinks[projectId] ?? [])
          .filter(link => link.teamId !== teamId)
      },
    },
    catalog: {
      async getSnapshot() {
        return clone(workspaceState.catalog)
      },
      async getManagementProjection(): Promise<CapabilityManagementProjection> {
        return clone(workspaceState.managementProjection)
      },
      async setAssetDisabled(patch: CapabilityAssetDisablePatch) {
        const current = findToolCatalogEntry(patch.sourceKey)
        if (!current) {
          throw new WorkspaceApiError({
            message: 'tool source not found',
            status: 404,
            requestId: 'req-tool-source-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        replaceToolCatalogEntry({
          ...current,
          disabled: patch.disabled,
        })
        syncManagedToolConfig()
        return clone(workspaceState.managementProjection)
      },
      async getSkill(skillId) {
        return clone(ensureSkillDocument(skillId))
      },
      async getSkillTree(skillId) {
        const document = ensureSkillDocument(skillId)
        return {
          skillId,
          sourceKey: document.sourceKey,
          displayPath: document.rootPath,
          rootPath: document.rootPath,
          tree: clone(document.tree),
        }
      },
      async getSkillFile(skillId: string, relativePath: string) {
        const file = ensureSkillFiles(skillId)[relativePath]
        if (!file) {
          throw new WorkspaceApiError({
            message: 'skill file not found',
            status: 404,
            requestId: 'req-skill-file-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        return clone(file)
      },
      async createSkill(input: CreateWorkspaceSkillInput) {
        const document = createManagedSkill(input.slug, {
          'SKILL.md': createSkillFileDocument(
            `skill-workspace-${input.slug.trim()}`,
            `skill:data/skills/${input.slug.trim()}/SKILL.md`,
            `data/skills/${input.slug.trim()}`,
            'SKILL.md',
            {
              content: input.content,
            },
          ),
        })
        return clone(document)
      },
      async updateSkill(skillId: string, input: UpdateWorkspaceSkillInput) {
        const current = ensureWorkspaceOwnedSkillDocument(skillId)
        const currentFiles = ensureSkillFiles(skillId)
        const nextFiles = cloneSkillFiles(currentFiles)
        nextFiles['SKILL.md'] = {
          ...nextFiles['SKILL.md'],
          content: input.content,
          byteSize: input.content.length,
        }
        const updated = createSkillDocument(
          current.id,
          current.sourceKey,
          skillNameFromContent(input.content, current.name || skillSlugFromRelativePath(current.relativePath) || 'skill'),
          skillDescriptionFromContent(input.content, current.description || 'Workspace skill'),
          current.displayPath,
          true,
          nextFiles,
          current.relativePath,
        )
        replaceSkillState(updated, nextFiles)
        return clone(updated)
      },
      async updateSkillFile(skillId: string, relativePath: string, input: UpdateWorkspaceSkillFileInput) {
        const current = ensureWorkspaceOwnedSkillDocument(skillId)
        const currentFiles = ensureSkillFiles(skillId)
        const file = currentFiles[relativePath]
        if (!file) {
          throw new WorkspaceApiError({
            message: 'skill file not found',
            status: 404,
            requestId: 'req-skill-file-update-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        if (!file.isText || file.readonly) {
          throw new WorkspaceApiError({
            message: 'skill file is read-only',
            status: 400,
            requestId: 'req-skill-file-readonly',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }

        const nextFiles = cloneSkillFiles(currentFiles)
        nextFiles[relativePath] = {
          ...nextFiles[relativePath],
          content: input.content,
          byteSize: input.content.length,
        }

        const updated = createSkillDocument(
          current.id,
          current.sourceKey,
          skillNameFromContent(nextFiles['SKILL.md']?.content ?? current.content, current.name || skillSlugFromRelativePath(current.relativePath) || 'skill'),
          skillDescriptionFromContent(nextFiles['SKILL.md']?.content ?? current.content, current.description || 'Workspace skill'),
          current.displayPath,
          true,
          nextFiles,
          current.relativePath,
        )
        replaceSkillState(updated, nextFiles)
        return clone(nextFiles[relativePath])
      },
      async importSkillArchive(input) {
        const skillId = `skill-workspace-${input.slug.trim()}`
        const sourceKey = `skill:data/skills/${input.slug.trim()}/SKILL.md`
        const rootPath = `data/skills/${input.slug.trim()}`
        const files = createImportedSkillFiles(skillId, sourceKey, rootPath, input.slug.trim())
        return clone(createManagedSkill(input.slug, files))
      },
      async importSkillFolder(input) {
        const skillId = `skill-workspace-${input.slug.trim()}`
        const sourceKey = `skill:data/skills/${input.slug.trim()}/SKILL.md`
        const rootPath = `data/skills/${input.slug.trim()}`
        const files = createImportedSkillFiles(skillId, sourceKey, rootPath, input.slug.trim(), input.files)
        return clone(createManagedSkill(input.slug, files))
      },
      async copySkillToManaged(skillId: string, input: CopyWorkspaceSkillToManagedInput) {
        const sourceDocument = ensureSkillDocument(skillId)
        const sourceFiles = ensureSkillFiles(skillId)
        const nextSlug = input.slug.trim()
        const nextSkillId = `skill-workspace-${input.slug.trim()}`
        const nextSourceKey = `skill:data/skills/${input.slug.trim()}/SKILL.md`
        const nextRootPath = `data/skills/${input.slug.trim()}`
        const copiedFiles = Object.fromEntries(
          Object.entries(sourceFiles).map(([path, file]) => {
            const cloned = clone(file)
            return [path, {
              ...cloned,
              skillId: nextSkillId,
              sourceKey: nextSourceKey,
              displayPath: `${nextRootPath}/${path}`,
              content: path === 'SKILL.md' && cloned.content
                ? normalizeSkillFrontmatterName(cloned.content, nextSlug)
                : cloned.content,
              readonly: false,
            } satisfies WorkspaceSkillFileDocument]
          }),
        )
        const document = createManagedSkill(input.slug, copiedFiles)
        if (copiedFiles['SKILL.md']?.content) {
          const nextFiles = ensureSkillFiles(document.id)
          nextFiles['SKILL.md'] = {
            ...nextFiles['SKILL.md'],
            content: copiedFiles['SKILL.md'].content,
            byteSize: copiedFiles['SKILL.md'].byteSize,
          }
          const updated = createSkillDocument(
            document.id,
            document.sourceKey,
            skillNameFromContent(nextFiles['SKILL.md'].content ?? '', sourceDocument.name),
            skillDescriptionFromContent(nextFiles['SKILL.md'].content ?? '', sourceDocument.description),
            document.displayPath,
            true,
            nextFiles,
            document.relativePath,
          )
          replaceSkillState(updated, nextFiles)
          return clone(updated)
        }
        return clone(document)
      },
      async deleteSkill(skillId: string) {
        ensureWorkspaceOwnedSkillDocument(skillId)

        const nextDocuments = { ...workspaceState.skillDocuments }
        delete nextDocuments[skillId]
        const nextFiles = { ...workspaceState.skillFiles }
        delete nextFiles[skillId]
        workspaceState.skillDocuments = nextDocuments
        workspaceState.skillFiles = nextFiles
        workspaceState.toolCatalog.entries = workspaceState.toolCatalog.entries.filter(entry => entry.id !== skillId)
        syncManagedToolConfig()
      },
      async getMcpServer(serverName: string) {
        const document = workspaceState.mcpDocuments[serverName]
        if (!document) {
          throw new WorkspaceApiError({
            message: 'MCP server not found',
            status: 404,
            requestId: 'req-mcp-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        return clone(document)
      },
      async copyMcpServerToManaged(serverName: string) {
        const current = workspaceState.mcpDocuments[serverName]
        if (!current || current.scope !== 'builtin') {
          throw new WorkspaceApiError({
            message: 'builtin MCP server not found',
            status: 404,
            requestId: 'req-builtin-mcp-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        const document: WorkspaceMcpServerDocument = {
          ...current,
          displayPath: 'config/runtime/workspace.json',
          scope: 'workspace',
        }
        workspaceState.mcpDocuments = {
          ...workspaceState.mcpDocuments,
          [serverName]: document,
        }
        replaceToolCatalogEntry(createMcpCatalogEntry(workspaceState.workspace.id, document, false, {
          ownerScope: 'workspace',
          ownerId: workspaceState.workspace.id,
          ownerLabel: workspaceState.workspace.name,
          consumers: findToolCatalogEntry(current.sourceKey)?.consumers,
          toolNames: findToolCatalogEntry(current.sourceKey)?.kind === 'mcp'
            ? findToolCatalogEntry(current.sourceKey)?.toolNames
            : [],
          description: 'Configured MCP server.',
        }))
        syncManagedToolConfig()
        return clone(document)
      },
      async createMcpServer(input) {
        const serverName = input.serverName.trim()
        if (!serverName) {
          throw new WorkspaceApiError({
            message: 'server name is required',
            status: 400,
            requestId: 'req-mcp-name-required',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }
        if (workspaceState.mcpDocuments[serverName]) {
          throw new WorkspaceApiError({
            message: 'MCP server already exists',
            status: 409,
            requestId: 'req-mcp-exists',
            retryable: false,
            code: 'CONFLICT',
          })
        }

        const document: WorkspaceMcpServerDocument = {
          serverName,
          sourceKey: `mcp:${serverName}`,
          displayPath: 'config/runtime/workspace.json',
          scope: 'workspace',
          config: clone(input.config),
        }
        workspaceState.mcpDocuments = {
          ...workspaceState.mcpDocuments,
          [serverName]: document,
        }
        replaceToolCatalogEntry(createMcpCatalogEntry(workspaceState.workspace.id, document))
        syncManagedToolConfig()
        return clone(document)
      },
      async updateMcpServer(serverName: string, input) {
        const current = workspaceState.mcpDocuments[serverName]
        if (!current) {
          throw new WorkspaceApiError({
            message: 'MCP server not found',
            status: 404,
            requestId: 'req-mcp-update-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        if (input.serverName.trim() !== serverName) {
          throw new WorkspaceApiError({
            message: 'renaming MCP servers is not supported',
            status: 400,
            requestId: 'req-mcp-rename-unsupported',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }

        const updated: WorkspaceMcpServerDocument = {
          ...current,
          config: clone(input.config),
        }
        workspaceState.mcpDocuments = {
          ...workspaceState.mcpDocuments,
          [serverName]: updated,
        }
        const existingEntry = findToolCatalogEntry(current.sourceKey)
        replaceToolCatalogEntry(createMcpCatalogEntry(workspaceState.workspace.id, updated, existingEntry?.disabled ?? false))
        syncManagedToolConfig()
        return clone(updated)
      },
      async deleteMcpServer(serverName: string) {
        if (!workspaceState.mcpDocuments[serverName]) {
          throw new WorkspaceApiError({
            message: 'MCP server not found',
            status: 404,
            requestId: 'req-mcp-delete-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        const nextDocuments = { ...workspaceState.mcpDocuments }
        delete nextDocuments[serverName]
        workspaceState.mcpDocuments = nextDocuments
        workspaceState.toolCatalog.entries = workspaceState.toolCatalog.entries.filter(entry => !(entry.kind === 'mcp' && entry.serverName === serverName))
        syncManagedToolConfig()
      },
      async listModels() {
        return clone(workspaceState.catalog.models)
      },
      async listProviderCredentials() {
        return clone(workspaceState.catalog.credentialBindings)
      },
      async listTools() {
        return clone(workspaceState.tools)
      },
      async createTool(record) {
        workspaceState.tools = [...workspaceState.tools, clone(record)]
        return clone(record)
      },
      async updateTool(toolId, record) {
        workspaceState.tools = workspaceState.tools.map(item => item.id === toolId ? clone(record) : item)
        return clone(record)
      },
      async deleteTool(toolId) {
        workspaceState.tools = workspaceState.tools.filter(item => item.id !== toolId)
      },
    },
    profile: {
      async updateCurrentUserProfile(input: UpdateCurrentUserProfileRequest) {
        const currentUserId = workspaceState.currentUserId
        const currentUser = workspaceState.users.find(user => user.id === currentUserId)
        if (!currentUser) {
          throw new WorkspaceApiError({
            message: 'Current user not found.',
            status: 404,
            requestId: 'req-user-profile-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        const updated: UserRecordSummary = {
          ...currentUser,
          username: input.username.trim(),
          displayName: input.displayName.trim(),
          avatar: input.removeAvatar
            ? undefined
            : input.avatar
              ? `data:${input.avatar.contentType};base64,${input.avatar.dataBase64}`
              : currentUser.avatar,
        }
        workspaceState.users = workspaceState.users.map(user => user.id === currentUserId ? clone(updated) : user)
        accessUsers = accessUsers.map(user => user.id === currentUserId
          ? {
              ...user,
              username: updated.username,
              displayName: updated.displayName,
              status: updated.status,
              passwordState: updated.passwordState,
            }
          : user)
        return clone(updated)
      },
      async changeCurrentUserPassword(input: ChangeCurrentUserPasswordRequest): Promise<ChangeCurrentUserPasswordResponse> {
        const currentUserId = workspaceState.currentUserId
        const currentPassword = workspaceState.userPasswords[currentUserId]
        if (!currentPassword) {
          throw new WorkspaceApiError({
            message: 'Current user not found.',
            status: 404,
            requestId: 'req-user-password-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        if (input.currentPassword !== currentPassword) {
          throw new WorkspaceApiError({
            message: 'Current password is incorrect.',
            status: 400,
            requestId: 'req-user-password-current-invalid',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }
        if (input.newPassword.length < 8) {
          throw new WorkspaceApiError({
            message: 'New password must be at least 8 characters.',
            status: 400,
            requestId: 'req-user-password-too-short',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }
        if (input.newPassword !== input.confirmPassword) {
          throw new WorkspaceApiError({
            message: 'Password confirmation does not match.',
            status: 400,
            requestId: 'req-user-password-confirm-invalid',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }

        workspaceState.userPasswords = {
          ...workspaceState.userPasswords,
          [currentUserId]: input.newPassword,
        }
        const currentUser = workspaceState.users.find(user => user.id === currentUserId)
        if (currentUser) {
          const updated: UserRecordSummary = {
            ...currentUser,
            passwordState: 'set',
          }
          workspaceState.users = workspaceState.users.map(user => user.id === currentUserId ? clone(updated) : user)
          accessUsers = accessUsers.map(user => user.id === currentUserId
            ? {
                ...user,
                passwordState: 'set',
              }
            : user)
        }
        return {
          success: true,
          passwordState: 'set',
        }
      },
    },
    accessControl: {
      async getCurrentAuthorization() {
        return buildAuthorizationSnapshot()
      },
      async getAccessExperience() {
        return buildAccessExperience()
      },
      async listMembers() {
        return buildAccessMembers()
      },
      async listAudit(query = {}) {
        const filtered = auditRecords.filter((record) => {
          if (query.actorId && record.actorId !== query.actorId) {
            return false
          }
          if (query.action && record.action !== query.action) {
            return false
          }
          if (query.resourceType) {
            const resourceType = record.resource.split(':', 1)[0]
            if (resourceType !== query.resourceType) {
              return false
            }
          }
          if (query.outcome && record.outcome !== query.outcome) {
            return false
          }
          if (typeof query.from === 'number' && record.createdAt < query.from) {
            return false
          }
          if (typeof query.to === 'number' && record.createdAt > query.to) {
            return false
          }
          return true
        })
        const offset = Number.parseInt(query.cursor ?? '0', 10)
        const start = Number.isFinite(offset) && offset > 0 ? offset : 0
        const pageSize = 20
        const items = filtered.slice(start, start + pageSize)
        const nextCursor = start + pageSize < filtered.length ? String(start + pageSize) : undefined
        return {
          items: clone(items),
          nextCursor,
        }
      },
      async listSessions() {
        return buildAccessSessionRecords()
      },
      async revokeCurrentSession() {
        const currentSession = fixtureSessions.find(record => record.token === activeSessionToken)
        if (!currentSession) {
          throw new WorkspaceApiError({
            message: 'session not found',
            status: 404,
            requestId: 'req-fixture-current-session-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        currentSession.status = 'revoked'
        appendAudit(
          'access.sessions.revoke-current',
          'success',
          `access.session:${currentSession.sessionId}`,
        )
      },
      async revokeSession(sessionId) {
        const session = fixtureSessions.find(record => record.sessionId === sessionId)
        if (!session) {
          throw new WorkspaceApiError({
            message: 'session not found',
            status: 404,
            requestId: 'req-fixture-session-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        session.status = 'revoked'
        appendAudit('access.sessions.revoke', 'success', `access.session:${sessionId}`)
      },
      async revokeUserSessions(userId) {
        fixtureSessions.forEach((session) => {
          if (session.userId === userId) {
            session.status = 'revoked'
          }
        })
        appendAudit('access.sessions.revoke-user', 'success', `access.user-sessions:${userId}`)
      },
      async listUsers() {
        return clone(accessUsers)
      },
      async updateUserPreset(userId, record) {
        const preset = accessRolePresets.find(candidate => candidate.code === record.presetCode)
        if (!preset) {
          throw new WorkspaceApiError({
            message: 'access preset not found',
            status: 404,
            requestId: 'req-access-preset-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        const managedRoleCodes = new Set(
          preset.templateCodes.flatMap(templateCode =>
            accessRoleTemplates
              .filter(template => template.code === templateCode)
              .flatMap(template => template.managedRoleCodes),
          ),
        )
        const managedRoleIds = new Set(
          accessRoles
            .filter(role => managedRoleCodes.has(role.code))
            .map(role => role.id),
        )

        accessRoleBindings = accessRoleBindings.filter((binding) => {
          if (binding.subjectType !== 'user' || binding.subjectId !== userId) {
            return true
          }

          const role = accessRoles.find(candidate => candidate.id === binding.roleId)
          return !(role && role.code.startsWith('system.'))
        })

        managedRoleIds.forEach((roleId) => {
          accessRoleBindings.push({
            id: `binding-${userId}-${roleId}-${Date.now()}`,
            roleId,
            subjectType: 'user',
            subjectId: userId,
            effect: 'allow',
          })
        })

        appendAudit('access.users.preset.update', 'success', `access.user-preset:${userId}`)
        return buildAccessMemberSummary(userId)
      },
      async createUser(record) {
        const created = {
          id: `access-user-${Date.now()}`,
          username: record.username,
          displayName: record.displayName,
          status: record.status,
          passwordState: record.password ? 'set' : 'reset-required',
        }
        accessUsers = [...accessUsers, created]
        workspaceState.users = workspaceState.users.concat({
          ...created,
        })
        appendAudit('access.users.create', 'success', `access.user:${created.id}`)
        return clone(created)
      },
      async updateUser(userId, record) {
        const current = accessUsers.find(user => user.id === userId)
        if (!current) {
          throw new WorkspaceApiError({
            message: 'access user not found',
            status: 404,
            requestId: 'req-access-user-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        const updated = {
          ...current,
          username: record.username,
          displayName: record.displayName,
          status: record.status,
          passwordState: record.resetPassword ? 'reset-required' : (record.password ? 'set' : current.passwordState),
        }
        accessUsers = accessUsers.map(user => user.id === userId ? updated : user)
        workspaceState.users = workspaceState.users.map(user => user.id === userId ? { ...user, ...updated } : user)
        appendAudit('access.users.update', 'success', `access.user:${userId}`)
        return clone(updated)
      },
      async deleteUser(userId) {
        accessUsers = accessUsers.filter(user => user.id !== userId)
        workspaceState.users = workspaceState.users.filter(user => user.id !== userId)
        accessUserOrgAssignments = accessUserOrgAssignments.filter(assignment => assignment.userId !== userId)
        accessRoleBindings = accessRoleBindings.filter(binding => !(binding.subjectType === 'user' && binding.subjectId === userId))
        accessDataPolicies = accessDataPolicies.filter(policy => !(policy.subjectType === 'user' && policy.subjectId === userId))
        appendAudit('access.users.delete', 'success', `access.user:${userId}`)
      },
      async listOrgUnits() {
        return clone(accessOrgUnits)
      },
      async createOrgUnit(record) {
        const created = {
          id: `org-${Date.now()}`,
          parentId: record.parentId,
          code: record.code,
          name: record.name,
          status: record.status,
        }
        accessOrgUnits = [...accessOrgUnits, created]
        return clone(created)
      },
      async updateOrgUnit(orgUnitId, record) {
        const updated = {
          id: orgUnitId,
          parentId: record.parentId,
          code: record.code,
          name: record.name,
          status: record.status,
        }
        accessOrgUnits = accessOrgUnits.map(unit => unit.id === orgUnitId ? updated : unit)
        return clone(updated)
      },
      async deleteOrgUnit(orgUnitId) {
        accessOrgUnits = accessOrgUnits.filter(unit => unit.id !== orgUnitId)
        accessUserOrgAssignments = accessUserOrgAssignments.filter(assignment => assignment.orgUnitId !== orgUnitId)
      },
      async listPositions() {
        return clone(accessPositions)
      },
      async createPosition(record) {
        const created = {
          id: `position-${Date.now()}`,
          code: record.code,
          name: record.name,
          status: record.status,
        }
        accessPositions = [...accessPositions, created]
        return clone(created)
      },
      async updatePosition(positionId, record) {
        const updated = {
          id: positionId,
          code: record.code,
          name: record.name,
          status: record.status,
        }
        accessPositions = accessPositions.map(position => position.id === positionId ? updated : position)
        return clone(updated)
      },
      async deletePosition(positionId) {
        accessPositions = accessPositions.filter(position => position.id !== positionId)
        accessUserOrgAssignments = accessUserOrgAssignments.map(assignment => ({
          ...assignment,
          positionIds: assignment.positionIds.filter(id => id !== positionId),
        }))
      },
      async listUserGroups() {
        return clone(accessUserGroups)
      },
      async createUserGroup(record) {
        const created = {
          id: `group-${Date.now()}`,
          code: record.code,
          name: record.name,
          status: record.status,
        }
        accessUserGroups = [...accessUserGroups, created]
        return clone(created)
      },
      async updateUserGroup(groupId, record) {
        const updated = {
          id: groupId,
          code: record.code,
          name: record.name,
          status: record.status,
        }
        accessUserGroups = accessUserGroups.map(group => group.id === groupId ? updated : group)
        return clone(updated)
      },
      async deleteUserGroup(groupId) {
        accessUserGroups = accessUserGroups.filter(group => group.id !== groupId)
        accessUserOrgAssignments = accessUserOrgAssignments.map(assignment => ({
          ...assignment,
          userGroupIds: assignment.userGroupIds.filter(id => id !== groupId),
        }))
      },
      async listUserOrgAssignments() {
        return clone(accessUserOrgAssignments)
      },
      async upsertUserOrgAssignment(record) {
        const nextRecord = {
          userId: record.userId,
          orgUnitId: record.orgUnitId,
          isPrimary: record.isPrimary,
          positionIds: clone(record.positionIds),
          userGroupIds: clone(record.userGroupIds),
        }
        accessUserOrgAssignments = accessUserOrgAssignments
          .filter(assignment => !(assignment.userId === record.userId && assignment.orgUnitId === record.orgUnitId))
          .concat(nextRecord)
        return clone(nextRecord)
      },
      async deleteUserOrgAssignment(userId, orgUnitId) {
        accessUserOrgAssignments = accessUserOrgAssignments.filter(assignment => !(assignment.userId === userId && assignment.orgUnitId === orgUnitId))
      },
      async listRoles() {
        return clone(accessRoles)
      },
      async createRole(record) {
        assertCustomRoleCode(record.code)
        const created = {
          id: `role-${Date.now()}`,
          code: record.code,
          name: record.name,
          description: record.description,
          source: 'custom' as const,
          editable: true,
          status: record.status,
          permissionCodes: clone(record.permissionCodes),
        }
        accessRoles = [...accessRoles, created]
        return clone(created)
      },
      async updateRole(roleId, record) {
        const current = accessRoles.find(role => role.id === roleId)
        if (!current) {
          throw new WorkspaceApiError({
            message: 'access role not found',
            status: 404,
            requestId: 'req-access-role-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        if (current.source === 'custom') {
          assertCustomRoleCode(record.code)
        }
        const updated = {
          id: roleId,
          code: record.code,
          name: record.name,
          description: record.description,
          source: current.source,
          editable: current.editable,
          status: record.status,
          permissionCodes: clone(record.permissionCodes),
        }
        accessRoles = accessRoles.map(role => role.id === roleId ? updated : role)
        return clone(updated)
      },
      async deleteRole(roleId) {
        accessRoles = accessRoles.filter(role => role.id !== roleId)
        accessRoleBindings = accessRoleBindings.filter(binding => binding.roleId !== roleId)
      },
      async listPermissionDefinitions() {
        return clone(workspaceState.permissionDefinitions)
      },
      async listRoleBindings() {
        return clone(accessRoleBindings)
      },
      async createRoleBinding(record) {
        const created = {
          id: `binding-${Date.now()}`,
          roleId: record.roleId,
          subjectType: record.subjectType,
          subjectId: record.subjectId,
          effect: record.effect,
        }
        accessRoleBindings = [...accessRoleBindings, created]
        appendAudit('access.role-bindings.create', 'success', `access.role-binding:${created.id}`)
        return clone(created)
      },
      async updateRoleBinding(bindingId, record) {
        const updated = {
          id: bindingId,
          roleId: record.roleId,
          subjectType: record.subjectType,
          subjectId: record.subjectId,
          effect: record.effect,
        }
        accessRoleBindings = accessRoleBindings.map(binding => binding.id === bindingId ? updated : binding)
        appendAudit('access.role-bindings.update', 'success', `access.role-binding:${bindingId}`)
        return clone(updated)
      },
      async deleteRoleBinding(bindingId) {
        accessRoleBindings = accessRoleBindings.filter(binding => binding.id !== bindingId)
        appendAudit('access.role-bindings.delete', 'success', `access.role-binding:${bindingId}`)
      },
      async listDataPolicies() {
        return clone(accessDataPolicies)
      },
      async createDataPolicy(record) {
        const created = {
          id: `data-policy-${Date.now()}`,
          name: record.name,
          subjectType: record.subjectType,
          subjectId: record.subjectId,
          resourceType: record.resourceType,
          scopeType: record.scopeType,
          projectIds: clone(record.projectIds),
          tags: clone(record.tags),
          classifications: clone(record.classifications ?? []),
          effect: record.effect,
        }
        accessDataPolicies = [...accessDataPolicies, created]
        appendAudit('access.data-policies.create', 'success', `access.data-policy:${created.id}`)
        return clone(created)
      },
      async updateDataPolicy(policyId, record) {
        const updated = {
          id: policyId,
          name: record.name,
          subjectType: record.subjectType,
          subjectId: record.subjectId,
          resourceType: record.resourceType,
          scopeType: record.scopeType,
          projectIds: clone(record.projectIds),
          tags: clone(record.tags),
          classifications: clone(record.classifications ?? []),
          effect: record.effect,
        }
        accessDataPolicies = accessDataPolicies.map(policy => policy.id === policyId ? updated : policy)
        appendAudit('access.data-policies.update', 'success', `access.data-policy:${policyId}`)
        return clone(updated)
      },
      async deleteDataPolicy(policyId) {
        accessDataPolicies = accessDataPolicies.filter(policy => policy.id !== policyId)
        appendAudit('access.data-policies.delete', 'success', `access.data-policy:${policyId}`)
      },
      async listResourcePolicies() {
        return clone(accessResourcePolicies)
      },
      async createResourcePolicy(record) {
        const created = {
          id: `resource-policy-${Date.now()}`,
          subjectType: record.subjectType,
          subjectId: record.subjectId,
          resourceType: record.resourceType,
          resourceId: record.resourceId,
          action: record.action,
          effect: record.effect,
        }
        accessResourcePolicies = [...accessResourcePolicies, created]
        workspaceState.resourcePolicies = clone(accessResourcePolicies)
        appendAudit('access.resource-policies.create', 'success', `access.resource-policy:${created.id}`)
        return clone(created)
      },
      async updateResourcePolicy(policyId, record) {
        const updated = {
          id: policyId,
          subjectType: record.subjectType,
          subjectId: record.subjectId,
          resourceType: record.resourceType,
          resourceId: record.resourceId,
          action: record.action,
          effect: record.effect,
        }
        accessResourcePolicies = accessResourcePolicies.map(policy => policy.id === policyId ? updated : policy)
        workspaceState.resourcePolicies = clone(accessResourcePolicies)
        appendAudit('access.resource-policies.update', 'success', `access.resource-policy:${policyId}`)
        return clone(updated)
      },
      async deleteResourcePolicy(policyId) {
        accessResourcePolicies = accessResourcePolicies.filter(policy => policy.id !== policyId)
        workspaceState.resourcePolicies = clone(accessResourcePolicies)
        appendAudit('access.resource-policies.delete', 'success', `access.resource-policy:${policyId}`)
      },
      async listMenuDefinitions() {
        return workspaceState.menus.map(menu => ({
          id: menu.id,
          parentId: menu.parentId,
          label: menu.label,
          routeName: menu.routeName,
          source: menu.source,
          status: menu.status,
          order: menu.order,
          featureCode: `feature:${menu.routeName ?? menu.id}`,
        }))
      },
      async listFeatureDefinitions() {
        return workspaceState.menus.map(menu => ({
          id: getFeatureCode(menu.id, menu.routeName),
          code: getFeatureCode(menu.id, menu.routeName),
          label: menu.label,
          requiredPermissionCodes: getMenuRequiredPermissionCodes(menu.id),
        }))
      },
      async listMenuGateResults() {
        const user = getCurrentUser()
        if (!user) {
          return []
        }

        return buildMenuGateResults(user.id)
      },
      async listProtectedResources() {
        return buildProtectedResources()
      },
      async listMenuPolicies() {
        return clone(accessMenuPolicies)
      },
      async createMenuPolicy(record) {
        const created = {
          menuId: record.menuId,
          enabled: record.enabled,
          order: record.order,
          group: record.group,
          visibility: record.visibility,
        }
        accessMenuPolicies = accessMenuPolicies.filter(policy => policy.menuId !== record.menuId).concat(created)
        appendAudit('access.menu-policies.create', 'success', `access.menu-policy:${record.menuId}`)
        return clone(created)
      },
      async updateMenuPolicy(menuId, record) {
        const updated = {
          menuId,
          enabled: record.enabled,
          order: record.order,
          group: record.group,
          visibility: record.visibility,
        }
        accessMenuPolicies = accessMenuPolicies.filter(policy => policy.menuId !== menuId).concat(updated)
        appendAudit('access.menu-policies.update', 'success', `access.menu-policy:${menuId}`)
        return clone(updated)
      },
      async deleteMenuPolicy(menuId) {
        accessMenuPolicies = accessMenuPolicies.filter(policy => policy.menuId !== menuId)
        appendAudit('access.menu-policies.delete', 'success', `access.menu-policy:${menuId}`)
      },
      async upsertProtectedResource(resourceType, resourceId, input) {
        const current = resolveProtectedResourceDescriptor(resourceType, resourceId)
        if (!current) {
          throw new WorkspaceApiError({
            message: 'protected resource not found',
            status: 404,
            requestId: 'req-protected-resource-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        const updated = {
          ...current,
          resourceSubtype: input.resourceSubtype ?? current.resourceSubtype,
          projectId: input.projectId ?? current.projectId,
          ownerSubjectType: input.ownerSubjectType ?? current.ownerSubjectType,
          ownerSubjectId: input.ownerSubjectId ?? current.ownerSubjectId,
          tags: clone(input.tags ?? current.tags),
          classification: input.classification ?? current.classification,
        }
        protectedResourceMetadata.set(protectedResourceKey(resourceType, resourceId), clone(updated))
        workspaceState.protectedResourceMetadata = Array.from(protectedResourceMetadata.values()).map(record => clone(record))
        appendAudit('access.protected-resources.update', 'success', `${resourceType}:${resourceId}`, updated.projectId)
        return clone(updated)
      },
    },
    runtime: {
      async bootstrap(): Promise<RuntimeBootstrap> {
        return {
          provider: {
            providerId: 'anthropic',
            defaultModel: 'claude-sonnet-4-5',
            defaultSurface: 'conversation',
            protocolFamily: 'anthropic_messages',
          },
          sessions: [...workspaceState.runtimeSessions.values()].map(state => clone(state.detail.summary)),
        }
      },
      async getConfig(): Promise<RuntimeEffectiveConfig> {
        return clone(workspaceState.runtimeWorkspaceConfig)
      },
      async validateConfig(_patch: RuntimeConfigPatch): Promise<RuntimeConfigValidationResult> {
        return {
          valid: true,
          errors: [],
          warnings: [],
        }
      },
      async upsertConfiguredModelCredential(
        configuredModelId: string,
        input: RuntimeConfiguredModelCredentialUpsertInput,
      ): Promise<RuntimeConfiguredModelCredentialRecord> {
        const credentialRef = `secret-ref:fixture:${configuredModelId}`
        managedConfiguredModelSecrets.set(credentialRef, input.apiKey)
        return {
          configuredModelId,
          credentialRef,
          storageKind: 'os-keyring',
          status: 'configured',
        }
      },
      async deleteConfiguredModelCredential(configuredModelId: string) {
        managedConfiguredModelSecrets.delete(`secret-ref:fixture:${configuredModelId}`)
      },
      async validateConfiguredModel(input) {
        const configuredModel = workspaceState.catalog.configuredModels.find(model => model.configuredModelId === input.configuredModelId)
        return {
          valid: true,
          reachable: true,
          configuredModelId: input.configuredModelId,
          configuredModelName: configuredModel?.name,
          requestId: 'fixture-probe-request',
          consumedTokens: 8,
          errors: [],
          warnings: [],
        }
      },
      async saveConfig(patch: RuntimeConfigPatch): Promise<RuntimeEffectiveConfig> {
        const source = workspaceState.runtimeWorkspaceConfig.sources.find(item => item.scope === 'workspace')
        if (source) {
          const current = (source.document ?? {}) as Record<string, any>
          source.document = applyJsonMergePatch(current, patch.patch as Record<string, any>)
          source.exists = true
          source.loaded = true
        }

        workspaceState.runtimeWorkspaceConfig = {
          ...workspaceState.runtimeWorkspaceConfig,
          effectiveConfig: applyJsonMergePatch(
            workspaceState.runtimeWorkspaceConfig.effectiveConfig as Record<string, any>,
            patch.patch as Record<string, any>,
          ),
          effectiveConfigHash: `${workspaceState.workspace.id}-cfg-hash-${Date.now()}`,
          validation: {
            valid: true,
            errors: [],
            warnings: [],
          },
        }

        return clone(workspaceState.runtimeWorkspaceConfig)
      },
      async getProjectConfig(projectId: string): Promise<RuntimeEffectiveConfig> {
        return clone(ensureRuntimeProjectConfig(projectId))
      },
      async validateProjectConfig(_projectId: string, _patch: RuntimeConfigPatch): Promise<RuntimeConfigValidationResult> {
        return {
          valid: true,
          errors: [],
          warnings: [],
        }
      },
      async saveProjectConfig(projectId: string, patch: RuntimeConfigPatch): Promise<RuntimeEffectiveConfig> {
        const config = ensureRuntimeProjectConfig(projectId)
        const source = config.sources.find(item => item.scope === 'project')
        if (source) {
          const current = (source.document ?? {}) as Record<string, any>
          source.document = applyJsonMergePatch(current, patch.patch as Record<string, any>)
          source.exists = true
          source.loaded = true
        }
        workspaceState.runtimeProjectConfigs[projectId] = {
          ...config,
          effectiveConfig: applyJsonMergePatch(
            config.effectiveConfig as Record<string, any>,
            patch.patch as Record<string, any>,
          ),
          effectiveConfigHash: `${workspaceState.workspace.id}-${projectId}-project-cfg-hash-${Date.now()}`,
          validation: {
            valid: true,
            errors: [],
            warnings: [],
          },
        }
        return clone(workspaceState.runtimeProjectConfigs[projectId])
      },
      async getUserConfig(): Promise<RuntimeEffectiveConfig> {
        return clone(workspaceState.runtimeUserConfig)
      },
      async validateUserConfig(_patch: RuntimeConfigPatch): Promise<RuntimeConfigValidationResult> {
        return {
          valid: true,
          errors: [],
          warnings: [],
        }
      },
      async saveUserConfig(patch: RuntimeConfigPatch): Promise<RuntimeEffectiveConfig> {
        const source = workspaceState.runtimeUserConfig.sources.find(item => item.scope === 'user')
        if (source) {
          const current = (source.document ?? {}) as Record<string, any>
          source.document = applyJsonMergePatch(current, patch.patch as Record<string, any>)
          source.exists = true
          source.loaded = true
        }
        workspaceState.runtimeUserConfig = {
          ...workspaceState.runtimeUserConfig,
          effectiveConfig: applyJsonMergePatch(
            workspaceState.runtimeUserConfig.effectiveConfig as Record<string, any>,
            patch.patch as Record<string, any>,
          ),
          effectiveConfigHash: `${workspaceState.workspace.id}-user-owner-runtime-cfg-hash-${Date.now()}`,
          validation: {
            valid: true,
            errors: [],
            warnings: [],
          },
        }
        return clone(workspaceState.runtimeUserConfig)
      },
      async listSessions(): Promise<RuntimeSessionSummary[]> {
        return [...workspaceState.runtimeSessions.values()].map(state => clone(state.detail.summary))
      },
      async getDeliverableDetail(deliverableId) {
        return clone(getDeliverableDetail(deliverableId))
      },
      async listDeliverableVersions(deliverableId) {
        ensureDeliverableVersionState(deliverableId)
        return clone(deliverableVersionSummaries.get(deliverableId) ?? [])
      },
      async getDeliverableVersionContent(deliverableId, version) {
        ensureDeliverableVersionState(deliverableId)
        const content = deliverableVersionContents.get(`${deliverableId}:${version}`)
        if (!content) {
          throw new WorkspaceApiError({
            message: 'deliverable version not found',
            status: 404,
            requestId: 'req-deliverable-version-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        return clone(content)
      },
      async createDeliverableVersion(deliverableId, input) {
        ensureDeliverableVersionState(deliverableId)
        const artifact = workspaceState.deliverables.find(record => record.id === deliverableId)
        if (!artifact) {
          throw new WorkspaceApiError({
            message: 'deliverable not found',
            status: 404,
            requestId: 'req-deliverable-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        const nextVersion = artifact.latestVersion + 1
        const updatedAt = Date.now()
        const previewKind = input.previewKind ?? resolveDeliverablePreviewKind(artifact.contentType, artifact.previewKind)
        const contentType = input.contentType ?? artifact.contentType
        const title = input.title?.trim() || artifact.title
        const textContent = input.textContent
          ?? (input.dataBase64 ? decodeBase64Text(input.dataBase64) : undefined)
          ?? deliverableVersionContents.get(`${deliverableId}:${artifact.latestVersion}`)?.textContent
          ?? ''

        const versionSummary: DeliverableVersionSummary = {
          artifactId: deliverableId,
          version: nextVersion,
          title,
          updatedAt,
          previewKind,
          contentType,
          byteSize: Math.max(textContent.length, 1),
          contentHash: `${deliverableId}-hash-v${nextVersion}`,
          parentVersion: input.parentVersion ?? artifact.latestVersion,
          sessionId: resolveDeliverableSessionState(artifact.conversationId)?.detail.summary.id,
          runId: resolveDeliverableSessionState(artifact.conversationId)?.detail.run.id,
          sourceMessageId: input.sourceMessageId,
        }
        deliverableVersionSummaries.set(
          deliverableId,
          [versionSummary, ...(deliverableVersionSummaries.get(deliverableId) ?? [])]
            .sort((left, right) => right.version - left.version),
        )
        deliverableVersionContents.set(`${deliverableId}:${nextVersion}`, {
          artifactId: deliverableId,
          version: nextVersion,
          editable: true,
          fileName: `${title}.md`,
          previewKind,
          contentType,
          byteSize: versionSummary.byteSize,
          textContent,
          dataBase64: input.dataBase64,
        })

        updateDeliverableRecord(deliverableId, {
          ...artifact,
          title,
          latestVersion: nextVersion,
          latestVersionRef: {
            artifactId: deliverableId,
            title,
            version: nextVersion,
            previewKind,
            contentType,
            updatedAt,
          },
          previewKind,
          contentType,
          updatedAt,
        })

        return clone(getDeliverableDetail(deliverableId))
      },
      async createSession(input) {
        const existing = [...workspaceState.runtimeSessions.values()].find(state => state.detail.summary.conversationId === input.conversationId)
        if (existing) {
          return clone(existing.detail)
        }

        const detail = createSessionDetail(
          input.conversationId,
          input.projectId ?? '',
          input.title,
          input.sessionKind ?? 'project',
          input.selectedActorRef,
          input.selectedConfiguredModelId,
          input.executionPermissionMode,
        )
        const state: RuntimeSessionState = {
          detail,
          events: [],
          nextSequence: 1,
        }
        workspaceState.runtimeSessions.set(detail.summary.id, state)
        return clone(detail)
      },
      async loadSession(sessionId) {
        return clone(ensureRuntimeState(sessionId).detail)
      },
      async promoteDeliverable(deliverableId, input) {
        const artifact = workspaceState.deliverables.find(record => record.id === deliverableId)
        if (!artifact) {
          throw new WorkspaceApiError({
            message: 'deliverable not found',
            status: 404,
            requestId: 'req-deliverable-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        const updatedAt = Date.now()
        const nextStatus = input.kind === 'candidate' ? 'candidate' : 'shared'
        const existingKnowledge = workspaceState.projectKnowledge[artifact.projectId]?.find(
          record => record.sourceRef === deliverableId,
        )
        const knowledgeRecord = {
          id: existingKnowledge?.id ?? `${artifact.projectId}-knowledge-${updatedAt}`,
          workspaceId: artifact.workspaceId,
          projectId: artifact.projectId,
          title: input.title,
          summary: input.summary,
          kind: input.kind,
          status: nextStatus,
          sourceType: 'artifact' as const,
          sourceRef: deliverableId,
          updatedAt,
        }
        workspaceState.projectKnowledge[artifact.projectId] = [
          knowledgeRecord,
          ...(workspaceState.projectKnowledge[artifact.projectId] ?? []).filter(
            record => record.id !== knowledgeRecord.id,
          ),
        ]

        updateDeliverableRecord(deliverableId, {
          ...artifact,
          promotionState: input.kind === 'candidate' ? 'candidate' : 'promoted',
          updatedAt,
        })

        return clone(createKnowledgeEntryRecord(knowledgeRecord))
      },
      async pollEvents(sessionId, options = {}) {
        return clone(eventsAfter(ensureRuntimeState(sessionId), options.after))
      },
      async subscribeEvents(sessionId, options) {
        const state = ensureRuntimeState(sessionId)
        const timers = eventsAfter(state, options.lastEventId).map((event, index) => window.setTimeout(() => {
          options.onEvent(clone(event))
        }, index * 5))

        return {
          mode: 'sse' as const,
          close: () => {
            timers.forEach(timer => window.clearTimeout(timer))
          },
        }
      },
      async submitUserTurn(sessionId, input) {
        const state = ensureRuntimeState(sessionId)
        const permissionMode = resolveRuntimePermissionMode(input.permissionMode ?? 'read-only')
        const baseSelectedMemory = state.detail.run.selectedMemory ?? []
        const ignoredMemoryIds = new Set(input.ignoredMemoryIds ?? [])
        const selectedMemory = input.recallMode === 'skip'
          ? []
          : baseSelectedMemory.filter(item => !ignoredMemoryIds.has(item.memoryId))
        const freshnessSummary = {
          freshnessRequired: true,
          freshCount: selectedMemory.filter(item => item.freshnessState === 'fresh').length,
          staleCount: selectedMemory.filter(item => item.freshnessState !== 'fresh').length,
        }
        const pendingMemoryProposal = input.memoryIntent
          ? {
              proposalId: `memory-proposal-${state.detail.summary.conversationId}`,
              sessionId,
              sourceRunId: state.detail.run.id,
              memoryId: `mem-${state.detail.summary.conversationId}-${input.memoryIntent}`,
              title: `${input.memoryIntent} memory proposal`,
              summary: `Capture ${input.memoryIntent} durable memory from the latest user turn.`,
              kind: input.memoryIntent,
              scope: state.detail.summary.projectId ? 'project' : 'user',
              proposalState: 'pending',
              proposalReason: 'user-feedback',
            }
          : undefined
        const configuredModelId = state.detail.summary.sessionPolicy.selectedConfiguredModelId
        const configuredModel = workspaceState.catalog.configuredModels.find(model => model.configuredModelId === configuredModelId)
        const registryModelId = configuredModel?.modelId ?? state.detail.run.modelId ?? 'claude-sonnet-4-5'
        const configuredModelName = configuredModel?.name ?? registryModelId
        const requestedActor = resolveSelectedActor(state.detail.summary.selectedActorRef)
        const actorLabel = resolveActorLabel(requestedActor.actorKind, requestedActor.actorId)
        const userMessage = createRuntimeMessage(
          state,
          'user',
          'You',
          input.content,
          registryModelId,
          configuredModelId,
          configuredModelName,
          requestedActor.actorKind,
          requestedActor.actorId,
        )
        state.detail.messages.push(userMessage)
        state.detail.summary.lastMessagePreview = input.content
        state.detail.summary.updatedAt = userMessage.timestamp
        state.detail.summary.memorySelectionSummary = {
          totalCandidateCount: baseSelectedMemory.length,
          selectedCount: selectedMemory.length,
          ignoredCount: ignoredMemoryIds.size,
          recallMode: input.recallMode ?? 'default',
          selectedMemoryIds: selectedMemory.map(item => item.memoryId),
        }
        state.detail.summary.pendingMemoryProposalCount = pendingMemoryProposal ? 1 : 0
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.message.created', { message: clone(userMessage) }))

        const requiresApproval = permissionMode === 'workspace-write'
          && /run pwd|bash pwd|workspace terminal/i.test(input.content)

        if (requiresApproval) {
          const approval = createApproval(state)
          const pendingTrace = createTraceItem(state, 'Awaiting approval before running the terminal command.', 'warning', requestedActor.actorKind, requestedActor.actorId, actorLabel)
          state.detail.pendingApproval = approval
          state.detail.trace.push(pendingTrace)
          state.detail.run = {
            ...state.detail.run,
            status: 'waiting_approval',
            currentStep: 'runtime.run.waitingApproval',
            updatedAt: approval.createdAt,
            selectedMemory: clone(selectedMemory),
            freshnessSummary: clone(freshnessSummary),
            pendingMemoryProposal: pendingMemoryProposal ? clone(pendingMemoryProposal) : undefined,
            configuredModelId,
            configuredModelName,
            modelId: registryModelId,
            nextAction: 'runtime.run.awaitingApproval',
            requestedActorKind: requestedActor.actorKind,
            requestedActorId: requestedActor.actorId,
            resolvedActorKind: requestedActor.actorKind,
            resolvedActorId: requestedActor.actorId,
            resolvedActorLabel: actorLabel,
          }
          state.detail.summary.status = 'waiting_approval'
          state.detail.summary.updatedAt = approval.createdAt
          state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.approval.requested', {
            approval: clone(approval),
            run: clone(state.detail.run),
          }))
          state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.trace.emitted', { trace: clone(pendingTrace) }))
          return clone(state.detail.run)
        }

        const modeLabel = permissionMode === 'read-only'
          ? 'read-only'
          : permissionMode === 'danger-full-access'
            ? 'danger-full-access'
            : 'workspace-write'
        const assistantMessage = createRuntimeMessage(
          state,
          'assistant',
          actorLabel,
          `Completed request in ${modeLabel} mode.`,
          registryModelId,
          configuredModelId,
          configuredModelName,
          requestedActor.actorKind,
          requestedActor.actorId,
        )
        const trace = createTraceItem(state, `Executed with ${modeLabel}.`, 'success', requestedActor.actorKind, requestedActor.actorId, actorLabel)

        state.detail.messages.push(assistantMessage)
        state.detail.trace.push(trace)
        state.detail.run = {
          ...state.detail.run,
          status: 'running',
          currentStep: 'runtime.run.processing',
          updatedAt: assistantMessage.timestamp,
          selectedMemory: clone(selectedMemory),
          freshnessSummary: clone(freshnessSummary),
          pendingMemoryProposal: pendingMemoryProposal ? clone(pendingMemoryProposal) : undefined,
          configuredModelId,
          configuredModelName,
          modelId: registryModelId,
          nextAction: 'runtime.run.processing',
          requestedActorKind: requestedActor.actorKind,
          requestedActorId: requestedActor.actorId,
          resolvedActorKind: requestedActor.actorKind,
          resolvedActorId: requestedActor.actorId,
          resolvedActorLabel: actorLabel,
        }
        const immediateRun: RuntimeRunSnapshot = clone(state.detail.run)
        state.detail.summary.status = 'running'
        state.detail.summary.updatedAt = assistantMessage.timestamp
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.message.created', { message: clone(assistantMessage) }))
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.trace.emitted', { trace: clone(trace) }))
        if (selectedMemory.length > 0) {
          state.events.push(createEvent(state, workspaceState.workspace.id, 'memory.selected', {
            selectedMemory: clone(selectedMemory),
            memorySelectionSummary: clone(state.detail.summary.memorySelectionSummary),
            freshnessSummary: clone(freshnessSummary),
            run: clone(state.detail.run),
          }))
        }
        if (pendingMemoryProposal) {
          state.events.push(createEvent(state, workspaceState.workspace.id, 'memory.proposed', {
            memoryProposal: clone(pendingMemoryProposal),
            run: clone(state.detail.run),
          }))
        }
        state.detail.run = {
          ...state.detail.run,
          status: 'completed',
          currentStep: 'runtime.run.completed',
          nextAction: 'runtime.run.idle',
          updatedAt: assistantMessage.timestamp + 10,
        }
        state.detail.summary.status = 'completed'
        state.detail.summary.updatedAt = state.detail.run.updatedAt
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.run.updated', { run: clone(state.detail.run) }))
        return immediateRun
      },
      async forkDeliverable(deliverableId, input) {
        const artifact = workspaceState.deliverables.find(record => record.id === deliverableId)
        if (!artifact) {
          throw new WorkspaceApiError({
            message: 'deliverable not found',
            status: 404,
            requestId: 'req-deliverable-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        findProjectRecord(input.projectId)
        const updatedAt = Date.now()
        const conversationId = `conv-fork-${deliverableId}-${updatedAt}`
        return {
          id: conversationId,
          workspaceId: artifact.workspaceId,
          projectId: input.projectId,
          sessionId: `rt-${conversationId}`,
          title: input.title?.trim() || `${artifact.title} Fork`,
          status: 'draft',
          lastMessagePreview: `Forked from ${artifact.title}`,
          updatedAt,
        }
      },
      async resolveApproval(sessionId, approvalId, input) {
        const state = ensureRuntimeState(sessionId)
        if (state.detail.pendingApproval?.id !== approvalId) {
          return
        }

        const resolutionStatus = input.decision === 'approve' ? 'approved' : 'rejected'
        const resolvedApproval: RuntimeApprovalRequest = {
          ...state.detail.pendingApproval,
          status: resolutionStatus,
        }
        const runStatus = input.decision === 'approve' ? 'completed' : 'blocked'
        state.detail.pendingApproval = undefined
        state.detail.run = {
          ...state.detail.run,
          status: runStatus,
          currentStep: input.decision === 'approve' ? 'runtime.run.completed' : 'runtime.run.blocked',
          updatedAt: state.detail.run.updatedAt + 10,
          nextAction: input.decision === 'approve' ? 'runtime.run.idle' : 'runtime.run.awaitingInput',
        }
        state.detail.summary.status = runStatus
        state.detail.summary.updatedAt = state.detail.run.updatedAt
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.approval.resolved', {
          approval: clone(resolvedApproval),
          decision: input.decision,
          run: clone(state.detail.run),
        }))

        if (input.decision === 'approve') {
          const assistantMessage = createRuntimeMessage(
            state,
            'assistant',
            state.detail.run.resolvedActorLabel ?? 'Octopus Runtime',
            'Command approved and execution completed.',
            state.detail.run.modelId,
            state.detail.run.configuredModelId,
            state.detail.run.configuredModelName,
            state.detail.run.resolvedActorKind ?? 'agent',
            state.detail.run.resolvedActorId ?? 'agent-architect',
          )
          const trace = createTraceItem(
            state,
            'Approval granted, command executed.',
            'success',
            state.detail.run.resolvedActorKind ?? 'agent',
            state.detail.run.resolvedActorId ?? 'agent-architect',
            state.detail.run.resolvedActorLabel ?? 'Octopus Runtime',
          )
          state.detail.messages.push(assistantMessage)
          state.detail.trace.push(trace)
          state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.message.created', { message: clone(assistantMessage) }))
          state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.trace.emitted', { trace: clone(trace) }))
        }
      },
      async resolveMemoryProposal(sessionId, proposalId, input) {
        const state = ensureRuntimeState(sessionId)
        const pendingProposal = state.detail.run.pendingMemoryProposal
        if (!pendingProposal || pendingProposal.proposalId !== proposalId) {
          return
        }

        const reviewedAt = Date.now()
        const proposalState = input.decision === 'approve'
          ? 'approved'
          : input.decision === 'revalidate'
            ? 'revalidated'
            : input.decision === 'ignore'
              ? 'ignored'
              : 'rejected'
        const resolvedProposal = {
          ...pendingProposal,
          proposalState,
          review: {
            decision: input.decision,
            note: input.note,
            reviewedAt,
          },
        }

        state.detail.run = {
          ...state.detail.run,
          pendingMemoryProposal: undefined,
        }
        state.detail.summary.pendingMemoryProposalCount = 0
        state.events.push(createEvent(
          state,
          workspaceState.workspace.id,
          input.decision === 'approve'
            ? 'memory.approved'
            : input.decision === 'revalidate'
              ? 'memory.revalidated'
              : 'memory.rejected',
          {
            memoryProposal: clone(resolvedProposal),
            run: clone(state.detail.run),
          },
        ))
      },
    },
  }

  return applySessionGuards(client)
}
