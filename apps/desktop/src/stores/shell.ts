import { defineStore } from 'pinia'

import type {
  ArtifactVersionReference,
  ConnectionProfile,
  ConversationWorkbenchMode,
  CreateHostWorkspaceConnectionInput,
  HostBackendConnection,
  HostState,
  ShellBootstrap,
  ShellPreferences,
  ShellRouteState,
  TransportSecurityLevel,
  WorkspaceConnectionRecord,
  WorkspaceSessionTokenEnvelope,
} from '@octopus/schema'
import {
  createDefaultShellPreferences,
  normalizeConversationWorkbenchMode,
  normalizeWorkbenchVersion,
} from '@octopus/schema'

import * as tauriClient from '@/tauri/client'
import {
  clearStoredWorkspaceSession,
  loadStoredWorkspaceSessions,
  saveStoredWorkspaceSession,
} from '@/tauri/shared'

function createInitialHostState(): HostState {
  return import.meta.env.VITE_HOST_RUNTIME === 'browser'
    ? {
        platform: 'web',
        mode: 'local',
        appVersion: '0.1.0',
        cargoWorkspace: false,
        shell: 'browser',
      }
    : {
        platform: 'tauri',
        mode: 'local',
        appVersion: '0.1.0',
        cargoWorkspace: false,
        shell: 'tauri2',
      }
}

const INITIAL_HOST_STATE = createInitialHostState()

function resolveTransportSecurity(mode: ConnectionProfile['mode']): TransportSecurityLevel {
  switch (mode) {
    case 'local':
      return 'loopback'
    case 'shared':
      return 'trusted'
    case 'remote':
      return 'public'
    default:
      return 'trusted'
  }
}

function resolveWorkspaceConnectionStatus(
  connection: ConnectionProfile,
  backend?: HostBackendConnection,
): WorkspaceConnectionRecord['status'] {
  if (connection.mode === 'local') {
    return backend?.state === 'ready' ? 'connected' : 'unreachable'
  }

  return connection.state === 'disconnected' ? 'disconnected' : 'connected'
}

function resolveWorkspaceConnectionBaseUrl(
  connection: ConnectionProfile,
  backend?: HostBackendConnection,
): string {
  return connection.baseUrl ?? backend?.baseUrl ?? 'http://127.0.0.1'
}

function deriveWorkspaceConnections(
  bootstrap: ShellBootstrap | null,
): WorkspaceConnectionRecord[] {
  if (bootstrap?.workspaceConnections?.length) {
    return bootstrap.workspaceConnections
  }

  return (bootstrap?.connections ?? []).map(connection => ({
    workspaceConnectionId: connection.id,
    workspaceId: connection.workspaceId,
    label: connection.label,
    baseUrl: resolveWorkspaceConnectionBaseUrl(connection, bootstrap?.backend),
    transportSecurity: resolveTransportSecurity(connection.mode),
    authMode: 'session-token',
    lastUsedAt: connection.lastSyncAt,
    status: resolveWorkspaceConnectionStatus(connection, bootstrap?.backend),
  }))
}

function resolveActiveWorkspaceConnectionId(
  connections: WorkspaceConnectionRecord[],
  workspaceId: string,
): string {
  return connections.find(connection => connection.workspaceId === workspaceId)?.workspaceConnectionId
    ?? connections[0]?.workspaceConnectionId
    ?? ''
}

function hasPreferencePatchKey<K extends keyof ShellPreferences>(
  patch: Partial<ShellPreferences>,
  key: K,
): boolean {
  return Object.prototype.hasOwnProperty.call(patch, key)
}

function mergeSavedPreferences(
  currentPreferences: ShellPreferences,
  savedPreferences: ShellPreferences,
  patch: Partial<ShellPreferences>,
): ShellPreferences {
  const updatesLeftSidebar = hasPreferencePatchKey(patch, 'leftSidebarCollapsed') || hasPreferencePatchKey(patch, 'compactSidebar')

  return {
    ...savedPreferences,
    theme: hasPreferencePatchKey(patch, 'theme') ? savedPreferences.theme : currentPreferences.theme,
    locale: hasPreferencePatchKey(patch, 'locale') ? savedPreferences.locale : currentPreferences.locale,
    fontSize: hasPreferencePatchKey(patch, 'fontSize') ? savedPreferences.fontSize : currentPreferences.fontSize,
    fontFamily: hasPreferencePatchKey(patch, 'fontFamily') ? savedPreferences.fontFamily : currentPreferences.fontFamily,
    fontStyle: hasPreferencePatchKey(patch, 'fontStyle') ? savedPreferences.fontStyle : currentPreferences.fontStyle,
    compactSidebar: updatesLeftSidebar ? savedPreferences.compactSidebar : currentPreferences.compactSidebar,
    leftSidebarCollapsed: updatesLeftSidebar ? savedPreferences.leftSidebarCollapsed : currentPreferences.leftSidebarCollapsed,
    rightSidebarCollapsed: hasPreferencePatchKey(patch, 'rightSidebarCollapsed')
      ? savedPreferences.rightSidebarCollapsed
      : currentPreferences.rightSidebarCollapsed,
    updateChannel: hasPreferencePatchKey(patch, 'updateChannel')
      ? savedPreferences.updateChannel
      : currentPreferences.updateChannel,
    defaultWorkspaceId: hasPreferencePatchKey(patch, 'defaultWorkspaceId')
      ? savedPreferences.defaultWorkspaceId
      : currentPreferences.defaultWorkspaceId,
    lastVisitedRoute: hasPreferencePatchKey(patch, 'lastVisitedRoute')
      ? savedPreferences.lastVisitedRoute
      : currentPreferences.lastVisitedRoute,
  }
}

function toShellErrorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback
}

function touchWorkspaceConnection(
  connection: WorkspaceConnectionRecord,
): WorkspaceConnectionRecord {
  return {
    ...connection,
    lastUsedAt: Date.now(),
  }
}

interface ConversationWorkbenchSelection {
  mode: ConversationWorkbenchMode
  modeLocked: boolean
  selectedDeliverableId: string
  selectedDeliverableVersion: number | null
}

function createConversationWorkbenchSelection(): ConversationWorkbenchSelection {
  return {
    mode: 'context',
    modeLocked: false,
    selectedDeliverableId: '',
    selectedDeliverableVersion: null,
  }
}

export const useShellStore = defineStore('shell', {
  state: () => ({
    defaultWorkspaceId: 'ws-local',
    defaultProjectId: 'proj-redesign',
    activeConversationId: '',
    workbenchMode: 'context' as ConversationWorkbenchMode,
    workbenchModeLocked: false,
    selectedDeliverableId: '',
    selectedDeliverableVersion: null as number | null,
    conversationWorkbenchSelections: {} as Record<string, ConversationWorkbenchSelection>,
    leftSidebarCollapsed: false,
    rightSidebarCollapsed: false,
    searchOpen: false,
    bootstrapPayload: null as ShellBootstrap | null,
    backendConnectionState: undefined as HostBackendConnection | undefined,
    preferencesState: null as ShellPreferences | null,
    workspaceConnectionsState: [] as WorkspaceConnectionRecord[],
    workspaceSessionsState: {} as Record<string, WorkspaceSessionTokenEnvelope>,
    activeWorkspaceConnectionId: '',
    loading: false,
    syncingBackend: false,
    restartingBackend: false,
    error: '',
  }),
  getters: {
    preferences(state): ShellPreferences {
      return state.preferencesState ?? createDefaultShellPreferences(state.defaultWorkspaceId, state.defaultProjectId)
    },
    hostState(state): HostState {
      return state.bootstrapPayload?.hostState ?? INITIAL_HOST_STATE
    },
    bootstrapConnections(state): ConnectionProfile[] {
      return state.bootstrapPayload?.connections ?? []
    },
    backendConnection(state): HostBackendConnection | undefined {
      return state.backendConnectionState
    },
    workspaceConnections(state): WorkspaceConnectionRecord[] {
      return state.workspaceConnectionsState
    },
    activeWorkspaceConnection(state): WorkspaceConnectionRecord | null {
      return state.workspaceConnectionsState.find(connection => connection.workspaceConnectionId === state.activeWorkspaceConnectionId) ?? null
    },
    activeWorkspaceSession(state): WorkspaceSessionTokenEnvelope | null {
      return state.activeWorkspaceConnectionId
        ? state.workspaceSessionsState[state.activeWorkspaceConnectionId] ?? null
        : null
    },
    canRestartBackend(): boolean {
      return this.hostState.platform === 'tauri'
    },
  },
  actions: {
    currentConversationWorkbenchKey(): string {
      if (!this.activeWorkspaceConnectionId || !this.activeConversationId) {
        return ''
      }

      return `${this.activeWorkspaceConnectionId}:${this.activeConversationId}`
    },
    applyConversationWorkbenchSelection(selection?: Partial<ConversationWorkbenchSelection> | null) {
      const next = selection ? {
        ...createConversationWorkbenchSelection(),
        ...selection,
      } : createConversationWorkbenchSelection()
      this.workbenchMode = next.mode
      this.workbenchModeLocked = next.modeLocked
      this.selectedDeliverableId = next.selectedDeliverableId
      this.selectedDeliverableVersion = next.selectedDeliverableVersion
    },
    persistConversationWorkbenchSelection() {
      const key = this.currentConversationWorkbenchKey()
      if (!key) {
        return
      }

      this.conversationWorkbenchSelections = {
        ...this.conversationWorkbenchSelections,
        [key]: {
          mode: this.workbenchMode,
          modeLocked: this.workbenchModeLocked,
          selectedDeliverableId: this.selectedDeliverableId,
          selectedDeliverableVersion: this.selectedDeliverableVersion,
        },
      }
    },
    syncConversationScope(conversationId?: string | null) {
      if (conversationId === this.activeConversationId) {
        return
      }

      this.persistConversationWorkbenchSelection()
      this.activeConversationId = conversationId ?? ''
      this.applyConversationWorkbenchSelection(
        this.conversationWorkbenchSelections[this.currentConversationWorkbenchKey()] ?? null,
      )
    },
    applyShellPreferences(preferences: ShellPreferences) {
      const preserveLeftSidebar = this.preferencesState === null && this.leftSidebarCollapsed !== false
      const preserveRightSidebar = this.preferencesState === null && this.rightSidebarCollapsed !== false
      this.preferencesState = preferences
      this.leftSidebarCollapsed = preserveLeftSidebar ? this.leftSidebarCollapsed : preferences.leftSidebarCollapsed
      this.rightSidebarCollapsed = preserveRightSidebar ? this.rightSidebarCollapsed : preferences.rightSidebarCollapsed
    },
    async bootstrap(defaultWorkspaceId: string, defaultProjectId: string) {
      this.defaultWorkspaceId = defaultWorkspaceId
      this.defaultProjectId = defaultProjectId
      this.loading = true
      this.error = ''

      try {
        const payload = await tauriClient.bootstrapShellHost(defaultWorkspaceId, defaultProjectId)
        this.bootstrapPayload = payload
        this.backendConnectionState = payload.backend
        this.applyShellPreferences(payload.preferences)
        this.workspaceSessionsState = loadStoredWorkspaceSessions()
        this.workspaceConnectionsState = deriveWorkspaceConnections(payload)
        this.activeWorkspaceConnectionId = resolveActiveWorkspaceConnectionId(this.workspaceConnectionsState, payload.preferences.defaultWorkspaceId)
      } catch (error) {
        this.error = toShellErrorMessage(error, 'Failed to bootstrap shell host')
        this.bootstrapPayload = null
        this.backendConnectionState = undefined
        this.applyShellPreferences(createDefaultShellPreferences(defaultWorkspaceId, defaultProjectId))
        this.workspaceSessionsState = loadStoredWorkspaceSessions()
        this.workspaceConnectionsState = []
        this.activeWorkspaceConnectionId = ''
      } finally {
        this.loading = false
      }
    },
    setWorkbenchMode(mode: ConversationWorkbenchMode) {
      this.workbenchMode = mode
      this.workbenchModeLocked = true
      this.persistConversationWorkbenchSelection()
    },
    selectDeliverable(deliverableId?: string, version?: number | null) {
      const nextDeliverableId = deliverableId ?? ''
      const sameDeliverable = Boolean(nextDeliverableId) && nextDeliverableId === this.selectedDeliverableId

      this.selectedDeliverableId = nextDeliverableId
      this.selectedDeliverableVersion = nextDeliverableId
        ? (version ?? (sameDeliverable ? this.selectedDeliverableVersion : null))
        : null
      this.workbenchModeLocked = true
      if (this.selectedDeliverableId) {
        this.workbenchMode = 'deliverable'
      } else if (this.workbenchMode === 'deliverable') {
        this.workbenchMode = 'context'
      }
      this.persistConversationWorkbenchSelection()
    },
    setSelectedDeliverableVersion(version?: number | null) {
      this.selectedDeliverableVersion = version ?? null
      this.workbenchModeLocked = true
      this.persistConversationWorkbenchSelection()
    },
    hydrateDeliverableSelection(deliverableRefs: ArtifactVersionReference[]) {
      if (!deliverableRefs.length) {
        this.selectedDeliverableId = ''
        this.selectedDeliverableVersion = null
        if (this.workbenchMode === 'deliverable') {
          this.workbenchMode = 'context'
        }
        this.persistConversationWorkbenchSelection()
        return
      }

      const selectedRef = deliverableRefs.find(ref => ref.artifactId === this.selectedDeliverableId)
      if (!selectedRef) {
        this.selectedDeliverableId = deliverableRefs[0].artifactId
        this.selectedDeliverableVersion = deliverableRefs[0].version
        if (this.workbenchMode === 'context' && !this.workbenchModeLocked) {
          this.workbenchMode = 'deliverable'
        }
      } else if (!this.selectedDeliverableVersion) {
        this.selectedDeliverableVersion = selectedRef.version
      }
      this.persistConversationWorkbenchSelection()
    },
    syncFromRoute(routeState: ShellRouteState) {
      this.syncConversationScope(routeState.conversationId)
      if (routeState.mode !== undefined) {
        this.workbenchMode = normalizeConversationWorkbenchMode(routeState.mode)
        this.workbenchModeLocked = true
      }
      this.selectedDeliverableId = routeState.deliverable ?? ''
      if (!routeState.deliverable) {
        this.selectedDeliverableVersion = null
      }
      this.workbenchModeLocked = true
      if (routeState.version !== undefined) {
        this.selectedDeliverableVersion = normalizeWorkbenchVersion(routeState.version)
        this.workbenchModeLocked = true
      }
      this.persistConversationWorkbenchSelection()
    },
    persistPreferencesLater(patch: Partial<ShellPreferences>) {
      void this.updatePreferences(patch).catch((error) => {
        this.error = toShellErrorMessage(error, 'Failed to save shell preferences')
      })
    },
    setLeftSidebarCollapsed(collapsed: boolean) {
      this.leftSidebarCollapsed = collapsed
      this.persistPreferencesLater({
        leftSidebarCollapsed: collapsed,
      })
    },
    toggleLeftSidebar() {
      this.setLeftSidebarCollapsed(!this.leftSidebarCollapsed)
    },
    setRightSidebarCollapsed(collapsed: boolean) {
      this.rightSidebarCollapsed = collapsed
      this.persistPreferencesLater({
        rightSidebarCollapsed: collapsed,
      })
    },
    toggleRightSidebar() {
      this.setRightSidebarCollapsed(!this.rightSidebarCollapsed)
    },
    openSearch() {
      this.searchOpen = true
    },
    closeSearch() {
      this.searchOpen = false
    },
    toggleSearch() {
      this.searchOpen = !this.searchOpen
    },
    async updatePreferences(patch: Partial<ShellPreferences>) {
      const nextPreferences = {
        ...this.preferences,
        ...patch,
        compactSidebar: typeof patch.leftSidebarCollapsed === 'boolean'
          ? patch.leftSidebarCollapsed
          : typeof patch.compactSidebar === 'boolean'
            ? patch.compactSidebar
            : this.preferences.leftSidebarCollapsed,
      }

      const savedPreferences = await tauriClient.savePreferences(nextPreferences)
      this.applyShellPreferences(mergeSavedPreferences(this.preferences, savedPreferences, patch))
    },
    async persistLastRoute(route: string) {
      await this.updatePreferences({
        lastVisitedRoute: route,
      })
    },
    async activateWorkspaceConnection(workspaceConnectionId: string) {
      const connection = this.workspaceConnections.find(item => item.workspaceConnectionId === workspaceConnectionId)
      if (!connection) {
        return
      }

      this.activeWorkspaceConnectionId = connection.workspaceConnectionId
      this.workspaceConnectionsState = this.workspaceConnectionsState.map((item) =>
        item.workspaceConnectionId === workspaceConnectionId ? touchWorkspaceConnection(item) : item,
      )
      if (this.preferences.defaultWorkspaceId !== connection.workspaceId) {
        await this.updatePreferences({
          defaultWorkspaceId: connection.workspaceId,
        })
      }
    },
    async activateWorkspaceByWorkspaceId(workspaceId: string) {
      const connection = this.workspaceConnections.find(item => item.workspaceId === workspaceId)
      if (!connection) {
        return
      }

      await this.activateWorkspaceConnection(connection.workspaceConnectionId)
    },
    setWorkspaceSession(session: WorkspaceSessionTokenEnvelope) {
      saveStoredWorkspaceSession(session)
      this.workspaceSessionsState = {
        ...this.workspaceSessionsState,
        [session.workspaceConnectionId]: session,
      }
    },
    clearWorkspaceSession(workspaceConnectionId: string) {
      clearStoredWorkspaceSession(workspaceConnectionId)
      const nextSessions = { ...this.workspaceSessionsState }
      delete nextSessions[workspaceConnectionId]
      this.workspaceSessionsState = nextSessions
    },
    async createWorkspaceConnection(input: CreateHostWorkspaceConnectionInput) {
      const created = await tauriClient.createWorkspaceConnection(input)
      const existingIndex = this.workspaceConnectionsState.findIndex(
        connection => connection.workspaceConnectionId === created.workspaceConnectionId,
      )
      const nextConnection = touchWorkspaceConnection(created)
      if (existingIndex >= 0) {
        const nextConnections = [...this.workspaceConnectionsState]
        nextConnections.splice(existingIndex, 1, nextConnection)
        this.workspaceConnectionsState = nextConnections
      } else {
        this.workspaceConnectionsState = [...this.workspaceConnectionsState, nextConnection]
      }
      return nextConnection
    },
    async deleteWorkspaceConnection(workspaceConnectionId: string) {
      await tauriClient.deleteWorkspaceConnection(workspaceConnectionId)
      const wasActive = this.activeWorkspaceConnectionId === workspaceConnectionId
      this.clearWorkspaceSession(workspaceConnectionId)
      this.workspaceConnectionsState = this.workspaceConnectionsState.filter(
        connection => connection.workspaceConnectionId !== workspaceConnectionId,
      )

      if (wasActive) {
        const fallbackConnection = this.workspaceConnectionsState.find(
          connection => connection.transportSecurity === 'loopback',
        ) ?? this.workspaceConnectionsState[0]
        this.activeWorkspaceConnectionId = fallbackConnection?.workspaceConnectionId ?? ''
        if (fallbackConnection && this.preferences.defaultWorkspaceId !== fallbackConnection.workspaceId) {
          await this.updatePreferences({
            defaultWorkspaceId: fallbackConnection.workspaceId,
          })
        }
        return fallbackConnection ?? null
      }

      return this.activeWorkspaceConnection
    },
    syncBackendConnection(nextConnection: HostBackendConnection | undefined) {
      this.backendConnectionState = nextConnection
      this.workspaceConnectionsState = this.workspaceConnectionsState.map((connection) => {
        if (connection.transportSecurity !== 'loopback') {
          return connection
        }

        return {
          ...connection,
          baseUrl: nextConnection?.baseUrl ?? connection.baseUrl,
          status: nextConnection?.state === 'ready' ? 'connected' : 'unreachable',
        }
      })
    },
    async refreshBackendStatus() {
      this.syncingBackend = true
      this.error = ''

      try {
        const status = await tauriClient.healthcheck()
        const nextConnection = this.backendConnection
          ? {
              ...this.backendConnection,
              state: status.backend.state,
              transport: status.backend.transport,
            }
          : {
              state: status.backend.state,
              transport: status.backend.transport,
            }
        this.syncBackendConnection(nextConnection)
      } catch (error) {
        this.error = error instanceof Error ? error.message : 'Failed to refresh backend status'
      } finally {
        this.syncingBackend = false
      }
    },
    async restartBackend() {
      this.restartingBackend = true
      this.error = ''

      try {
        await tauriClient.restartDesktopBackend()
        await this.refreshBackendStatus()
      } catch (error) {
        this.error = error instanceof Error ? error.message : 'Failed to restart desktop backend'
        throw error
      } finally {
        this.restartingBackend = false
      }
    },
  },
})
