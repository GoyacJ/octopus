import type {
  MenuSource,
  MenuStatus,
  PasswordState,
  RbacPermissionKind,
  RbacPermissionStatus,
  RbacPermissionTargetType,
  RbacRoleStatus,
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
