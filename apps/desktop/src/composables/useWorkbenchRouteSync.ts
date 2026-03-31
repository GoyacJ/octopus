import { watch } from 'vue'
import { useRoute } from 'vue-router'

import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

export function useWorkbenchRouteSync(): void {
  const route = useRoute()
  const workbench = useWorkbenchStore()
  const shell = useShellStore()

  watch(
    () => route.fullPath,
    () => {
      const workspaceId = typeof route.params.workspaceId === 'string' ? route.params.workspaceId : undefined
      const projectId = typeof route.params.projectId === 'string' ? route.params.projectId : undefined
      const conversationId = typeof route.params.conversationId === 'string' ? route.params.conversationId : undefined
      const dashboardProjectId = typeof route.query.project === 'string' ? route.query.project : undefined

      if (workspaceId) {
        workbench.selectWorkspace(workspaceId)
      }

      if (dashboardProjectId) {
        workbench.selectProject(dashboardProjectId)
      }

      if (projectId) {
        workbench.selectProject(projectId)
      }

      if (conversationId) {
        workbench.selectConversation(conversationId)
      }

      shell.syncFromRoute({
        pane: typeof route.query.pane === 'string' ? route.query.pane : undefined,
        artifact: typeof route.query.artifact === 'string' ? route.query.artifact : undefined,
      })
      shell.hydrateArtifactSelection(workbench.activeConversationArtifacts.map((artifact: { id: string }) => artifact.id))
      void shell.persistLastRoute(route.fullPath)
    },
    { immediate: true },
  )
}
