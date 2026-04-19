import type { ProjectRecord, WorkspaceSummary } from '@octopus/schema'

export type ProjectModulePermissionKey = 'agents' | 'resources' | 'tools' | 'knowledge' | 'tasks'

const PROJECT_DELETION_REVIEW_ROLE_CODES = ['system.owner', 'system.admin'] as const
const PROJECT_DELETION_REVIEW_PERMISSION_CODES = ['project.manage'] as const

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

export function canReviewProjectDeletion(
  permissionCodes: readonly string[] = [],
  roleCodes: readonly string[] = [],
): boolean {
  const permissionCodeSet = new Set(permissionCodes)
  if (PROJECT_DELETION_REVIEW_PERMISSION_CODES.some(code => permissionCodeSet.has(code))) {
    return true
  }

  const roleCodeSet = new Set(roleCodes)
  return PROJECT_DELETION_REVIEW_ROLE_CODES.some(code => roleCodeSet.has(code))
}

export function canAccessProjectSettings(
  project?: Pick<ProjectRecord, 'ownerUserId'> | null,
  actorUserId?: string | null,
  permissionCodes: readonly string[] = [],
  roleCodes: readonly string[] = [],
): boolean {
  return isProjectOwner(project, actorUserId) || canReviewProjectDeletion(permissionCodes, roleCodes)
}

export function canShowProjectInShell(
  project?: Pick<ProjectRecord, 'ownerUserId' | 'memberUserIds'> | null,
  actorUserId?: string | null,
  permissionCodes: readonly string[] = [],
  roleCodes: readonly string[] = [],
): boolean {
  return isProjectMember(project, actorUserId)
    || canAccessProjectSettings(project, actorUserId, permissionCodes, roleCodes)
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
    case 'project-tasks':
      return 'tasks'
    default:
      return null
  }
}
