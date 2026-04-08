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

export interface ProjectModelAssignments {
  configuredModelIds: string[]
  defaultConfiguredModelId: string
}

export interface ProjectToolAssignments {
  sourceKeys: string[]
}

export interface ProjectAgentAssignments {
  agentIds: string[]
  teamIds: string[]
}

export interface ProjectWorkspaceAssignments {
  models?: ProjectModelAssignments
  tools?: ProjectToolAssignments
  agents?: ProjectAgentAssignments
}

export interface ProjectRecord {
  id: string
  workspaceId: string
  name: string
  status: 'active' | 'archived'
  description: string
  assignments?: ProjectWorkspaceAssignments
}

export interface CreateProjectRequest {
  name: string
  description: string
  assignments?: ProjectWorkspaceAssignments
}

export interface UpdateProjectRequest {
  name: string
  description: string
  status: 'active' | 'archived'
  assignments?: ProjectWorkspaceAssignments
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
