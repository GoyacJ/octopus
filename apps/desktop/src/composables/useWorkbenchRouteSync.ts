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
      typeof route.query.mode === 'string' ? route.query.mode : '',
      typeof route.query.deliverable === 'string' ? route.query.deliverable : '',
      typeof route.query.version === 'string' ? route.query.version : '',
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
        conversationId,
        mode: typeof route.query.mode === 'string' ? route.query.mode : undefined,
        deliverable: typeof route.query.deliverable === 'string' ? route.query.deliverable : undefined,
        version: typeof route.query.version === 'string' ? route.query.version : undefined,
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
