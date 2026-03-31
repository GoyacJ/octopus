import { defineStore } from 'pinia'

import type { ConnectionProfile, HostState, ShellBootstrap, ShellPreferences } from '@octopus/schema'

import { bootstrapShellHost, savePreferences } from '@/tauri/client'

export type ContextPaneTab = 'context' | 'artifacts' | 'inbox' | 'trace'

interface RouteSyncState {
  pane?: string
  artifact?: string
}

function createDefaultPreferences(defaultWorkspaceId: string, defaultProjectId: string): ShellPreferences {
  return {
    theme: 'system',
    locale: 'zh-CN',
    compactSidebar: false,
    defaultWorkspaceId,
    lastVisitedRoute: `/workspaces/${defaultWorkspaceId}/dashboard?project=${defaultProjectId}`,
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

function normalizePane(pane?: string): ContextPaneTab {
  if (pane === 'artifacts' || pane === 'inbox' || pane === 'trace') {
    return pane
  }

  return 'context'
}

export const useShellStore = defineStore('shell', {
  state: () => ({
    defaultWorkspaceId: 'ws-local',
    defaultProjectId: 'proj-redesign',
    contextPane: 'context' as ContextPaneTab,
    selectedArtifactId: '',
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
    async bootstrap(defaultWorkspaceId: string, defaultProjectId: string, mockConnections: ConnectionProfile[]) {
      this.defaultWorkspaceId = defaultWorkspaceId
      this.defaultProjectId = defaultProjectId
      this.loading = true
      this.error = ''

      try {
        const payload = await bootstrapShellHost(defaultWorkspaceId, defaultProjectId, mockConnections)
        this.bootstrapPayload = payload
        this.preferencesState = payload.preferences
      } catch (error) {
        this.error = error instanceof Error ? error.message : 'Failed to bootstrap shell host'
        this.bootstrapPayload = {
          hostState: createFallbackHostState(),
          preferences: createDefaultPreferences(defaultWorkspaceId, defaultProjectId),
          connections: mockConnections,
        }
        this.preferencesState = this.bootstrapPayload.preferences
      } finally {
        this.loading = false
      }
    },
    setContextPane(pane: ContextPaneTab) {
      this.contextPane = pane
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
      this.contextPane = normalizePane(routeState.pane)
      if (routeState.artifact) {
        this.selectedArtifactId = routeState.artifact
      }
    },
    async updatePreferences(patch: Partial<ShellPreferences>) {
      const nextPreferences = {
        ...this.preferences,
        ...patch,
      }

      this.preferencesState = await savePreferences(nextPreferences)
    },
    async persistLastRoute(route: string) {
      await this.updatePreferences({
        lastVisitedRoute: route,
      })
    },
  },
})
