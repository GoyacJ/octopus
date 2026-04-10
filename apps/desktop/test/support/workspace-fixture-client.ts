import type {
  BindPetConversationInput,
  ChangeCurrentUserPasswordRequest,
  ChangeCurrentUserPasswordResponse,
  CopyWorkspaceSkillToManagedInput,
  CreateWorkspaceSkillInput,
  ExportWorkspaceAgentBundleInput,
  ExportWorkspaceAgentBundleResult,
  ImportWorkspaceAgentBundleInput,
  ImportWorkspaceAgentBundlePreview,
  ImportWorkspaceAgentBundlePreviewInput,
  ImportWorkspaceAgentBundleResult,
  PetConversationBinding,
  ProjectAgentLinkRecord,
  ProjectTeamLinkRecord,
  RegisterWorkspaceOwnerRequest,
  RegisterWorkspaceOwnerResponse,
  RuntimeApprovalRequest,
  RuntimeBootstrap,
  RuntimeConfigPatch,
  RuntimeConfigValidationResult,
  RuntimeEffectiveConfig,
  RuntimeRunSnapshot,
  RuntimeSessionSummary,
  SavePetPresenceInput,
  UpsertAgentInput,
  UpsertTeamInput,
  UpdateCurrentUserProfileRequest,
  UpdateProjectRequest,
  UpdateWorkspaceSkillFileInput,
  UpdateWorkspaceSkillInput,
  UpdateWorkspaceUserRequest,
  UserRecordSummary,
  WorkspaceConnectionRecord,
  WorkspaceMcpServerDocument,
  WorkspaceSessionTokenEnvelope,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceToolCatalogEntry,
  WorkspaceToolDisablePatch,
  WorkspaceDirectoryUploadEntry,
} from '@octopus/schema'
import { resolveRuntimePermissionMode } from '@octopus/schema'

import type { WorkspaceClient } from '@/tauri/workspace-client'
import { WorkspaceApiError } from '@/tauri/shared'

import { WORKSPACE_SESSIONS, clone } from './workspace-fixture-bootstrap'
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
): WorkspaceClient {
  const ensureRuntimeState = (sessionId: string): RuntimeSessionState => {
    const state = workspaceState.runtimeSessions.get(sessionId)
    if (!state) {
      throw new Error(`Unknown runtime session ${sessionId}`)
    }
    return state
  }

  const defaultSession = clone(WORKSPACE_SESSIONS.find(item => item.workspaceConnectionId === connection.workspaceConnectionId)!)

  const buildSession = (userId: string, token = defaultSession.token): WorkspaceSessionTokenEnvelope['session'] => ({
    ...clone(defaultSession.session),
    userId,
    token,
  })

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
    const rootDirName = input.mode === 'single' ? 'agent-bundle-single' : 'agent-bundle-batch'
    const files = [
      ...input.agentIds.map((agentId, index) => ({
        fileName: `${agentId}.md`,
        contentType: 'text/markdown',
        byteSize: 64,
        dataBase64: btoa(`# ${agentId}\n`),
        relativePath: `${rootDirName}/agents/${index + 1}-${agentId}/${agentId}.md`,
      })),
      ...input.teamIds.map((teamId, index) => ({
        fileName: `${teamId}.md`,
        contentType: 'text/markdown',
        byteSize: 64,
        dataBase64: btoa(`# ${teamId}\n`),
        relativePath: `${rootDirName}/teams/${index + 1}-${teamId}/${teamId}.md`,
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

  return {
    system: {
      async bootstrap() {
        return clone(workspaceState.systemBootstrap)
      },
    },
    auth: {
      async login() {
        const user = workspaceState.users.find(record => record.id === workspaceState.workspace.ownerUserId) ?? workspaceState.users[0]
        return {
          session: buildSession(user?.id ?? 'user-owner'),
          workspace: clone(workspaceState.workspace),
        }
      },
      async registerOwner(request: RegisterWorkspaceOwnerRequest): Promise<RegisterWorkspaceOwnerResponse> {
        const ownerId = 'user-owner'
        const ownerAvatar = `data:${request.avatar.contentType};base64,${request.avatar.dataBase64}`
        const ownerRecord: UserRecordSummary = {
          id: ownerId,
          username: request.username,
          displayName: request.displayName,
          avatar: ownerAvatar,
          status: 'active',
          passwordState: 'set',
          roleIds: ['role-owner'],
          scopeProjectIds: [],
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
        workspaceState.permissionCenterOverview = {
          ...workspaceState.permissionCenterOverview,
          currentUser: clone(ownerRecord),
          roleNames: ['Owner'],
          metrics: workspaceState.permissionCenterOverview.metrics.map(metric =>
            metric.id === 'users'
              ? { ...metric, value: String(workspaceState.users.length) }
              : metric),
        }

        return {
          session: buildSession(ownerId, 'token-owner'),
          workspace: clone(workspaceState.workspace),
        }
      },
      async logout() {},
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

        const user = workspaceState.users.find(record => record.id === workspaceState.workspace.ownerUserId) ?? workspaceState.users[0]
        return buildSession(user?.id ?? 'user-owner')
      },
    },
    workspace: {
      async get() {
        return clone(workspaceState.workspace)
      },
      async getOverview() {
        return clone(workspaceState.overview)
      },
    },
    projects: {
      async list() {
        return clone(workspaceState.projects)
      },
      async create(input) {
        const project = createProjectRecord(workspaceState.workspace.id, input)
        workspaceState.projects = [...workspaceState.projects, project]
        workspaceState.dashboards[project.id] = {
          project: clone(project),
          metrics: [],
          recentConversations: [],
          recentActivity: [],
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
    },
    resources: {
      async listWorkspace() {
        return clone(workspaceState.workspaceResources)
      },
      async listProject(projectId) {
        return clone(workspaceState.projectResources[projectId] ?? [])
      },
    },
    artifacts: {
      async listWorkspace() {
        return clone(workspaceState.artifacts)
      },
    },
    knowledge: {
      async listWorkspace() {
        return clone(workspaceState.workspaceKnowledge)
      },
      async listProject(projectId) {
        return clone(workspaceState.projectKnowledge[projectId] ?? [])
      },
    },
    pet: {
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
          projectId: projectId ?? '',
          conversationId: input.conversationId,
          sessionId: input.sessionId,
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
      async getToolCatalog() {
        return clone(workspaceState.toolCatalog)
      },
      async setToolDisabled(patch: WorkspaceToolDisablePatch) {
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
        return clone(workspaceState.toolCatalog)
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
    automations: {
      async list() {
        return clone(workspaceState.automations)
      },
      async create(record) {
        workspaceState.automations = [...workspaceState.automations, clone(record)]
        return clone(record)
      },
      async update(automationId, record) {
        workspaceState.automations = workspaceState.automations.map(item => item.id === automationId ? clone(record) : item)
        return clone(record)
      },
      async delete(automationId) {
        workspaceState.automations = workspaceState.automations.filter(item => item.id !== automationId)
      },
    },
    rbac: {
      async getPermissionCenterOverview() {
        return clone(workspaceState.permissionCenterOverview)
      },
      async listUsers() {
        return clone(workspaceState.users)
      },
      async createUser(record) {
        const created: UserRecordSummary = {
          id: `user-${record.username}`,
          username: record.username,
          displayName: record.displayName,
          avatar: record.useDefaultAvatar || !record.avatar
            ? undefined
            : `data:${record.avatar.contentType};base64,${record.avatar.dataBase64}`,
          status: record.status,
          passwordState: record.useDefaultPassword || !record.password ? 'reset-required' : 'set',
          roleIds: clone(record.roleIds),
          scopeProjectIds: clone(record.scopeProjectIds),
        }
        workspaceState.users = [...workspaceState.users, clone(created)]
        workspaceState.userPasswords = {
          ...workspaceState.userPasswords,
          [created.id]: record.useDefaultPassword || !record.password ? 'changeme' : record.password,
        }
        workspaceState.permissionCenterOverview = {
          ...workspaceState.permissionCenterOverview,
          metrics: workspaceState.permissionCenterOverview.metrics.map(metric =>
            metric.id === 'users'
              ? { ...metric, value: String(workspaceState.users.length) }
              : metric),
        }
        return clone(created)
      },
      async updateUser(userId, record: UpdateWorkspaceUserRequest) {
        const currentUser = workspaceState.users.find(item => item.id === userId)
        if (!currentUser) {
          throw new WorkspaceApiError({
            message: 'User not found.',
            status: 404,
            requestId: 'req-user-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        const updated: UserRecordSummary = {
          ...currentUser,
          username: record.username,
          displayName: record.displayName,
          avatar: record.removeAvatar
            ? undefined
            : record.avatar
              ? `data:${record.avatar.contentType};base64,${record.avatar.dataBase64}`
              : currentUser.avatar,
          status: record.status,
          passwordState: record.resetPasswordToDefault || (!record.password && currentUser.passwordState === 'reset-required')
            ? 'reset-required'
            : record.password
              ? 'set'
              : currentUser.passwordState,
          roleIds: clone(record.roleIds),
          scopeProjectIds: clone(record.scopeProjectIds),
        }
        workspaceState.users = workspaceState.users.map(item => item.id === userId ? clone(updated) : item)
        if (record.resetPasswordToDefault) {
          workspaceState.userPasswords = {
            ...workspaceState.userPasswords,
            [userId]: 'changeme',
          }
        } else if (record.password) {
          workspaceState.userPasswords = {
            ...workspaceState.userPasswords,
            [userId]: record.password,
          }
        }
        if (workspaceState.permissionCenterOverview.currentUser.id === userId) {
          workspaceState.permissionCenterOverview = {
            ...workspaceState.permissionCenterOverview,
            currentUser: clone(updated),
          }
        }
        return clone(updated)
      },
      async deleteUser(userId) {
        workspaceState.users = workspaceState.users.filter(item => item.id !== userId)
        const nextPasswords = { ...workspaceState.userPasswords }
        delete nextPasswords[userId]
        workspaceState.userPasswords = nextPasswords
        workspaceState.permissionCenterOverview = {
          ...workspaceState.permissionCenterOverview,
          metrics: workspaceState.permissionCenterOverview.metrics.map(metric =>
            metric.id === 'users'
              ? { ...metric, value: String(workspaceState.users.length) }
              : metric),
        }
      },
      async updateCurrentUserProfile(input: UpdateCurrentUserProfileRequest) {
        const currentUserId = workspaceState.permissionCenterOverview.currentUser.id
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
        workspaceState.permissionCenterOverview = {
          ...workspaceState.permissionCenterOverview,
          currentUser: clone(updated),
        }
        return clone(updated)
      },
      async changeCurrentUserPassword(input: ChangeCurrentUserPasswordRequest): Promise<ChangeCurrentUserPasswordResponse> {
        const currentUserId = workspaceState.permissionCenterOverview.currentUser.id
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
        if (input.newPassword === input.currentPassword) {
          throw new WorkspaceApiError({
            message: 'New password must be different from the current password.',
            status: 400,
            requestId: 'req-user-password-same',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }

        workspaceState.userPasswords = {
          ...workspaceState.userPasswords,
          [currentUserId]: input.newPassword,
        }
        workspaceState.users = workspaceState.users.map((user) => {
          if (user.id !== currentUserId) {
            return user
          }
          return {
            ...user,
            passwordState: 'set',
          }
        })
        workspaceState.permissionCenterOverview = {
          ...workspaceState.permissionCenterOverview,
          currentUser: {
            ...workspaceState.permissionCenterOverview.currentUser,
            passwordState: 'set',
          },
        }
        return {
          passwordState: 'set',
        }
      },
      async listRoles() {
        return clone(workspaceState.roles)
      },
      async createRole(record) {
        workspaceState.roles = [...workspaceState.roles, clone(record)]
        workspaceState.permissionCenterOverview = {
          ...workspaceState.permissionCenterOverview,
          metrics: workspaceState.permissionCenterOverview.metrics.map(metric =>
            metric.id === 'roles'
              ? { ...metric, value: String(workspaceState.roles.length) }
              : metric),
        }
        return clone(record)
      },
      async updateRole(roleId, record) {
        workspaceState.roles = workspaceState.roles.map(item => item.id === roleId ? clone(record) : item)
        return clone(record)
      },
      async deleteRole(roleId) {
        workspaceState.roles = workspaceState.roles.filter(item => item.id !== roleId)
        workspaceState.users = workspaceState.users.map(user => ({
          ...user,
          roleIds: user.roleIds.filter(id => id !== roleId),
        }))
        workspaceState.permissionCenterOverview = {
          ...workspaceState.permissionCenterOverview,
          currentUser: {
            ...workspaceState.permissionCenterOverview.currentUser,
            roleIds: workspaceState.permissionCenterOverview.currentUser.roleIds.filter(id => id !== roleId),
          },
          roleNames: workspaceState.permissionCenterOverview.roleNames.filter(name =>
            workspaceState.roles.some(role => role.name === name),
          ),
          metrics: workspaceState.permissionCenterOverview.metrics.map(metric =>
            metric.id === 'roles'
              ? { ...metric, value: String(workspaceState.roles.length) }
              : metric),
        }
      },
      async listPermissions() {
        return clone(workspaceState.permissions)
      },
      async createPermission(record) {
        workspaceState.permissions = [...workspaceState.permissions, clone(record)]
        return clone(record)
      },
      async updatePermission(permissionId, record) {
        workspaceState.permissions = workspaceState.permissions.map(item => item.id === permissionId ? clone(record) : item)
        return clone(record)
      },
      async deletePermission(permissionId) {
        workspaceState.permissions = workspaceState.permissions
          .filter(item => item.id !== permissionId)
          .map(permission => ({
            ...permission,
            memberPermissionIds: permission.memberPermissionIds.filter(id => id !== permissionId),
          }))
        workspaceState.roles = workspaceState.roles.map(role => ({
          ...role,
          permissionIds: role.permissionIds.filter(id => id !== permissionId),
        }))
      },
      async listMenus() {
        return clone(workspaceState.menus)
      },
      async createMenu(record) {
        workspaceState.menus = [...workspaceState.menus, clone(record)]
        return clone(record)
      },
      async updateMenu(menuId, record) {
        workspaceState.menus = workspaceState.menus.map(item => item.id === menuId ? clone(record) : item)
        return clone(record)
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
        return clone(workspaceState.runtimeProjectConfigs[projectId])
      },
      async validateProjectConfig(_projectId: string, _patch: RuntimeConfigPatch): Promise<RuntimeConfigValidationResult> {
        return {
          valid: true,
          errors: [],
          warnings: [],
        }
      },
      async saveProjectConfig(projectId: string, patch: RuntimeConfigPatch): Promise<RuntimeEffectiveConfig> {
        const config = workspaceState.runtimeProjectConfigs[projectId]
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
      async createSession(input) {
        const existing = [...workspaceState.runtimeSessions.values()].find(state => state.detail.summary.conversationId === input.conversationId)
        if (existing) {
          return clone(existing.detail)
        }

        const detail = createSessionDetail(input.conversationId, input.projectId, input.title, input.sessionKind ?? 'project')
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
        const permissionMode = resolveRuntimePermissionMode(input.permissionMode)
        const configuredModelId = input.configuredModelId ?? input.modelId ?? 'anthropic-primary'
        const configuredModel = workspaceState.catalog.configuredModels.find(model => model.configuredModelId === configuredModelId)
        const registryModelId = configuredModel?.modelId ?? input.modelId ?? 'claude-sonnet-4-5'
        const configuredModelName = configuredModel?.name ?? registryModelId
        const requestedActorKind = input.actorKind ?? 'agent'
        const requestedActorId = input.actorId ?? 'agent-architect'
        const actorRecord = requestedActorKind === 'team'
          ? workspaceState.teams.find(team => team.id === requestedActorId)
          : workspaceState.agents.find(agent => agent.id === requestedActorId)
        const actorLabel = actorRecord
          ? `${actorRecord.name} · ${requestedActorKind === 'team' ? 'Team' : 'Agent'}`
          : '默认智能体'
        const userMessage = createRuntimeMessage(
          state,
          'user',
          'You',
          input.content,
          registryModelId,
          configuredModelId,
          configuredModelName,
          requestedActorKind,
          requestedActorId,
        )
        state.detail.messages.push(userMessage)
        state.detail.summary.lastMessagePreview = input.content
        state.detail.summary.updatedAt = userMessage.timestamp
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.message.created', { message: clone(userMessage) }))

        const requiresApproval = permissionMode === 'workspace-write'
          && /run pwd|bash pwd|workspace terminal/i.test(input.content)

        if (requiresApproval) {
          const approval = createApproval(state)
          const pendingTrace = createTraceItem(state, 'Awaiting approval before running the terminal command.', 'warning', requestedActorKind, requestedActorId, actorLabel)
          state.detail.pendingApproval = approval
          state.detail.trace.push(pendingTrace)
          state.detail.run = {
            ...state.detail.run,
            status: 'waiting_approval',
            currentStep: 'runtime.run.waitingApproval',
            updatedAt: approval.createdAt,
            configuredModelId,
            configuredModelName,
            modelId: registryModelId,
            nextAction: 'runtime.run.awaitingApproval',
            requestedActorKind,
            requestedActorId,
            resolvedActorKind: requestedActorKind,
            resolvedActorId: requestedActorId,
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
          requestedActorKind,
          requestedActorId,
        )
        const trace = createTraceItem(state, `Executed with ${modeLabel}.`, 'success', requestedActorKind, requestedActorId, actorLabel)

        state.detail.messages.push(assistantMessage)
        state.detail.trace.push(trace)
        state.detail.run = {
          ...state.detail.run,
          status: 'running',
          currentStep: 'runtime.run.processing',
          updatedAt: assistantMessage.timestamp,
          configuredModelId,
          configuredModelName,
          modelId: registryModelId,
          nextAction: 'runtime.run.processing',
          requestedActorKind,
          requestedActorId,
          resolvedActorKind: requestedActorKind,
          resolvedActorId: requestedActorId,
          resolvedActorLabel: actorLabel,
        }
        const immediateRun: RuntimeRunSnapshot = clone(state.detail.run)
        state.detail.summary.status = 'running'
        state.detail.summary.updatedAt = assistantMessage.timestamp
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.message.created', { message: clone(assistantMessage) }))
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.trace.emitted', { trace: clone(trace) }))
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
    },
  }
}
