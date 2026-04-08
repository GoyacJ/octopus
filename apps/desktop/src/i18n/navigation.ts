import type { ProjectRecord, WorkspaceSummary } from '@octopus/schema'
import type { RouteLocationNamedRaw } from 'vue-router'

export function createWorkspaceOverviewTarget(workspaceId: string, projectId?: string) {
  return {
    name: 'workspace-overview',
    params: { workspaceId },
    query: projectId ? { project: projectId } : undefined,
  } as const
}

export function createWorkspaceProjectsTarget(workspaceId: string) {
  return {
    name: 'workspace-projects',
    params: { workspaceId },
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

export function createWorkspaceSwitchTarget(
  workspaces: Array<Pick<WorkspaceSummary, 'id' | 'defaultProjectId'>>,
  workspaceId: string,
) {
  const workspace = workspaces.find((item) => item.id === workspaceId)
  return createWorkspaceOverviewTarget(workspaceId, workspace?.defaultProjectId)
}

export const createWorkspaceDashboardTarget = createWorkspaceOverviewTarget

export function getProjectFirstConversationId(
  _projects: Pick<ProjectRecord, 'id'>[],
  _projectId: string,
): string | undefined {
  return undefined
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
  routeName: 'project-agents' | 'project-resources' | 'project-knowledge' | 'project-trace' | 'project-settings' | 'project-runtime',
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
