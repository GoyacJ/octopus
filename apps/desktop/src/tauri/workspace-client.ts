import type {
  AgentRecord,
  AutomationRecord,
  ChangeCurrentUserPasswordRequest,
  ChangeCurrentUserPasswordResponse,
  CopyWorkspaceSkillToManagedInput,
  CreateWorkspaceResourceFolderInput,
  CreateWorkspaceResourceInput,
  CredentialBinding,
  BindPetConversationInput,
  CreateProjectRequest,
  CreateWorkspaceUserRequest,
  CreateWorkspaceSkillInput,
  CreateRuntimeSessionInput,
  ImportWorkspaceSkillArchiveInput,
  ImportWorkspaceSkillFolderInput,
  ImportWorkspaceAgentBundleInput,
  ImportWorkspaceAgentBundlePreview,
  ImportWorkspaceAgentBundlePreviewInput,
  ImportWorkspaceAgentBundleResult,
  KnowledgeRecord,
  LoginRequest,
  LoginResponse,
  MenuRecord,
  ModelCatalogSnapshot,
  PermissionRecord,
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
  PetConversationBinding,
  PetPresenceState,
  PetWorkspaceSnapshot,
  RuntimeBootstrap,
  RuntimeConfigPatch,
  RuntimeConfigValidationResult,
  RuntimeConfiguredModelProbeInput,
  RuntimeConfiguredModelProbeResult,
  RuntimeEventEnvelope,
  RuntimeEffectiveConfig,
  RuntimePermissionMode,
  RuntimeRunSnapshot,
  RuntimeSessionDetail,
  RuntimeSessionSummary,
  SubmitRuntimeTurnInput,
  SystemBootstrapStatus,
  SavePetPresenceInput,
  TeamRecord,
  ToolRecord,
  UpdateCurrentUserProfileRequest,
  UpdateProjectRequest,
  UpdateWorkspaceResourceInput,
  UpsertAgentInput,
  UpsertTeamInput,
  UpdateWorkspaceUserRequest,
  UpdateWorkspaceSkillFileInput,
  UpdateWorkspaceSkillInput,
  UpsertWorkspaceMcpServerInput,
  WorkspaceMcpServerDocument,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceSkillTreeDocument,
  WorkspaceToolCatalogSnapshot,
  WorkspaceToolDisablePatch,
  UserCenterOverviewSnapshot,
  UserRecordSummary,
  WorkspaceConnectionRecord,
  WorkspaceOverviewSnapshot,
  WorkspaceResourceRecord,
  WorkspaceSessionTokenEnvelope,
  ArtifactRecord,
} from '@octopus/schema'
import { resolveRuntimePermissionMode } from '@octopus/schema'

import {
  createWorkspaceHeaders,
  decodeApiError,
  fetchWorkspaceApi,
  fetchWorkspaceOpenApi,
  joinBaseUrl,
  openWorkspaceOpenApiStream,
} from './shared'

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
    ) => Promise<ImportWorkspaceAgentBundlePreview>
    importBundle: (
      input: ImportWorkspaceAgentBundleInput,
    ) => Promise<ImportWorkspaceAgentBundleResult>
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
    getUserCenterOverview: () => Promise<UserCenterOverviewSnapshot>
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

function assertWorkspaceConnectionReady(context: WorkspaceClientContext): void {
  if (!context.connection.baseUrl || context.connection.status !== 'connected') {
    throw new Error(`Workspace connection ${context.connection.workspaceConnectionId} is unavailable`)
  }
}

function assertWorkspaceRequestReady(context: WorkspaceClientContext): WorkspaceSessionTokenEnvelope {
  assertWorkspaceConnectionReady(context)
  if (!context.session?.token) {
    throw new Error(`Workspace session is unavailable for ${context.connection.workspaceConnectionId}`)
  }

  return context.session
}

function parseRuntimeEventBlock(block: string): RuntimeEventEnvelope | null {
  const lines = block
    .split('\n')
    .map(line => line.trimEnd())
    .filter(Boolean)

  let data = ''
  let id = ''
  let eventType = ''

  for (const line of lines) {
    if (line.startsWith('id:')) {
      id = line.slice(3).trim()
      continue
    }
    if (line.startsWith('event:')) {
      eventType = line.slice(6).trim()
      continue
    }
    if (line.startsWith('data:')) {
      data += `${line.slice(5).trim()}`
    }
  }

  if (!data) {
    return null
  }

  const parsed = JSON.parse(data) as RuntimeEventEnvelope
  return {
    ...parsed,
    id: parsed.id || id,
    eventType: parsed.eventType || parsed.kind || eventType || 'runtime.error',
  }
}

async function fetchWorkspace<T>(
  context: WorkspaceClientContext,
  path: string,
  init?: RequestInit & {
    idempotencyKey?: string
  },
): Promise<T> {
  const session = assertWorkspaceRequestReady(context)
  return await fetchWorkspaceApi<T>(context.connection, path, {
    ...init,
    session,
    idempotencyKey: init?.idempotencyKey,
  })
}

async function fetchWorkspaceWithoutSession<T>(
  context: WorkspaceClientContext,
  path: string,
  init?: RequestInit & {
    idempotencyKey?: string
  },
): Promise<T> {
  assertWorkspaceConnectionReady(context)
  return await fetchWorkspaceApi<T>(context.connection, path, {
    ...init,
    idempotencyKey: init?.idempotencyKey,
  })
}

async function fetchWorkspaceVoid(
  context: WorkspaceClientContext,
  path: string,
  init?: RequestInit & {
    idempotencyKey?: string
  },
): Promise<void> {
  const session = assertWorkspaceRequestReady(context)
  const headers = createWorkspaceHeaders({
    ...init,
    session,
    workspace: context.connection,
    idempotencyKey: init?.idempotencyKey,
  })
  const requestId = headers.get('X-Request-Id') ?? 'req-unknown'
  const response = await fetch(joinBaseUrl(context.connection.baseUrl, path), {
    ...init,
    headers,
  })
  if (!response.ok) {
    throw await decodeApiError(response, requestId, context.connection.workspaceConnectionId)
  }
}

async function openRuntimeSseStream(
  context: WorkspaceClientContext,
  sessionId: string,
  options: RuntimeEventSubscriptionOptions,
): Promise<RuntimeEventSubscription> {
  const session = assertWorkspaceRequestReady(context)
  const controller = new AbortController()
  const response = await openWorkspaceOpenApiStream(
    context.connection,
    '/api/v1/runtime/sessions/{sessionId}/events',
    {
      session,
      signal: controller.signal,
      pathParams: {
        sessionId,
      },
      queryParams: options.lastEventId
        ? {
            after: options.lastEventId,
          }
        : undefined,
      headers: {
        Accept: 'text/event-stream',
        ...(options.lastEventId ? { 'Last-Event-ID': options.lastEventId } : {}),
      },
    },
  )

  if (!response.headers.get('Content-Type')?.includes('text/event-stream')) {
    throw new Error('Runtime event stream is unavailable')
  }

  if (!response.body) {
    throw new Error('Runtime event stream body is unavailable')
  }

  const reader = response.body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''

  const consume = async () => {
    try {
      while (true) {
        const result = await reader.read()
        if (result.done) {
          break
        }

        buffer += decoder.decode(result.value, { stream: true })
        const blocks = buffer.split('\n\n')
        buffer = blocks.pop() ?? ''

        for (const block of blocks) {
          const event = parseRuntimeEventBlock(block)
          if (event) {
            options.onEvent(event)
          }
        }
      }

      if (!controller.signal.aborted) {
        options.onError(new Error('Runtime event stream closed'))
      }
    } catch (error) {
      if (!controller.signal.aborted) {
        options.onError(error instanceof Error ? error : new Error('Runtime event stream failed'))
      }
    }
  }

  void consume()

  return {
    mode: 'sse',
    close() {
      controller.abort()
    },
  }
}

export function createWorkspaceClient(context: WorkspaceClientContext): WorkspaceClient {
  return {
    system: {
      async bootstrap() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/system/bootstrap',
          'get',
          {
            workspace: context.connection,
          },
        )
      },
    },
    auth: {
      async login(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/auth/login',
          'post',
          {
            body: JSON.stringify(input),
          },
        )
      },
      async registerOwner(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/auth/register-owner',
          'post',
          {
            body: JSON.stringify(input),
          },
        )
      },
      async logout() {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/auth/logout', 'post', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async session() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/auth/session', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
    },
    workspace: {
      async get() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async getOverview() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/overview', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
    },
    projects: {
      async list() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async create(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        })
      },
      async update(projectId, input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects/{projectId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
          pathParams: {
            projectId,
          },
        })
      },
      async getDashboard(projectId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/dashboard',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
            },
          },
        )
      },
    },
    resources: {
      async listWorkspace() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/resources', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async listProject(projectId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/resources',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
            },
          },
        )
      },
      async createWorkspace(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/resources', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        })
      },
      async createProject(projectId, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/resources',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
            pathParams: {
              projectId,
            },
          },
        )
      },
      async createProjectFolder(projectId, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/resources/folder',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
            pathParams: {
              projectId,
            },
          },
        )
      },
      async updateWorkspace(resourceId, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/resources/{resourceId}',
          'patch',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
            pathParams: {
              resourceId,
            },
          },
        )
      },
      async updateProject(projectId, resourceId, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/resources/{resourceId}',
          'patch',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
            pathParams: {
              projectId,
              resourceId,
            },
          },
        )
      },
      async deleteWorkspace(resourceId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/resources/{resourceId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            resourceId,
          },
        })
      },
      async deleteProject(projectId, resourceId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects/{projectId}/resources/{resourceId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            projectId,
            resourceId,
          },
        })
      },
    },
    artifacts: {
      async listWorkspace() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/artifacts', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
    },
    knowledge: {
      async listWorkspace() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/knowledge', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async listProject(projectId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/knowledge',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
            },
          },
        )
      },
    },
    pet: {
      async getSnapshot(projectId) {
        const session = assertWorkspaceRequestReady(context)
        if (projectId) {
          return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects/{projectId}/pet', 'get', {
            session,
            pathParams: {
              projectId,
            },
          })
        }
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/pet', 'get', {
          session,
        })
      },
      async savePresence(input, projectId) {
        const session = assertWorkspaceRequestReady(context)
        if (projectId) {
          return await fetchWorkspaceOpenApi(
            context.connection,
            '/api/v1/projects/{projectId}/pet/presence',
            'patch',
            {
              session,
              pathParams: {
                projectId,
              },
              body: JSON.stringify(input),
            },
          )
        }
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/pet/presence', 'patch', {
          session,
          body: JSON.stringify(input),
        })
      },
      async bindConversation(input, projectId) {
        const session = assertWorkspaceRequestReady(context)
        if (projectId) {
          return await fetchWorkspaceOpenApi(
            context.connection,
            '/api/v1/projects/{projectId}/pet/conversation',
            'put',
            {
              session,
              pathParams: {
                projectId,
              },
              body: JSON.stringify(input),
            },
          )
        }
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/pet/conversation', 'put', {
          session,
          body: JSON.stringify(input),
        })
      },
    },
    agents: {
      async list() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/agents', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as unknown as AgentRecord[]
      },
      async create(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/agents', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        }) as unknown as AgentRecord
      },
      async update(agentId, input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/agents/{agentId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            agentId,
          },
          body: JSON.stringify(input),
        }) as unknown as AgentRecord
      },
      async delete(agentId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/agents/{agentId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            agentId,
          },
        })
      },
      async previewImportBundle(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/agents/import-preview',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
          },
        ) as unknown as ImportWorkspaceAgentBundlePreview
      },
      async importBundle(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/agents/import',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
          },
        ) as unknown as ImportWorkspaceAgentBundleResult
      },
      async listProjectLinks(projectId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/agent-links',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
            },
          },
        ) as unknown as ProjectAgentLinkRecord[]
      },
      async linkProject(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/agent-links',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId: input.projectId,
            },
            body: JSON.stringify(input),
          },
        ) as unknown as ProjectAgentLinkRecord
      },
      async unlinkProject(projectId, agentId) {
        await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/agent-links/{agentId}',
          'delete',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
              agentId,
            },
          },
        )
      },
    },
    teams: {
      async list() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/teams', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      
      async create(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/teams', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        })
      },
      async update(teamId, input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/teams/{teamId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            teamId,
          },
          body: JSON.stringify(input),
        })
      },
      async delete(teamId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/teams/{teamId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            teamId,
          },
        })
      },
      async listProjectLinks(projectId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/team-links',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
            },
          },
        )
      },
      async linkProject(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/team-links',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId: input.projectId,
            },
            body: JSON.stringify(input),
          },
        )
      },
      async unlinkProject(projectId, teamId) {
        await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/team-links/{teamId}',
          'delete',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
              teamId,
            },
          },
        )
      },
    },
    catalog: {
      async getSnapshot() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/models', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as unknown as ModelCatalogSnapshot
      },
      async getToolCatalog() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/tool-catalog', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as unknown as WorkspaceToolCatalogSnapshot
      },
      async setToolDisabled(patch) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/tool-catalog/disable', 'patch', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(patch),
        }) as unknown as WorkspaceToolCatalogSnapshot
      },
      async getSkill(skillId) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/{skillId}', 'get', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            skillId,
          },
        }) as unknown as WorkspaceSkillDocument
      },
      async getSkillTree(skillId) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/{skillId}/tree', 'get', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            skillId,
          },
        }) as unknown as WorkspaceSkillTreeDocument
      },
      async getSkillFile(skillId, relativePath) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/catalog/skills/{skillId}/files/{relativePath}',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              skillId,
              relativePath,
            },
          },
        ) as WorkspaceSkillFileDocument
      },
      async createSkill(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        }) as unknown as WorkspaceSkillDocument
      },
      async updateSkill(skillId, input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/{skillId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            skillId,
          },
          body: JSON.stringify(input),
        }) as unknown as WorkspaceSkillDocument
      },
      async updateSkillFile(skillId, relativePath, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/catalog/skills/{skillId}/files/{relativePath}',
          'patch',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              skillId,
              relativePath,
            },
            body: JSON.stringify(input),
          },
        ) as WorkspaceSkillFileDocument
      },
      async importSkillArchive(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/import-archive', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        }) as unknown as WorkspaceSkillDocument
      },
      async importSkillFolder(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/import-folder', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        }) as unknown as WorkspaceSkillDocument
      },
      async copySkillToManaged(skillId, input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/{skillId}/copy-to-managed', 'post', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            skillId,
          },
          body: JSON.stringify(input),
        }) as unknown as WorkspaceSkillDocument
      },
      async deleteSkill(skillId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/{skillId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            skillId,
          },
        })
      },
      async getMcpServer(serverName) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/mcp-servers/{serverName}', 'get', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            serverName,
          },
        }) as unknown as WorkspaceMcpServerDocument
      },
      async createMcpServer(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/mcp-servers', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        }) as unknown as WorkspaceMcpServerDocument
      },
      async updateMcpServer(serverName, input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/mcp-servers/{serverName}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            serverName,
          },
          body: JSON.stringify(input),
        }) as unknown as WorkspaceMcpServerDocument
      },
      async deleteMcpServer(serverName) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/mcp-servers/{serverName}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            serverName,
          },
        })
      },
      async listModels() {
        const snapshot = await this.getSnapshot()
        return snapshot.models
      },
      async listProviderCredentials() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/catalog/provider-credentials',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as unknown as CredentialBinding[]
      },
      async listTools() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/tools', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as unknown as ToolRecord[]
      },
      async createTool(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/tools', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        }) as unknown as ToolRecord
      },
      async updateTool(toolId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/tools/{toolId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            toolId,
          },
          body: JSON.stringify(record),
        }) as unknown as ToolRecord
      },
      async deleteTool(toolId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/tools/{toolId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            toolId,
          },
        })
      },
    },
    automations: {
      async list() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/automations', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as unknown as AutomationRecord[]
      },
      async create(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/automations', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        }) as unknown as AutomationRecord
      },
      async update(automationId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/automations/{automationId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            automationId,
          },
          body: JSON.stringify(record),
        }) as unknown as AutomationRecord
      },
      async delete(automationId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/automations/{automationId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            automationId,
          },
        })
      },
    },
    rbac: {
      async getUserCenterOverview() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/user-center/overview',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        )
      },
      async listUsers() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/users', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async createUser(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/users', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        })
      },
      async updateUser(userId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/users/{userId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            userId,
          },
          body: JSON.stringify(record),
        })
      },
      async deleteUser(userId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/users/{userId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            userId,
          },
        })
      },
      async updateCurrentUserProfile(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/user-center/profile', 'patch', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        })
      },
      async changeCurrentUserPassword(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/user-center/profile/password', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        })
      },
      async listRoles() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/roles', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async createRole(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/roles', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        })
      },
      async updateRole(roleId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/roles/{roleId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            roleId,
          },
          body: JSON.stringify(record),
        })
      },
      async deleteRole(roleId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/roles/{roleId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            roleId,
          },
        })
      },
      async listPermissions() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/rbac/permissions',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        )
      },
      async createPermission(record) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/rbac/permissions',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(record),
          },
        )
      },
      async updatePermission(permissionId, record) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/rbac/permissions/{permissionId}',
          'patch',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              permissionId,
            },
            body: JSON.stringify(record),
          },
        )
      },
      async deletePermission(permissionId) {
        await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/rbac/permissions/{permissionId}',
          'delete',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              permissionId,
            },
          },
        )
      },
      async listMenus() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/menus', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async createMenu(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/menus', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        })
      },
      async updateMenu(menuId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/menus/{menuId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            menuId,
          },
          body: JSON.stringify(record),
        })
      },
    },
    runtime: {
      async bootstrap() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/bootstrap', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async getConfig() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/config', 'get')
      },
      async validateConfig(patch) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/config/validate', 'post', {
          body: JSON.stringify(patch),
        })
      },
      async validateConfiguredModel(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/config/configured-models/probe', 'post', {
          body: JSON.stringify(input),
        })
      },
      async saveConfig(patch) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/config/scopes/{scope}', 'patch', {
          pathParams: {
            scope: 'workspace',
          },
          body: JSON.stringify(patch),
        })
      },
      async getProjectConfig(projectId) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects/{projectId}/runtime-config', 'get', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            projectId,
          },
        })
      },
      async validateProjectConfig(projectId, patch) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects/{projectId}/runtime-config/validate', 'post', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            projectId,
          },
          body: JSON.stringify(patch),
        })
      },
      async saveProjectConfig(projectId, patch) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects/{projectId}/runtime-config', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            projectId,
          },
          body: JSON.stringify(patch),
        })
      },
      async getUserConfig() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/user-center/profile/runtime-config', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async validateUserConfig(patch) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/user-center/profile/runtime-config/validate', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(patch),
        })
      },
      async saveUserConfig(patch) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/user-center/profile/runtime-config', 'patch', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(patch),
        })
      },
      async listSessions() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/sessions', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async createSession(input, idempotencyKey) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/sessions', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
          idempotencyKey,
        })
      },
      async loadSession(sessionId) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/sessions/{sessionId}', 'get', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            sessionId,
          },
        })
      },
      async deleteSession(sessionId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/sessions/{sessionId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            sessionId,
          },
        })
      },
      async pollEvents(sessionId, options = {}) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/runtime/sessions/{sessionId}/events',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              sessionId,
            },
            queryParams: {
              after: options.after,
            },
          },
        )
      },
      async subscribeEvents(sessionId, options) {
        return await openRuntimeSseStream(context, sessionId, options)
      },
      async submitUserTurn(sessionId, input, idempotencyKey) {
        const resolvedPermissionMode: RuntimePermissionMode = resolveRuntimePermissionMode(input.permissionMode)
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/sessions/{sessionId}/turns', 'post', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            sessionId,
          },
          body: JSON.stringify({
            ...input,
            permissionMode: resolvedPermissionMode,
          }),
          idempotencyKey,
        })
      },
      async resolveApproval(sessionId, approvalId, input, idempotencyKey) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/sessions/{sessionId}/approvals/{approvalId}', 'post', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            sessionId,
            approvalId,
          },
          body: JSON.stringify(input),
          idempotencyKey,
        })
      },
    },
  }
}
