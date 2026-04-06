import type { Project, Workspace } from '@octopus/schema'
import type { RouteLocationNamedRaw } from 'vue-router'

export function createWorkspaceOverviewTarget(workspaceId: string, projectId?: string) {
  return {
    name: 'workspace-overview',
    params: { workspaceId },
    query: projectId ? { project: projectId } : undefined,
  } as const
}

export function createProjectDashboardTarget(workspaceId: string, projectId: string): RouteLocationNamedRaw {
  return {
    name: 'project-dashboard',
    params: {
      workspaceId,
      projectId,
    },
  }
}

export function createWorkspaceSwitchTarget(workspaces: Workspace[], workspaceId: string) {
  const workspace = workspaces.find((item) => item.id === workspaceId)
  return createWorkspaceOverviewTarget(workspaceId, workspace?.projectIds[0])
}

export const createWorkspaceDashboardTarget = createWorkspaceOverviewTarget

export function getProjectFirstConversationId(projects: Pick<Project, 'id' | 'conversationIds'>[], projectId: string): string | undefined {
  return projects.find((item) => item.id === projectId)?.conversationIds[0]
}

export function createProjectConversationTarget(workspaceId: string, projectId: string, conversationId?: string | null): RouteLocationNamedRaw {
  if (conversationId) {
    return {
      name: 'project-conversation',
      params: {
        workspaceId,
        projectId,
        conversationId,
      },
    }
  }

  return {
    name: 'project-conversations',
    params: {
      workspaceId,
      projectId,
    },
  }
}

export function createProjectSurfaceTarget(
  routeName: 'project-agents' | 'project-resources' | 'project-knowledge' | 'project-trace',
  workspaceId: string,
  projectId: string,
): RouteLocationNamedRaw {
  return {
    name: routeName,
    params: {
      workspaceId,
      projectId,
    },
  }
}
