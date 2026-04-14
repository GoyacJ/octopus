import { watch } from 'vue'
import { useRoute } from 'vue-router'

import { useAuthStore } from '@/stores/auth'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

export function useWorkbenchRouteSync(): void {
  const route = useRoute()
  const auth = useAuthStore()
  const runtime = useRuntimeStore()
  const shell = useShellStore()
  const workspaceAccessControlStore = useWorkspaceAccessControlStore()
  const workspaceStore = useWorkspaceStore()

  watch(
    () => [
      typeof route.name === 'string' ? route.name : '',
      typeof route.params.workspaceId === 'string' ? route.params.workspaceId : '',
      typeof route.params.projectId === 'string' ? route.params.projectId : '',
      typeof route.params.conversationId === 'string' ? route.params.conversationId : '',
      route.name === 'workspace-overview' && typeof route.query.project === 'string' ? route.query.project : '',
      typeof route.query.detail === 'string' ? route.query.detail : '',
      typeof route.query.pane === 'string' ? route.query.pane : '',
      typeof route.query.artifact === 'string' ? route.query.artifact : '',
      shell.activeWorkspaceConnectionId,
      shell.activeWorkspaceSession?.token ?? '',
      auth.isReady,
      auth.isAuthenticated,
    ],
    async (next, previous) => {
      const startedAt = performance.now()
      const previousToken = previous?.[9] ?? ''
      const previousAuthenticated = previous?.[11] ?? false
      const workspaceId = typeof route.params.workspaceId === 'string' ? route.params.workspaceId : undefined
      const projectId = typeof route.params.projectId === 'string' ? route.params.projectId : undefined
      const conversationId = typeof route.params.conversationId === 'string' ? route.params.conversationId : undefined
      const overviewProjectId = route.name === 'workspace-overview' && typeof route.query.project === 'string'
        ? route.query.project
        : undefined

      if (workspaceId && shell.activeWorkspaceConnection?.workspaceId !== workspaceId) {
        await shell.activateWorkspaceByWorkspaceId(workspaceId)
      }
      runtime.syncWorkspaceScopeFromShell()

      workspaceStore.syncRouteScope(workspaceId, projectId ?? overviewProjectId, conversationId)

      shell.syncFromRoute({
        detail: typeof route.query.detail === 'string' ? route.query.detail : undefined,
        pane: typeof route.query.pane === 'string' ? route.query.pane : undefined,
        artifact: typeof route.query.artifact === 'string' ? route.query.artifact : undefined,
      })

      if (shell.activeWorkspaceConnectionId) {
        if (auth.isReady && auth.isAuthenticated) {
          const force = previousToken !== next[9] || previousAuthenticated !== next[11]
          await workspaceStore.ensureWorkspaceBootstrap(shell.activeWorkspaceConnectionId, { force })
          await workspaceAccessControlStore.ensureAuthorizationContext(shell.activeWorkspaceConnectionId, { force })
        }
      }

      void shell.persistLastRoute(route.fullPath)
      if (import.meta.env.DEV) {
        console.debug(`[route-sync] ${route.fullPath} ${Math.round(performance.now() - startedAt)}ms`)
      }
    },
    { immediate: true },
  )
}
