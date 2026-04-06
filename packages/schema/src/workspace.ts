export interface WorkspaceSummary {
  id: string
  name: string
  slug: string
  deployment: 'local' | 'remote'
  bootstrapStatus: 'setup_required' | 'ready'
  ownerUserId?: string
  host: string
  listenAddress: string
  defaultProjectId: string
}

export interface ProjectRecord {
  id: string
  workspaceId: string
  name: string
  status: 'active' | 'archived'
  description: string
}

export interface SystemBootstrapStatus {
  workspace: WorkspaceSummary
  setupRequired: boolean
  ownerReady: boolean
  registeredApps: import('./app').ClientAppRecord[]
  protocolVersion: string
  apiBasePath: string
  transportSecurity: import('./workspace-protocol').TransportSecurityLevel
  authMode: import('./workspace-protocol').WorkspaceAuthMode
  capabilities: import('./workspace-protocol').WorkspaceCapabilitySet
}
