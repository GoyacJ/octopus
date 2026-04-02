import { invoke } from '@tauri-apps/api/core'

import type {
  ConnectionProfile,
  DesktopBackendConnection,
  HealthcheckStatus,
  HostState,
  RuntimeBootstrap,
  RuntimeDecisionAction,
  RuntimeRunSnapshot,
  RuntimeSessionDetail,
  ShellBootstrap,
  ShellPreferences,
} from '@octopus/schema'

const PREFERENCES_STORAGE_KEY = 'octopus-shell-preferences'
const DEFAULT_BACKEND_BASE_URL = 'http://127.0.0.1:43127'

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function createDefaultPreferences(defaultWorkspaceId: string, defaultProjectId: string): ShellPreferences {
  return {
    theme: 'system',
    locale: 'zh-CN',
    compactSidebar: false,
    leftSidebarCollapsed: false,
    rightSidebarCollapsed: false,
    defaultWorkspaceId,
    lastVisitedRoute: `/workspaces/${defaultWorkspaceId}/overview?project=${defaultProjectId}`,
  }
}

function extractProjectIdFromRoute(lastVisitedRoute: string): string {
  if (lastVisitedRoute.includes('?project=')) {
    return lastVisitedRoute.split('?project=')[1]?.split('&')[0] ?? 'proj-redesign'
  }

  const projectMatch = lastVisitedRoute.match(/\/projects\/([^/]+)/)
  return projectMatch?.[1] ?? 'proj-redesign'
}

function normalizePreferences(
  value: Partial<ShellPreferences>,
  defaultWorkspaceId: string,
  defaultProjectId: string,
): ShellPreferences {
  const defaults = createDefaultPreferences(defaultWorkspaceId, defaultProjectId)
  const leftSidebarCollapsed = typeof value.leftSidebarCollapsed === 'boolean'
    ? value.leftSidebarCollapsed
    : Boolean(value.compactSidebar)

  return {
    ...defaults,
    ...value,
    compactSidebar: typeof value.compactSidebar === 'boolean' ? value.compactSidebar : leftSidebarCollapsed,
    leftSidebarCollapsed,
    rightSidebarCollapsed: typeof value.rightSidebarCollapsed === 'boolean' ? value.rightSidebarCollapsed : defaults.rightSidebarCollapsed,
  }
}

function fallbackHostState(): HostState {
  return {
    platform: 'web',
    mode: 'local',
    appVersion: '0.1.0',
    cargoWorkspace: false,
    shell: 'browser',
  }
}

function fallbackBackendConnection(ready = true, transport = 'mock'): DesktopBackendConnection {
  return {
    baseUrl: ready ? DEFAULT_BACKEND_BASE_URL : undefined,
    authToken: ready ? 'desktop-mock-token' : undefined,
    ready,
    transport,
  }
}

function loadStoredPreferences(defaultWorkspaceId: string, defaultProjectId: string): ShellPreferences {
  if (typeof window === 'undefined' || !window.localStorage) {
    return createDefaultPreferences(defaultWorkspaceId, defaultProjectId)
  }

  const raw = window.localStorage.getItem(PREFERENCES_STORAGE_KEY)
  if (!raw) {
    return createDefaultPreferences(defaultWorkspaceId, defaultProjectId)
  }

  try {
    return normalizePreferences(JSON.parse(raw) as Partial<ShellPreferences>, defaultWorkspaceId, defaultProjectId)
  } catch {
    return createDefaultPreferences(defaultWorkspaceId, defaultProjectId)
  }
}

function saveStoredPreferences(preferences: ShellPreferences): void {
  if (typeof window === 'undefined' || !window.localStorage) {
    return
  }

  window.localStorage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(preferences))
}

async function fetchBackend<T>(backend: DesktopBackendConnection | undefined, path: string, init?: RequestInit): Promise<T> {
  if (!backend?.ready || !backend.baseUrl) {
    throw new Error('Desktop backend is unavailable')
  }

  const headers = new Headers(init?.headers)
  if (backend.authToken) {
    headers.set('Authorization', `Bearer ${backend.authToken}`)
  }
  if (!headers.has('Content-Type') && init?.body) {
    headers.set('Content-Type', 'application/json')
  }

  const response = await fetch(`${backend.baseUrl}${path}`, {
    ...init,
    headers,
  })

  if (!response.ok) {
    throw new Error(`Desktop backend request failed: ${response.status}`)
  }

  return await response.json() as T
}

function buildMockRuntimeBootstrap(): RuntimeBootstrap {
  return {
    provider: {
      provider: 'anthropic',
      defaultModel: 'claude-sonnet-4-5',
    },
    sessions: [],
  }
}

function createMockSessionSummary(conversationId: string, projectId: string, title: string, timestamp: number) {
  return {
    id: `runtime-session-${conversationId}`,
    conversationId,
    projectId,
    title,
    status: 'idle',
    updatedAt: timestamp,
    lastMessagePreview: undefined,
  }
}

function createMockRuntimeSessionDetail(conversationId: string, projectId: string, title: string, timestamp: number): RuntimeSessionDetail {
  const summary = createMockSessionSummary(conversationId, projectId, title, timestamp)
  return {
    summary,
    run: {
      id: `runtime-run-${conversationId}`,
      sessionId: summary.id,
      conversationId,
      status: 'idle',
      currentStep: 'runtime.run.idle',
      startedAt: timestamp,
      updatedAt: timestamp,
      modelId: undefined,
      nextAction: 'runtime.run.awaitingInput',
    },
    messages: [],
    trace: [],
  }
}

function readMockRuntimeState(): Record<string, RuntimeSessionDetail> {
  if (typeof window === 'undefined') {
    return {}
  }

  return (window as typeof window & { __octopusRuntimeMock__?: Record<string, RuntimeSessionDetail> }).__octopusRuntimeMock__ ?? {}
}

function writeMockRuntimeState(state: Record<string, RuntimeSessionDetail>) {
  if (typeof window === 'undefined') {
    return
  }

  ;(window as typeof window & { __octopusRuntimeMock__?: Record<string, RuntimeSessionDetail> }).__octopusRuntimeMock__ = state
}

function upsertMockSession(detail: RuntimeSessionDetail): RuntimeSessionDetail {
  const state = readMockRuntimeState()
  state[detail.summary.id] = detail
  writeMockRuntimeState(state)
  return detail
}

function loadMockSession(sessionId: string): RuntimeSessionDetail {
  const state = readMockRuntimeState()
  const detail = state[sessionId]
  if (!detail) {
    throw new Error('Mock runtime session not found')
  }
  return detail
}

function listMockSessions() {
  return Object.values(readMockRuntimeState()).map((detail) => detail.summary)
}

export async function bootstrapShellHost(
  defaultWorkspaceId: string,
  defaultProjectId: string,
  mockConnections: ConnectionProfile[],
): Promise<ShellBootstrap> {
  const fallbackPreferences = loadStoredPreferences(defaultWorkspaceId, defaultProjectId)
  if (!isTauriRuntime()) {
    return {
      hostState: fallbackHostState(),
      preferences: fallbackPreferences,
      connections: mockConnections,
      backend: fallbackBackendConnection(),
    }
  }

  try {
    const bootstrap = await invoke<ShellBootstrap>('bootstrap_shell')
    const preferences = bootstrap.preferences
      ? normalizePreferences(bootstrap.preferences, defaultWorkspaceId, defaultProjectId)
      : fallbackPreferences
    saveStoredPreferences(preferences)

    return {
      hostState: bootstrap.hostState ?? fallbackHostState(),
      preferences,
      connections: bootstrap.connections ?? mockConnections,
      backend: bootstrap.backend ?? fallbackBackendConnection(),
    }
  } catch {
    return {
      hostState: fallbackHostState(),
      preferences: fallbackPreferences,
      connections: mockConnections,
      backend: fallbackBackendConnection(),
    }
  }
}

export async function loadPreferences(defaultWorkspaceId: string, defaultProjectId: string): Promise<ShellPreferences> {
  const fallbackPreferences = loadStoredPreferences(defaultWorkspaceId, defaultProjectId)
  if (!isTauriRuntime()) {
    return fallbackPreferences
  }

  try {
    const preferences = normalizePreferences(await invoke<ShellPreferences>('load_preferences'), defaultWorkspaceId, defaultProjectId)
    saveStoredPreferences(preferences)
    return preferences
  } catch {
    return fallbackPreferences
  }
}

export async function savePreferences(preferences: ShellPreferences): Promise<ShellPreferences> {
  const normalizedPreferences = normalizePreferences(
    {
      ...preferences,
      compactSidebar: preferences.leftSidebarCollapsed,
    },
    preferences.defaultWorkspaceId,
    extractProjectIdFromRoute(preferences.lastVisitedRoute),
  )
  saveStoredPreferences(normalizedPreferences)
  if (!isTauriRuntime()) {
    return normalizedPreferences
  }

  try {
    const savedPreferences = normalizePreferences(
      await invoke<ShellPreferences>('save_preferences', { preferences: normalizedPreferences }),
      normalizedPreferences.defaultWorkspaceId,
      extractProjectIdFromRoute(normalizedPreferences.lastVisitedRoute),
    )
    saveStoredPreferences(savedPreferences)
    return savedPreferences
  } catch {
    return normalizedPreferences
  }
}

export async function getHostState(): Promise<HostState> {
  if (!isTauriRuntime()) {
    return fallbackHostState()
  }

  try {
    return await invoke<HostState>('get_host_state')
  } catch {
    return fallbackHostState()
  }
}

export async function listConnectionsStub(): Promise<ConnectionProfile[]> {
  if (!isTauriRuntime()) {
    return []
  }

  try {
    return await invoke<ConnectionProfile[]>('list_connections_stub')
  } catch {
    return []
  }
}

export async function healthcheck(): Promise<HealthcheckStatus> {
  if (!isTauriRuntime()) {
    return {
      status: 'ok',
      host: 'web',
      mode: 'local',
      cargoWorkspace: false,
      backendReady: true,
      transport: 'mock',
    }
  }

  try {
    return await invoke<HealthcheckStatus>('healthcheck')
  } catch {
    return {
      status: 'ok',
      host: 'tauri',
      mode: 'local',
      cargoWorkspace: false,
      backendReady: false,
      transport: 'http',
    }
  }
}

export async function restartDesktopBackend(): Promise<void> {
  if (!isTauriRuntime()) {
    return
  }

  try {
    await invoke('restart_desktop_backend')
  } catch {
    return
  }
}

export async function bootstrapRuntime(): Promise<RuntimeBootstrap> {
  if (!isTauriRuntime()) {
    return buildMockRuntimeBootstrap()
  }

  const shellBootstrap = await invoke<ShellBootstrap>('bootstrap_shell')
  if (!shellBootstrap.backend?.ready) {
    throw new Error('Desktop backend is unavailable')
  }

  return await fetchBackend<RuntimeBootstrap>(shellBootstrap.backend, '/runtime/bootstrap', {
    method: 'GET',
  })
}

export async function createRuntimeSession(
  conversationId: string,
  projectId: string,
  title: string,
  _workingDir?: string,
): Promise<RuntimeSessionDetail> {
  if (!isTauriRuntime()) {
    return upsertMockSession(createMockRuntimeSessionDetail(conversationId, projectId, title, Date.now()))
  }

  const shellBootstrap = await invoke<ShellBootstrap>('bootstrap_shell')
  const backend = shellBootstrap.backend
  if (!backend?.ready) {
    throw new Error('Desktop backend is unavailable')
  }

  return await fetchBackend<RuntimeSessionDetail>(backend, '/runtime/sessions', {
    method: 'POST',
    body: JSON.stringify({ conversationId, projectId, title }),
  })
}

export async function loadRuntimeSession(sessionId: string): Promise<RuntimeSessionDetail> {
  if (!isTauriRuntime()) {
    return loadMockSession(sessionId)
  }

  const shellBootstrap = await invoke<ShellBootstrap>('bootstrap_shell')
  return await fetchBackend<RuntimeSessionDetail>(shellBootstrap.backend, `/runtime/sessions/${sessionId}`, {
    method: 'GET',
  })
}

export async function pollRuntimeEvents(sessionId: string) {
  if (!isTauriRuntime()) {
    return []
  }

  const shellBootstrap = await invoke<ShellBootstrap>('bootstrap_shell')
  return await fetchBackend<any[]>(shellBootstrap.backend, `/runtime/sessions/${sessionId}/events`, {
    method: 'GET',
  })
}

export async function submitRuntimeUserTurn(
  sessionId: string,
  content: string,
  modelId: string,
  permissionMode: string,
): Promise<RuntimeRunSnapshot> {
  if (!isTauriRuntime()) {
    const detail = loadMockSession(sessionId)
    const timestamp = Date.now()
    const waitingApproval = /\b(pwd|rm|delete|terminal|bash|shell)\b/i.test(content)

    const userMessage = {
      id: `runtime-message-user-${timestamp}`,
      sessionId,
      conversationId: detail.summary.conversationId,
      senderType: 'user' as const,
      senderLabel: 'You',
      content,
      timestamp,
      modelId,
      status: 'completed',
    }
    const assistantMessage = {
      id: `runtime-message-assistant-${timestamp}`,
      sessionId,
      conversationId: detail.summary.conversationId,
      senderType: 'assistant' as const,
      senderLabel: 'Octopus Runtime',
      content: waitingApproval ? '运行前需要审批。' : '已记录你的运行请求，并生成了运行摘要。',
      timestamp: timestamp + 1,
      modelId,
      status: waitingApproval ? 'waiting_approval' : 'completed',
    }
    const traceItem = {
      id: `runtime-trace-${timestamp}`,
      sessionId,
      runId: detail.run.id,
      conversationId: detail.summary.conversationId,
      kind: waitingApproval ? 'approval' : 'step',
      title: waitingApproval ? 'Requested approval for workspace terminal access' : 'Captured runtime execution step',
      detail: waitingApproval ? 'The runtime requested approval before executing a terminal command.' : `Processed a runtime turn with permission mode ${permissionMode}.`,
      tone: waitingApproval ? 'warning' : 'success',
      timestamp: timestamp + 2,
      actor: 'Octopus Runtime',
      relatedMessageId: assistantMessage.id,
      relatedToolName: waitingApproval ? 'terminal' : undefined,
    }

    const nextRun: RuntimeRunSnapshot = {
      ...detail.run,
      status: waitingApproval ? 'waiting_approval' : 'completed',
      currentStep: waitingApproval ? 'runtime.run.waitingApproval' : 'runtime.run.completed',
      updatedAt: timestamp + 2,
      startedAt: detail.run.startedAt || timestamp,
      modelId,
      nextAction: waitingApproval ? 'runtime.run.awaitingApproval' : 'runtime.run.idle',
    }

    const nextDetail: RuntimeSessionDetail = {
      ...detail,
      summary: {
        ...detail.summary,
        status: nextRun.status,
        updatedAt: timestamp + 2,
        lastMessagePreview: content,
      },
      run: nextRun,
      messages: [...detail.messages, userMessage, assistantMessage],
      trace: [...detail.trace, traceItem],
      pendingApproval: waitingApproval
        ? {
            id: `runtime-approval-${timestamp}`,
            sessionId,
            conversationId: detail.summary.conversationId,
            runId: detail.run.id,
            toolName: 'terminal',
            summary: 'Workspace terminal access requested',
            detail: content,
            riskLevel: 'medium',
            createdAt: timestamp + 1,
          }
        : undefined,
    }

    upsertMockSession(nextDetail)
    return nextRun
  }

  const shellBootstrap = await invoke<ShellBootstrap>('bootstrap_shell')
  return await fetchBackend<RuntimeRunSnapshot>(shellBootstrap.backend, `/runtime/sessions/${sessionId}/turns`, {
    method: 'POST',
    body: JSON.stringify({ content, modelId, permissionMode }),
  })
}

export async function resolveRuntimeApproval(
  sessionId: string,
  approvalId: string,
  decision: RuntimeDecisionAction,
): Promise<void> {
  if (!isTauriRuntime()) {
    const detail = loadMockSession(sessionId)
    const timestamp = Date.now()
    const nextDetail: RuntimeSessionDetail = {
      ...detail,
      summary: {
        ...detail.summary,
        status: decision === 'approve' ? 'completed' : 'blocked',
        updatedAt: timestamp,
      },
      run: {
        ...detail.run,
        status: decision === 'approve' ? 'completed' : 'blocked',
        currentStep: decision === 'approve' ? 'runtime.run.resuming' : 'runtime.run.blocked',
        updatedAt: timestamp,
        nextAction: decision === 'approve' ? 'runtime.run.idle' : 'runtime.run.manualRecovery',
      },
      trace: [
        ...detail.trace,
        {
          id: `runtime-trace-approval-${timestamp}`,
          sessionId,
          runId: detail.run.id,
          conversationId: detail.summary.conversationId,
          kind: 'approval',
          title: decision === 'approve' ? 'Approval resolved and run resumed' : 'Approval rejected and run blocked',
          detail: `Approval ${approvalId} was ${decision}.`,
          tone: decision === 'approve' ? 'success' : 'warning',
          timestamp,
          actor: 'Octopus Runtime',
          relatedToolName: 'terminal',
        },
      ],
      pendingApproval: undefined,
    }
    upsertMockSession(nextDetail)
    return
  }

  const shellBootstrap = await invoke<ShellBootstrap>('bootstrap_shell')
  await fetchBackend(shellBootstrap.backend, `/runtime/sessions/${sessionId}/approvals/${approvalId}`, {
    method: 'POST',
    body: JSON.stringify({ decision }),
  })
}

export async function listRuntimeSessions(): Promise<RuntimeSessionDetail['summary'][]> {
  if (!isTauriRuntime()) {
    return listMockSessions()
  }

  const shellBootstrap = await invoke<ShellBootstrap>('bootstrap_shell')
  return await fetchBackend(shellBootstrap.backend, '/runtime/sessions', {
    method: 'GET',
  })
}
