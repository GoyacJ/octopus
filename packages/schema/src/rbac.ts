import type {
  MenuSource,
  MenuStatus,
  PasswordState,
  RbacPermissionKind,
  RbacPermissionStatus,
  RbacPermissionTargetType,
  RbacRoleStatus,
  RiskLevel,
  UserGender,
  UserStatus,
  WorkspaceScopeMode,
} from './shared'

export interface Workspace {
  id: string
  name: string
  avatar?: string
  isLocal: boolean
  description: string
  roleSummary: string
  memberCount: number
  projectIds: string[]
}

export function createWorkspaceRecord(input: {
  id: string
  name: string
  avatar?: string
  isLocal?: boolean
  description?: string
  roleSummary?: string
  memberCount?: number
  projectIds?: string[]
}): Workspace {
  return {
    id: input.id,
    name: input.name,
    avatar: input.avatar,
    isLocal: input.isLocal ?? true,
    description: input.description ?? '',
    roleSummary: input.roleSummary ?? '',
    memberCount: input.memberCount ?? 1,
    projectIds: input.projectIds ?? [],
  }
}

export interface UserAccount {
  id: string
  username: string
  nickname: string
  gender: UserGender
  avatar: string
  phone: string
  email: string
  status: UserStatus
  passwordState: PasswordState
  passwordUpdatedAt: number
}

export interface WorkspaceMembership {
  workspaceId: string
  userId: string
  roleIds: string[]
  scopeMode: WorkspaceScopeMode
  scopeProjectIds: string[]
}

export interface RbacRole {
  id: string
  workspaceId: string
  name: string
  code: string
  description: string
  status: RbacRoleStatus
  permissionIds: string[]
  menuIds: string[]
}

export interface RbacPermission {
  id: string
  workspaceId: string
  name: string
  code: string
  description: string
  status: RbacPermissionStatus
  kind: RbacPermissionKind
  targetType?: RbacPermissionTargetType
  targetIds?: string[]
  action?: string
  memberPermissionIds?: string[]
}

export interface MenuNode {
  id: string
  workspaceId: string
  parentId?: string
  source: MenuSource
  label: string
  routeName?: string
  status: MenuStatus
  order: number
}

export interface PermissionCenterMetric {
  id: string
  label: string
  value: string
  helper?: string
  tone?: 'default' | 'success' | 'warning' | 'error' | 'info'
}

export interface PermissionCenterAlert {
  id: string
  title: string
  description: string
  severity: RiskLevel
  workspaceId?: string
  routeName?: string
  entityId?: string
}

export interface PermissionCenterQuickLink {
  id: string
  label: string
  helper: string
  routeName: string
}

export interface PermissionCenterOverview {
  workspaceId: string
  currentUserId: string
  metrics: PermissionCenterMetric[]
  alerts: PermissionCenterAlert[]
  quickLinks: PermissionCenterQuickLink[]
}

export interface UserPermissionGroupSummary {
  targetType: RbacPermissionTargetType
  permissions: Array<{
    id: string
    name: string
    code: string
    action?: string
    targetLabels: string[]
  }>
}

export interface PersonalCenterProfileSnapshot {
  userId: string
  roleNames: string[]
  scopeSummary: string
  permissionCount: number
  menuCount: number
  groups: UserPermissionGroupSummary[]
  recentActivity: Array<{
    id: string
    title: string
    description: string
    timestamp: number
  }>
}

export interface PermissionCenterUserListItem {
  id: string
  nickname: string
  username: string
  email: string
  avatar: string
  status: UserStatus
  roleNames: string[]
  roleSummary: string
  scopeSummary: string
  projectCount: number
  effectivePermissionCount: number
  effectiveMenuCount: number
  isCurrentUser: boolean
  lastActivityAt?: number
}

export interface PermissionCenterRoleListItem {
  id: string
  name: string
  code: string
  description: string
  status: RbacRoleStatus
  memberCount: number
  permissionCount: number
  menuCount: number
  riskFlags: string[]
}

export interface PermissionCenterPermissionListItem {
  id: string
  name: string
  code: string
  description: string
  status: RbacPermissionStatus
  kind: RbacPermissionKind
  targetType?: RbacPermissionTargetType
  targetSummary: string
  usedByRoleCount: number
  bundleMemberCount: number
  riskFlags: string[]
}

export interface PermissionCenterMenuTreeItem {
  id: string
  label: string
  routeName?: string
  source: MenuSource
  status: MenuStatus
  order: number
  parentId?: string
  parentLabel?: string
  depth: number
  roleUsageCount: number
}
