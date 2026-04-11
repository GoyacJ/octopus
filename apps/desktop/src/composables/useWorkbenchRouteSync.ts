import { watch } from 'vue'
import { useRoute } from 'vue-router'

import { useAuthStore } from '@/stores/auth'
import { useUserProfileStore } from '@/stores/user-profile'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

export function useWorkbenchRouteSync(): void {
  const route = useRoute()
  const auth = useAuthStore()
  const runtime = useRuntimeStore()
  const shell = useShellStore()
  const userProfileStore = useUserProfileStore()
  const workspaceAccessControlStore = useWorkspaceAccessControlStore()
  const workspaceStore = useWorkspaceStore()

  watch(
    () => [
      route.fullPath,
      shell.activeWorkspaceConnectionId,
      shell.activeWorkspaceSession?.token ?? '',
      auth.isReady,
      auth.isAuthenticated,
    ],
    async () => {
      const workspaceId = typeof route.params.workspaceId === 'string' ? route.params.workspaceId : undefined
      const projectId = typeof route.params.projectId === 'string' ? route.params.projectId : undefined
      const conversationId = typeof route.params.conversationId === 'string' ? route.params.conversationId : undefined
      const overviewProjectId = route.name === 'workspace-overview' && typeof route.query.project === 'string'
        ? route.query.project
        : undefined

      if (workspaceId) {
        await shell.activateWorkspaceByWorkspaceId(workspaceId)
        runtime.syncWorkspaceScopeFromShell()
      }

      workspaceStore.syncRouteScope(workspaceId, projectId ?? overviewProjectId, conversationId)

      shell.syncFromRoute({
        detail: typeof route.query.detail === 'string' ? route.query.detail : undefined,
        pane: typeof route.query.pane === 'string' ? route.query.pane : undefined,
        artifact: typeof route.query.artifact === 'string' ? route.query.artifact : undefined,
      })

      if (shell.activeWorkspaceConnectionId) {
        if (auth.isReady && auth.isAuthenticated) {
          await workspaceStore.bootstrap(shell.activeWorkspaceConnectionId)
          await workspaceAccessControlStore.load(shell.activeWorkspaceConnectionId)
          await userProfileStore.load(shell.activeWorkspaceConnectionId)
          await runtime.bootstrap()
        }
      }

      if (auth.isReady && auth.isAuthenticated && projectId) {
        await workspaceStore.loadProjectDashboard(projectId, shell.activeWorkspaceConnectionId)
      } else if (auth.isReady && auth.isAuthenticated && overviewProjectId) {
        await workspaceStore.loadProjectDashboard(overviewProjectId, shell.activeWorkspaceConnectionId)
      }

      void shell.persistLastRoute(route.fullPath)
    },
    { immediate: true },
  )
}
