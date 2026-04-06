export interface LoginRequest {
  clientAppId: string
  username: string
  password: string
  workspaceId?: string
}

export interface SessionRecord {
  id: string
  workspaceId: string
  userId: string
  clientAppId: string
  token: string
  status: 'active' | 'revoked' | 'expired'
  createdAt: number
  expiresAt?: number
  roleIds: string[]
  scopeProjectIds: string[]
}

export interface LoginResponse {
  session: SessionRecord
  workspace: import('./workspace').WorkspaceSummary
}
