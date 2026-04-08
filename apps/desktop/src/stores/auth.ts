import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  LoginRequest,
  RegisterWorkspaceOwnerRequest,
  SystemBootstrapStatus,
  WorkspaceConnectionRecord,
  WorkspaceSessionTokenEnvelope,
} from '@octopus/schema'

import * as tauriClient from '@/tauri/client'

import { isWorkspaceApiError } from '@/tauri/shared'
import { useShellStore } from './shell'

export type AuthMode = 'login' | 'register'
export type AuthReason = 'first-launch' | 'missing-session' | 'session-expired' | 'manual'

function toSessionEnvelope(
  workspaceConnectionId: string,
  session: WorkspaceSessionTokenEnvelope['session'],
  issuedAt = Date.now(),
): WorkspaceSessionTokenEnvelope {
  return {
    workspaceConnectionId,
    token: session.token,
    session,
    issuedAt,
    expiresAt: session.expiresAt,
  }
}

function fallbackBootstrapStatus(
  connection: Pick<WorkspaceConnectionRecord, 'transportSecurity' | 'authMode'>,
  status: SystemBootstrapStatus,
): SystemBootstrapStatus {
  return {
    ...status,
    transportSecurity: status.transportSecurity ?? connection.transportSecurity,
    authMode: status.authMode ?? connection.authMode,
  }
}

export const useAuthStore = defineStore('auth', () => {
  const shell = useShellStore()

  const bootstrapStatusByConnection = ref<Record<string, SystemBootstrapStatus>>({})
  const authenticatedByConnection = ref<Record<string, boolean>>({})
  const readyByConnection = ref<Record<string, boolean>>({})
  const bootstrappingByConnection = ref<Record<string, boolean>>({})

  const dialogOpen = ref(false)
  const mode = ref<AuthMode>('login')
  const reason = ref<AuthReason>('manual')
  const submitting = ref(false)
  const error = ref('')

  const activeConnectionId = computed(() => shell.activeWorkspaceConnectionId)
  const activeConnection = computed(() => shell.activeWorkspaceConnection)
  const bootstrapStatus = computed(() =>
    activeConnectionId.value ? bootstrapStatusByConnection.value[activeConnectionId.value] ?? null : null,
  )
  const isAuthenticated = computed(() =>
    activeConnectionId.value ? authenticatedByConnection.value[activeConnectionId.value] ?? false : false,
  )
  const isReady = computed(() =>
    activeConnectionId.value ? readyByConnection.value[activeConnectionId.value] ?? false : true,
  )
  const bootstrapping = computed(() =>
    activeConnectionId.value ? bootstrappingByConnection.value[activeConnectionId.value] ?? false : false,
  )

  function resolveConnection(workspaceConnectionId?: string): WorkspaceConnectionRecord | null {
    if (workspaceConnectionId) {
      return shell.workspaceConnections.find(connection => connection.workspaceConnectionId === workspaceConnectionId) ?? null
    }

    return activeConnection.value
  }

  function resolveMode(status: SystemBootstrapStatus | null): AuthMode {
    // null means we couldn't reach the server or haven't bootstrapped yet — treat as first launch
    if (!status) {
      return 'register'
    }

    return status.setupRequired || !status.ownerReady ? 'register' : 'login'
  }

  function openAuthDialog(nextMode: AuthMode, nextReason: AuthReason, workspaceConnectionId?: string) {
    if (workspaceConnectionId && workspaceConnectionId !== activeConnectionId.value) {
      return
    }

    mode.value = nextMode
    reason.value = nextReason
    dialogOpen.value = true
  }

  function closeAuthDialog(workspaceConnectionId?: string) {
    if (workspaceConnectionId && workspaceConnectionId !== activeConnectionId.value) {
      return
    }

    dialogOpen.value = false
    error.value = ''
  }

  function markReady(workspaceConnectionId: string, ready: boolean) {
    readyByConnection.value = {
      ...readyByConnection.value,
      [workspaceConnectionId]: ready,
    }
  }

  function markBootstrapping(workspaceConnectionId: string, loading: boolean) {
    bootstrappingByConnection.value = {
      ...bootstrappingByConnection.value,
      [workspaceConnectionId]: loading,
    }
  }

  function setAuthenticated(workspaceConnectionId: string, value: boolean) {
    authenticatedByConnection.value = {
      ...authenticatedByConnection.value,
      [workspaceConnectionId]: value,
    }
  }

  function storeBootstrapStatus(workspaceConnectionId: string, status: SystemBootstrapStatus) {
    bootstrapStatusByConnection.value = {
      ...bootstrapStatusByConnection.value,
      [workspaceConnectionId]: status,
    }
  }

  function getBootstrapStatus(workspaceConnectionId: string): SystemBootstrapStatus | null {
    return bootstrapStatusByConnection.value[workspaceConnectionId] ?? null
  }

  function applyUnauthenticatedState(
    workspaceConnectionId: string,
    nextReason: AuthReason,
    status = getBootstrapStatus(workspaceConnectionId),
  ) {
    shell.clearWorkspaceSession(workspaceConnectionId)
    setAuthenticated(workspaceConnectionId, false)
    markReady(workspaceConnectionId, true)
    openAuthDialog(resolveMode(status), nextReason, workspaceConnectionId)
  }

  async function bootstrapAuth(workspaceConnectionId?: string) {
    const connection = resolveConnection(workspaceConnectionId)
    if (!connection) {
      dialogOpen.value = false
      return
    }

    const connectionId = connection.workspaceConnectionId
    markBootstrapping(connectionId, true)
    markReady(connectionId, false)
    error.value = ''

    try {
      const publicClient = tauriClient.createWorkspaceClient({ connection })
      const status = await publicClient.system.bootstrap()
      storeBootstrapStatus(connectionId, status)

      const requiredMode = resolveMode(status)
      if (requiredMode === 'register') {
        applyUnauthenticatedState(connectionId, 'first-launch', status)
        return
      }

      const storedSession = shell.workspaceSessionsState[connectionId]
      if (!storedSession?.token) {
        applyUnauthenticatedState(connectionId, 'missing-session', status)
        return
      }

      const sessionClient = tauriClient.createWorkspaceClient({
        connection,
        session: storedSession,
      })
      const restoredSession = await sessionClient.auth.session()
      shell.setWorkspaceSession(toSessionEnvelope(connectionId, restoredSession, storedSession.issuedAt))
      setAuthenticated(connectionId, true)
      markReady(connectionId, true)
      closeAuthDialog(connectionId)
    } catch (cause) {
      if (isWorkspaceApiError(cause) && (cause.code === 'UNAUTHENTICATED' || cause.code === 'SESSION_EXPIRED')) {
        applyUnauthenticatedState(connectionId, 'session-expired')
        return
      }

      error.value = cause instanceof Error ? cause.message : 'Failed to bootstrap auth state'
      // Pass null status to ensure resolveMode returns 'register' for first launch
      applyUnauthenticatedState(connectionId, 'first-launch', null)
    } finally {
      markBootstrapping(connectionId, false)
    }
  }

  function requireLogin(nextReason: AuthReason = 'manual', workspaceConnectionId?: string) {
    const connection = resolveConnection(workspaceConnectionId)
    if (!connection) {
      return
    }

    openAuthDialog(resolveMode(getBootstrapStatus(connection.workspaceConnectionId)), nextReason, connection.workspaceConnectionId)
  }

  async function login(input: Omit<LoginRequest, 'clientAppId' | 'workspaceId'>, workspaceConnectionId?: string) {
    const connection = resolveConnection(workspaceConnectionId)
    if (!connection) {
      throw new Error('Active workspace connection is unavailable')
    }

    submitting.value = true
    error.value = ''
    try {
      const client = tauriClient.createWorkspaceClient({ connection })
      const response = await client.auth.login({
        clientAppId: 'octopus-desktop',
        workspaceId: connection.workspaceId,
        ...input,
      })
      shell.setWorkspaceSession(toSessionEnvelope(connection.workspaceConnectionId, response.session))
      setAuthenticated(connection.workspaceConnectionId, true)
      markReady(connection.workspaceConnectionId, true)
      storeBootstrapStatus(
        connection.workspaceConnectionId,
        fallbackBootstrapStatus(connection, {
          ...(getBootstrapStatus(connection.workspaceConnectionId) ?? {
            workspace: response.workspace,
            setupRequired: false,
            ownerReady: true,
            registeredApps: [],
            protocolVersion: 'unknown',
            apiBasePath: '/api/v1',
            transportSecurity: connection.transportSecurity,
            authMode: connection.authMode,
            capabilities: {
              polling: true,
              sse: true,
              idempotency: true,
              reconnect: true,
              eventReplay: true,
            },
          }),
          workspace: response.workspace,
          setupRequired: false,
          ownerReady: true,
        }),
      )
      closeAuthDialog(connection.workspaceConnectionId)
      return response
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : 'Failed to login'
      throw cause
    } finally {
      submitting.value = false
    }
  }

  async function registerOwner(
    input: Omit<RegisterWorkspaceOwnerRequest, 'clientAppId' | 'workspaceId'>,
    workspaceConnectionId?: string,
  ) {
    const connection = resolveConnection(workspaceConnectionId)
    if (!connection) {
      throw new Error('Active workspace connection is unavailable')
    }

    submitting.value = true
    error.value = ''
    try {
      const client = tauriClient.createWorkspaceClient({ connection })
      const response = await client.auth.registerOwner({
        clientAppId: 'octopus-desktop',
        workspaceId: connection.workspaceId,
        ...input,
      })
      shell.setWorkspaceSession(toSessionEnvelope(connection.workspaceConnectionId, response.session))
      setAuthenticated(connection.workspaceConnectionId, true)
      markReady(connection.workspaceConnectionId, true)
      storeBootstrapStatus(
        connection.workspaceConnectionId,
        fallbackBootstrapStatus(connection, {
          ...(getBootstrapStatus(connection.workspaceConnectionId) ?? {
            workspace: response.workspace,
            setupRequired: false,
            ownerReady: true,
            registeredApps: [],
            protocolVersion: 'unknown',
            apiBasePath: '/api/v1',
            transportSecurity: connection.transportSecurity,
            authMode: connection.authMode,
            capabilities: {
              polling: true,
              sse: true,
              idempotency: true,
              reconnect: true,
              eventReplay: true,
            },
          }),
          workspace: response.workspace,
          setupRequired: false,
          ownerReady: true,
        }),
      )
      closeAuthDialog(connection.workspaceConnectionId)
      return response
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : 'Failed to register workspace owner'
      throw cause
    } finally {
      submitting.value = false
    }
  }

  async function logout(workspaceConnectionId?: string) {
    const connection = resolveConnection(workspaceConnectionId)
    if (!connection) {
      return
    }

    const session = shell.workspaceSessionsState[connection.workspaceConnectionId]
    try {
      if (session?.token) {
        const client = tauriClient.createWorkspaceClient({ connection, session })
        await client.auth.logout()
      }
    } finally {
      shell.clearWorkspaceSession(connection.workspaceConnectionId)
      setAuthenticated(connection.workspaceConnectionId, false)
      markReady(connection.workspaceConnectionId, true)
      requireLogin('manual', connection.workspaceConnectionId)
    }
  }

  function handleAuthError(
    workspaceConnectionId = activeConnectionId.value,
    nextReason: AuthReason = 'session-expired',
  ) {
    if (!workspaceConnectionId) {
      return
    }

    applyUnauthenticatedState(workspaceConnectionId, nextReason)
  }

  function isConnectionAuthenticated(workspaceConnectionId?: string) {
    const connection = resolveConnection(workspaceConnectionId)
    if (!connection) {
      return false
    }

    return authenticatedByConnection.value[connection.workspaceConnectionId] ?? false
  }

  async function connectWorkspace(input: {
    baseUrl: string
    username: string
    password: string
  }) {
    submitting.value = true
    error.value = ''

    const normalizedBaseUrl = input.baseUrl.trim().replace(/\/+$/, '')
    const provisionalConnection: WorkspaceConnectionRecord = {
      workspaceConnectionId: `temp-${Date.now()}`,
      workspaceId: '',
      label: normalizedBaseUrl,
      baseUrl: normalizedBaseUrl,
      transportSecurity: normalizedBaseUrl.startsWith('http://127.0.0.1') || normalizedBaseUrl.startsWith('http://localhost')
        ? 'loopback'
        : normalizedBaseUrl.startsWith('https://')
          ? 'trusted'
          : 'public',
      authMode: 'session-token',
      status: 'connected',
    }

    try {
      const bootstrapClient = tauriClient.createWorkspaceClient({ connection: provisionalConnection })
      const status = await bootstrapClient.system.bootstrap()
      const authenticatedConnection: WorkspaceConnectionRecord = {
        ...provisionalConnection,
        workspaceId: status.workspace.id,
        label: status.workspace.name,
        transportSecurity: status.transportSecurity,
        authMode: status.authMode,
      }
      const loginClient = tauriClient.createWorkspaceClient({ connection: authenticatedConnection })
      const response = await loginClient.auth.login({
        clientAppId: 'octopus-desktop',
        workspaceId: status.workspace.id,
        username: input.username.trim(),
        password: input.password,
      })

      const persistedConnection = await shell.createWorkspaceConnection({
        workspaceId: status.workspace.id,
        label: status.workspace.name,
        baseUrl: normalizedBaseUrl,
        transportSecurity: status.transportSecurity,
        authMode: status.authMode,
      })
      shell.setWorkspaceSession(
        toSessionEnvelope(persistedConnection.workspaceConnectionId, response.session),
      )
      setAuthenticated(persistedConnection.workspaceConnectionId, true)
      markReady(persistedConnection.workspaceConnectionId, true)
      storeBootstrapStatus(
        persistedConnection.workspaceConnectionId,
        fallbackBootstrapStatus(persistedConnection, {
          ...status,
          workspace: response.workspace,
          setupRequired: false,
          ownerReady: true,
        }),
      )
      await shell.activateWorkspaceConnection(persistedConnection.workspaceConnectionId)
      return persistedConnection
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : 'Failed to connect workspace'
      throw cause
    } finally {
      submitting.value = false
    }
  }

  return {
    dialogOpen,
    mode,
    reason,
    error,
    submitting,
    bootstrapping,
    bootstrapStatus,
    isReady,
    isAuthenticated,
    bootstrapAuth,
    requireLogin,
    login,
    registerOwner,
    connectWorkspace,
    logout,
    handleAuthError,
    isConnectionAuthenticated,
  }
})
