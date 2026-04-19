import type { WorkspaceToolPermissionMode } from './shared'

export interface ProjectModelSettings {
  // Forward-compatible delta field for the governance rebuild.
  disabledConfiguredModelIds?: string[]
  allowedConfiguredModelIds: string[]
  defaultConfiguredModelId: string
  totalTokens?: number
}

export interface ProjectToolPermissionOverride {
  permissionMode: WorkspaceToolPermissionMode
}

export interface ProjectToolSettings {
  disabledSourceKeys: string[]
  overrides: Record<string, ProjectToolPermissionOverride>
}

export interface ProjectAgentSettings {
  disabledAgentIds: string[]
  disabledTeamIds: string[]
}

export interface ProjectSettingsConfig {
  models?: ProjectModelSettings
  tools?: ProjectToolSettings
  agents?: ProjectAgentSettings
}
