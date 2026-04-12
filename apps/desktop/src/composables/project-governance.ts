import type { ProjectRecord, WorkspaceSummary } from '@octopus/schema'

export type ProjectModulePermissionKey = 'agents' | 'resources' | 'tools' | 'knowledge'

export function resolveProjectActorUserId(
  sessionUserId?: string | null,
  fallbackUserId?: string | null,
): string {
  return sessionUserId?.trim() || fallbackUserId?.trim() || ''
}

export function isProjectMember(
  project?: Pick<ProjectRecord, 'memberUserIds'> | null,
  userId?: string | null,
): boolean {
  if (!project || !userId?.trim()) {
    return false
  }
  return project.memberUserIds.includes(userId.trim())
}

export function isProjectOwner(
  project?: Pick<ProjectRecord, 'ownerUserId'> | null,
  userId?: string | null,
): boolean {
  if (!project || !userId?.trim()) {
    return false
  }
  return project.ownerUserId === userId.trim()
}

export function resolveProjectModulePermission(
  workspace?: Pick<WorkspaceSummary, 'projectDefaultPermissions'> | null,
  project?: Pick<ProjectRecord, 'permissionOverrides'> | null,
  module?: ProjectModulePermissionKey | null,
): 'allow' | 'deny' {
  if (!module) {
    return 'allow'
  }

  const workspaceDefault = workspace?.projectDefaultPermissions?.[module] ?? 'allow'
  const projectOverride = project?.permissionOverrides?.[module] ?? 'inherit'

  return projectOverride === 'inherit' ? workspaceDefault : projectOverride
}

export function isProjectModuleAllowed(
  workspace?: Pick<WorkspaceSummary, 'projectDefaultPermissions'> | null,
  project?: Pick<ProjectRecord, 'permissionOverrides'> | null,
  module?: ProjectModulePermissionKey | null,
): boolean {
  return resolveProjectModulePermission(workspace, project, module) === 'allow'
}

export function projectModuleForRouteName(
  routeName?: string | null,
): ProjectModulePermissionKey | null {
  switch (routeName) {
    case 'project-agents':
      return 'agents'
    case 'project-resources':
      return 'resources'
    case 'project-knowledge':
      return 'knowledge'
    default:
      return null
  }
}

export function isProjectOwnerOnlyRoute(routeName?: string | null): boolean {
  return routeName === 'project-settings' || routeName === 'project-runtime'
}
