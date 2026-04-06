import type {
  AgentRecord,
  AutomationRecord,
  CreateRuntimeSessionInput,
  KnowledgeRecord,
  LoginRequest,
  LoginResponse,
  MenuRecord,
  ModelCatalogSnapshot,
  PermissionRecord,
  ProjectDashboardSnapshot,
  ProjectRecord,
  ProviderCredentialRecord,
  RegisterWorkspaceOwnerRequest,
  RegisterWorkspaceOwnerResponse,
  ResolveRuntimeApprovalInput,
  RoleRecord,
  RuntimeBootstrap,
  RuntimeConfigPatch,
  RuntimeConfigValidationResult,
  RuntimeEventEnvelope,
  RuntimeEffectiveConfig,
  RuntimePermissionMode,
  RuntimeRunSnapshot,
  RuntimeSessionDetail,
  RuntimeSessionSummary,
  SubmitRuntimeTurnInput,
  SystemBootstrapStatus,
  TeamRecord,
  ToolRecord,
  UserCenterOverviewSnapshot,
  UserRecordSummary,
  WorkspaceConnectionRecord,
  WorkspaceOverviewSnapshot,
  WorkspaceResourceRecord,
  WorkspaceSessionTokenEnvelope,
} from '@octopus/schema'
import { resolveRuntimePermissionMode } from '@octopus/schema'

import {
  createWorkspaceHeaders,
  decodeApiError,
  fetchWorkspaceApi,
  joinBaseUrl,
} from './shared'

const API_BASE = '/api/v1'
const RUNTIME_API_BASE = `${API_BASE}/runtime`

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
    getDashboard: (projectId: string) => Promise<ProjectDashboardSnapshot>
  }
  resources: {
    listWorkspace: () => Promise<WorkspaceResourceRecord[]>
    listProject: (projectId: string) => Promise<WorkspaceResourceRecord[]>
  }
  knowledge: {
    listWorkspace: () => Promise<KnowledgeRecord[]>
    listProject: (projectId: string) => Promise<KnowledgeRecord[]>
  }
  agents: {
    list: () => Promise<AgentRecord[]>
    create: (record: AgentRecord) => Promise<AgentRecord>
    update: (agentId: string, record: AgentRecord) => Promise<AgentRecord>
    delete: (agentId: string) => Promise<void>
  }
  teams: {
    list: () => Promise<TeamRecord[]>
    create: (record: TeamRecord) => Promise<TeamRecord>
    update: (teamId: string, record: TeamRecord) => Promise<TeamRecord>
    delete: (teamId: string) => Promise<void>
  }
  catalog: {
    getSnapshot: () => Promise<ModelCatalogSnapshot>
    listModels: () => Promise<ModelCatalogSnapshot['models']>
    listProviderCredentials: () => Promise<ProviderCredentialRecord[]>
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
    createUser: (record: UserRecordSummary) => Promise<UserRecordSummary>
    updateUser: (userId: string, record: UserRecordSummary) => Promise<UserRecordSummary>
    listRoles: () => Promise<RoleRecord[]>
    createRole: (record: RoleRecord) => Promise<RoleRecord>
    updateRole: (roleId: string, record: RoleRecord) => Promise<RoleRecord>
    listPermissions: () => Promise<PermissionRecord[]>
    createPermission: (record: PermissionRecord) => Promise<PermissionRecord>
    updatePermission: (permissionId: string, record: PermissionRecord) => Promise<PermissionRecord>
    listMenus: () => Promise<MenuRecord[]>
    createMenu: (record: MenuRecord) => Promise<MenuRecord>
    updateMenu: (menuId: string, record: MenuRecord) => Promise<MenuRecord>
  }
  runtime: {
    bootstrap: () => Promise<RuntimeBootstrap>
    getConfig: () => Promise<RuntimeEffectiveConfig>
    validateConfig: (patch: RuntimeConfigPatch) => Promise<RuntimeConfigValidationResult>
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
  const params = new URLSearchParams()
  if (options.lastEventId) {
    params.set('after', options.lastEventId)
  }
  const suffix = params.size ? `?${params.toString()}` : ''
  const controller = new AbortController()
  const headers = createWorkspaceHeaders({
    session,
    workspace: context.connection,
    headers: {
      Accept: 'text/event-stream',
      ...(options.lastEventId ? { 'Last-Event-ID': options.lastEventId } : {}),
    },
  })

  const response = await fetch(joinBaseUrl(context.connection.baseUrl, `${RUNTIME_API_BASE}/sessions/${sessionId}/events${suffix}`), {
    method: 'GET',
    headers,
    signal: controller.signal,
  })

  if (!response.ok) {
    throw await decodeApiError(
      response,
      headers.get('X-Request-Id') ?? 'req-unknown',
      context.connection.workspaceConnectionId,
    )
  }

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
        return await fetchWorkspaceApi<SystemBootstrapStatus>(
          context.connection,
          `${API_BASE}/system/bootstrap`,
          {
            method: 'GET',
            workspace: context.connection,
          },
        )
      },
    },
    auth: {
      async login(input) {
        return await fetchWorkspaceApi<LoginResponse>(
          context.connection,
          `${API_BASE}/auth/login`,
          {
            method: 'POST',
            body: JSON.stringify(input),
            workspace: context.connection,
          },
        )
      },
      async registerOwner(input) {
        return await fetchWorkspaceWithoutSession<RegisterWorkspaceOwnerResponse>(
          context,
          `${API_BASE}/auth/register-owner`,
          {
            method: 'POST',
            body: JSON.stringify(input),
          },
        )
      },
      async logout() {
        await fetchWorkspaceVoid(context, `${API_BASE}/auth/logout`, {
          method: 'POST',
        })
      },
      async session() {
        return await fetchWorkspace<WorkspaceSessionTokenEnvelope['session']>(context, `${API_BASE}/auth/session`, {
          method: 'GET',
        })
      },
    },
    workspace: {
      async get() {
        return await fetchWorkspace<WorkspaceOverviewSnapshot['workspace']>(context, `${API_BASE}/workspace`, {
          method: 'GET',
        })
      },
      async getOverview() {
        return await fetchWorkspace<WorkspaceOverviewSnapshot>(context, `${API_BASE}/workspace/overview`, {
          method: 'GET',
        })
      },
    },
    projects: {
      async list() {
        return await fetchWorkspace<ProjectRecord[]>(context, `${API_BASE}/projects`, {
          method: 'GET',
        })
      },
      async getDashboard(projectId) {
        return await fetchWorkspace<ProjectDashboardSnapshot>(
          context,
          `${API_BASE}/projects/${projectId}/dashboard`,
          {
            method: 'GET',
          },
        )
      },
    },
    resources: {
      async listWorkspace() {
        return await fetchWorkspace<WorkspaceResourceRecord[]>(context, `${API_BASE}/workspace/resources`, {
          method: 'GET',
        })
      },
      async listProject(projectId) {
        return await fetchWorkspace<WorkspaceResourceRecord[]>(
          context,
          `${API_BASE}/projects/${projectId}/resources`,
          {
            method: 'GET',
          },
        )
      },
    },
    knowledge: {
      async listWorkspace() {
        return await fetchWorkspace<KnowledgeRecord[]>(context, `${API_BASE}/workspace/knowledge`, {
          method: 'GET',
        })
      },
      async listProject(projectId) {
        return await fetchWorkspace<KnowledgeRecord[]>(
          context,
          `${API_BASE}/projects/${projectId}/knowledge`,
          {
            method: 'GET',
          },
        )
      },
    },
    agents: {
      async list() {
        return await fetchWorkspace<AgentRecord[]>(context, `${API_BASE}/workspace/agents`, {
          method: 'GET',
        })
      },
      async create(record) {
        return await fetchWorkspace<AgentRecord>(context, `${API_BASE}/workspace/agents`, {
          method: 'POST',
          body: JSON.stringify(record),
        })
      },
      async update(agentId, record) {
        return await fetchWorkspace<AgentRecord>(context, `${API_BASE}/workspace/agents/${agentId}`, {
          method: 'PATCH',
          body: JSON.stringify(record),
        })
      },
      async delete(agentId) {
        await fetchWorkspaceVoid(context, `${API_BASE}/workspace/agents/${agentId}`, {
          method: 'DELETE',
        })
      },
    },
    teams: {
      async list() {
        return await fetchWorkspace<TeamRecord[]>(context, `${API_BASE}/workspace/teams`, {
          method: 'GET',
        })
      },
      async create(record) {
        return await fetchWorkspace<TeamRecord>(context, `${API_BASE}/workspace/teams`, {
          method: 'POST',
          body: JSON.stringify(record),
        })
      },
      async update(teamId, record) {
        return await fetchWorkspace<TeamRecord>(context, `${API_BASE}/workspace/teams/${teamId}`, {
          method: 'PATCH',
          body: JSON.stringify(record),
        })
      },
      async delete(teamId) {
        await fetchWorkspaceVoid(context, `${API_BASE}/workspace/teams/${teamId}`, {
          method: 'DELETE',
        })
      },
    },
    catalog: {
      async getSnapshot() {
        return await fetchWorkspace<ModelCatalogSnapshot>(context, `${API_BASE}/workspace/catalog/models`, {
          method: 'GET',
        })
      },
      async listModels() {
        const snapshot = await this.getSnapshot()
        return snapshot.models
      },
      async listProviderCredentials() {
        return await fetchWorkspace<ProviderCredentialRecord[]>(
          context,
          `${API_BASE}/workspace/catalog/provider-credentials`,
          {
            method: 'GET',
          },
        )
      },
      async listTools() {
        return await fetchWorkspace<ToolRecord[]>(context, `${API_BASE}/workspace/catalog/tools`, {
          method: 'GET',
        })
      },
      async createTool(record) {
        return await fetchWorkspace<ToolRecord>(context, `${API_BASE}/workspace/catalog/tools`, {
          method: 'POST',
          body: JSON.stringify(record),
        })
      },
      async updateTool(toolId, record) {
        return await fetchWorkspace<ToolRecord>(context, `${API_BASE}/workspace/catalog/tools/${toolId}`, {
          method: 'PATCH',
          body: JSON.stringify(record),
        })
      },
      async deleteTool(toolId) {
        await fetchWorkspaceVoid(context, `${API_BASE}/workspace/catalog/tools/${toolId}`, {
          method: 'DELETE',
        })
      },
    },
    automations: {
      async list() {
        return await fetchWorkspace<AutomationRecord[]>(context, `${API_BASE}/workspace/automations`, {
          method: 'GET',
        })
      },
      async create(record) {
        return await fetchWorkspace<AutomationRecord>(context, `${API_BASE}/workspace/automations`, {
          method: 'POST',
          body: JSON.stringify(record),
        })
      },
      async update(automationId, record) {
        return await fetchWorkspace<AutomationRecord>(context, `${API_BASE}/workspace/automations/${automationId}`, {
          method: 'PATCH',
          body: JSON.stringify(record),
        })
      },
      async delete(automationId) {
        await fetchWorkspaceVoid(context, `${API_BASE}/workspace/automations/${automationId}`, {
          method: 'DELETE',
        })
      },
    },
    rbac: {
      async getUserCenterOverview() {
        return await fetchWorkspace<UserCenterOverviewSnapshot>(
          context,
          `${API_BASE}/workspace/user-center/overview`,
          {
            method: 'GET',
          },
        )
      },
      async listUsers() {
        return await fetchWorkspace<UserRecordSummary[]>(context, `${API_BASE}/workspace/rbac/users`, {
          method: 'GET',
        })
      },
      async createUser(record) {
        return await fetchWorkspace<UserRecordSummary>(context, `${API_BASE}/workspace/rbac/users`, {
          method: 'POST',
          body: JSON.stringify(record),
        })
      },
      async updateUser(userId, record) {
        return await fetchWorkspace<UserRecordSummary>(context, `${API_BASE}/workspace/rbac/users/${userId}`, {
          method: 'PATCH',
          body: JSON.stringify(record),
        })
      },
      async listRoles() {
        return await fetchWorkspace<RoleRecord[]>(context, `${API_BASE}/workspace/rbac/roles`, {
          method: 'GET',
        })
      },
      async createRole(record) {
        return await fetchWorkspace<RoleRecord>(context, `${API_BASE}/workspace/rbac/roles`, {
          method: 'POST',
          body: JSON.stringify(record),
        })
      },
      async updateRole(roleId, record) {
        return await fetchWorkspace<RoleRecord>(context, `${API_BASE}/workspace/rbac/roles/${roleId}`, {
          method: 'PATCH',
          body: JSON.stringify(record),
        })
      },
      async listPermissions() {
        return await fetchWorkspace<PermissionRecord[]>(
          context,
          `${API_BASE}/workspace/rbac/permissions`,
          {
            method: 'GET',
          },
        )
      },
      async createPermission(record) {
        return await fetchWorkspace<PermissionRecord>(
          context,
          `${API_BASE}/workspace/rbac/permissions`,
          {
            method: 'POST',
            body: JSON.stringify(record),
          },
        )
      },
      async updatePermission(permissionId, record) {
        return await fetchWorkspace<PermissionRecord>(
          context,
          `${API_BASE}/workspace/rbac/permissions/${permissionId}`,
          {
            method: 'PATCH',
            body: JSON.stringify(record),
          },
        )
      },
      async listMenus() {
        return await fetchWorkspace<MenuRecord[]>(context, `${API_BASE}/workspace/rbac/menus`, {
          method: 'GET',
        })
      },
      async createMenu(record) {
        return await fetchWorkspace<MenuRecord>(context, `${API_BASE}/workspace/rbac/menus`, {
          method: 'POST',
          body: JSON.stringify(record),
        })
      },
      async updateMenu(menuId, record) {
        return await fetchWorkspace<MenuRecord>(context, `${API_BASE}/workspace/rbac/menus/${menuId}`, {
          method: 'PATCH',
          body: JSON.stringify(record),
        })
      },
    },
    runtime: {
      async bootstrap() {
        return await fetchWorkspace<RuntimeBootstrap>(context, `${RUNTIME_API_BASE}/bootstrap`, {
          method: 'GET',
        })
      },
      async getConfig() {
        return await fetchWorkspaceWithoutSession<RuntimeEffectiveConfig>(context, `${RUNTIME_API_BASE}/config`, {
          method: 'GET',
        })
      },
      async validateConfig(patch) {
        return await fetchWorkspaceWithoutSession<RuntimeConfigValidationResult>(context, `${RUNTIME_API_BASE}/config/validate`, {
          method: 'POST',
          body: JSON.stringify(patch),
        })
      },
      async saveConfig(patch) {
        return await fetchWorkspaceWithoutSession<RuntimeEffectiveConfig>(context, `${RUNTIME_API_BASE}/config/scopes/workspace`, {
          method: 'PATCH',
          body: JSON.stringify(patch),
        })
      },
      async getProjectConfig(projectId) {
        return await fetchWorkspace<RuntimeEffectiveConfig>(context, `${API_BASE}/projects/${projectId}/runtime-config`, {
          method: 'GET',
        })
      },
      async validateProjectConfig(projectId, patch) {
        return await fetchWorkspace<RuntimeConfigValidationResult>(context, `${API_BASE}/projects/${projectId}/runtime-config/validate`, {
          method: 'POST',
          body: JSON.stringify(patch),
        })
      },
      async saveProjectConfig(projectId, patch) {
        return await fetchWorkspace<RuntimeEffectiveConfig>(context, `${API_BASE}/projects/${projectId}/runtime-config`, {
          method: 'PATCH',
          body: JSON.stringify(patch),
        })
      },
      async getUserConfig() {
        return await fetchWorkspace<RuntimeEffectiveConfig>(context, `${API_BASE}/workspace/user-center/profile/runtime-config`, {
          method: 'GET',
        })
      },
      async validateUserConfig(patch) {
        return await fetchWorkspace<RuntimeConfigValidationResult>(context, `${API_BASE}/workspace/user-center/profile/runtime-config/validate`, {
          method: 'POST',
          body: JSON.stringify(patch),
        })
      },
      async saveUserConfig(patch) {
        return await fetchWorkspace<RuntimeEffectiveConfig>(context, `${API_BASE}/workspace/user-center/profile/runtime-config`, {
          method: 'PATCH',
          body: JSON.stringify(patch),
        })
      },
      async listSessions() {
        return await fetchWorkspace<RuntimeSessionSummary[]>(context, `${RUNTIME_API_BASE}/sessions`, {
          method: 'GET',
        })
      },
      async createSession(input, idempotencyKey) {
        return await fetchWorkspace<RuntimeSessionDetail>(context, `${RUNTIME_API_BASE}/sessions`, {
          method: 'POST',
          body: JSON.stringify(input),
          idempotencyKey,
        })
      },
      async loadSession(sessionId) {
        return await fetchWorkspace<RuntimeSessionDetail>(context, `${RUNTIME_API_BASE}/sessions/${sessionId}`, {
          method: 'GET',
        })
      },
      async pollEvents(sessionId, options = {}) {
        const params = new URLSearchParams()
        if (options.after) {
          params.set('after', options.after)
        }
        const suffix = params.size ? `?${params.toString()}` : ''
        return await fetchWorkspace<RuntimeEventEnvelope[]>(
          context,
          `${RUNTIME_API_BASE}/sessions/${sessionId}/events${suffix}`,
          {
            method: 'GET',
          },
        )
      },
      async subscribeEvents(sessionId, options) {
        return await openRuntimeSseStream(context, sessionId, options)
      },
      async submitUserTurn(sessionId, input, idempotencyKey) {
        const resolvedPermissionMode: RuntimePermissionMode = resolveRuntimePermissionMode(input.permissionMode)
        return await fetchWorkspace<RuntimeRunSnapshot>(
          context,
          `${RUNTIME_API_BASE}/sessions/${sessionId}/turns`,
          {
            method: 'POST',
            body: JSON.stringify({
              ...input,
              permissionMode: resolvedPermissionMode,
            }),
            idempotencyKey,
          },
        )
      },
      async resolveApproval(sessionId, approvalId, input, idempotencyKey) {
        await fetchWorkspaceVoid(context, `${RUNTIME_API_BASE}/sessions/${sessionId}/approvals/${approvalId}`, {
          method: 'POST',
          body: JSON.stringify(input),
          idempotencyKey,
        })
      },
    },
  }
}
