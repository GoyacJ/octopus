import { defineStore } from 'pinia'

import type { ConnectionProfile, HostState, ShellBootstrap, ShellPreferences } from '@octopus/schema'

import { bootstrapShellHost, savePreferences } from '@/tauri/client'

export type ConversationDetailFocus =
  | 'summary'
  | 'memories'
  | 'artifacts'
  | 'knowledge'
  | 'resources'
  | 'tools'
  | 'timeline'

interface RouteSyncState {
  detail?: string
  pane?: string
  artifact?: string
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
    compactSidebar: updatesLeftSidebar ? savedPreferences.compactSidebar : currentPreferences.compactSidebar,
    leftSidebarCollapsed: updatesLeftSidebar ? savedPreferences.leftSidebarCollapsed : currentPreferences.leftSidebarCollapsed,
    rightSidebarCollapsed: hasPreferencePatchKey(patch, 'rightSidebarCollapsed')
      ? savedPreferences.rightSidebarCollapsed
      : currentPreferences.rightSidebarCollapsed,
    defaultWorkspaceId: hasPreferencePatchKey(patch, 'defaultWorkspaceId')
      ? savedPreferences.defaultWorkspaceId
      : currentPreferences.defaultWorkspaceId,
    lastVisitedRoute: hasPreferencePatchKey(patch, 'lastVisitedRoute')
      ? savedPreferences.lastVisitedRoute
      : currentPreferences.lastVisitedRoute,
  }
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

function createFallbackHostState(): HostState {
  return {
    platform: 'web',
    mode: 'local',
    appVersion: '0.1.0',
    cargoWorkspace: false,
    shell: 'browser',
  }
}

function normalizeDetail(detail?: string, legacyPane?: string): ConversationDetailFocus {
  if (
    detail === 'summary'
    || detail === 'memories'
    || detail === 'artifacts'
    || detail === 'knowledge'
    || detail === 'resources'
    || detail === 'tools'
    || detail === 'timeline'
  ) {
    return detail
  }

  if (legacyPane === 'artifacts') {
    return 'artifacts'
  }

  if (legacyPane === 'inbox') {
    return 'timeline'
  }

  if (legacyPane === 'trace') {
    return 'timeline'
  }

  return 'summary'
}

export const useShellStore = defineStore('shell', {
  state: () => ({
    defaultWorkspaceId: 'ws-local',
    defaultProjectId: 'proj-redesign',
    detailFocus: 'summary' as ConversationDetailFocus,
    selectedArtifactId: '',
    leftSidebarCollapsed: false,
    rightSidebarCollapsed: false,
    searchOpen: false,
    bootstrapPayload: null as ShellBootstrap | null,
    preferencesState: null as ShellPreferences | null,
    loading: false,
    error: '',
  }),
  getters: {
    preferences(state): ShellPreferences {
      return state.preferencesState ?? createDefaultPreferences(state.defaultWorkspaceId, state.defaultProjectId)
    },
    hostState(state): HostState {
      return state.bootstrapPayload?.hostState ?? createFallbackHostState()
    },
    bootstrapConnections(state): ConnectionProfile[] {
      return state.bootstrapPayload?.connections ?? []
    },
  },
  actions: {
    applyShellPreferences(preferences: ShellPreferences) {
      const preserveLeftSidebar = this.preferencesState === null && this.leftSidebarCollapsed !== false
      const preserveRightSidebar = this.preferencesState === null && this.rightSidebarCollapsed !== false
      this.preferencesState = preferences
      this.leftSidebarCollapsed = preserveLeftSidebar ? this.leftSidebarCollapsed : preferences.leftSidebarCollapsed
      this.rightSidebarCollapsed = preserveRightSidebar ? this.rightSidebarCollapsed : preferences.rightSidebarCollapsed
    },
    async bootstrap(defaultWorkspaceId: string, defaultProjectId: string, mockConnections: ConnectionProfile[]) {
      this.defaultWorkspaceId = defaultWorkspaceId
      this.defaultProjectId = defaultProjectId
      this.loading = true
      this.error = ''

      try {
        const payload = await bootstrapShellHost(defaultWorkspaceId, defaultProjectId, mockConnections)
        this.bootstrapPayload = payload
        this.applyShellPreferences(payload.preferences)
      } catch (error) {
        this.error = error instanceof Error ? error.message : 'Failed to bootstrap shell host'
        this.bootstrapPayload = {
          hostState: createFallbackHostState(),
          preferences: createDefaultPreferences(defaultWorkspaceId, defaultProjectId),
          connections: mockConnections,
        }
        this.applyShellPreferences(this.bootstrapPayload.preferences)
      } finally {
        this.loading = false
      }
    },
    setDetailFocus(detail: ConversationDetailFocus) {
      this.detailFocus = detail
    },
    selectArtifact(artifactId?: string) {
      this.selectedArtifactId = artifactId ?? ''
    },
    hydrateArtifactSelection(artifactIds: string[]) {
      if (!artifactIds.length) {
        this.selectedArtifactId = ''
        return
      }

      if (!this.selectedArtifactId || !artifactIds.includes(this.selectedArtifactId)) {
        this.selectedArtifactId = artifactIds[0]
      }
    },
    syncFromRoute(routeState: RouteSyncState) {
      this.detailFocus = normalizeDetail(routeState.detail, routeState.pane)
      if (routeState.artifact) {
        this.selectedArtifactId = routeState.artifact
      }
    },
    setLeftSidebarCollapsed(collapsed: boolean) {
      this.leftSidebarCollapsed = collapsed
      void this.updatePreferences({
        leftSidebarCollapsed: collapsed,
      })
    },
    toggleLeftSidebar() {
      this.setLeftSidebarCollapsed(!this.leftSidebarCollapsed)
    },
    setRightSidebarCollapsed(collapsed: boolean) {
      this.rightSidebarCollapsed = collapsed
      void this.updatePreferences({
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

      const savedPreferences = await savePreferences(nextPreferences)
      this.applyShellPreferences(mergeSavedPreferences(this.preferences, savedPreferences, patch))
    },
    async persistLastRoute(route: string) {
      await this.updatePreferences({
        lastVisitedRoute: route,
      })
    },
  },
})
