import type { Workspace } from '@octopus/schema'

export function createWorkspaceDashboardTarget(workspaceId: string, projectId?: string) {
  return {
    name: 'dashboard',
    params: { workspaceId },
    query: projectId ? { project: projectId } : undefined,
  } as const
}

export function createWorkspaceSwitchTarget(workspaces: Workspace[], workspaceId: string) {
  const workspace = workspaces.find((item) => item.id === workspaceId)
  return createWorkspaceDashboardTarget(workspaceId, workspace?.projectIds[0])
}
