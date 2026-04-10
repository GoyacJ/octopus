import type { WorkspaceDirectoryUploadEntry } from './catalog'

export interface ImportWorkspaceAgentBundlePreviewInput {
  files: WorkspaceDirectoryUploadEntry[]
}

export interface ImportWorkspaceAgentBundleInput {
  files: WorkspaceDirectoryUploadEntry[]
}

export interface ImportIssue {
  severity: 'warning' | 'error'
  scope: 'bundle' | 'agent' | 'skill'
  sourceId?: string
  message: string
}

export interface ImportedAgentPreviewItem {
  sourceId: string
  agentId?: string
  name: string
  department: string
  action: 'create' | 'update' | 'skip' | 'failed'
  skillSlugs: string[]
  mcpServerNames: string[]
}

export interface ImportedTeamPreviewItem {
  sourceId: string
  teamId?: string
  name: string
  action: 'create' | 'update' | 'skip' | 'failed'
  leaderName?: string
  memberNames: string[]
  agentSourceIds: string[]
}

export interface ImportedSkillPreviewItem {
  slug: string
  skillId: string
  name: string
  action: 'create' | 'update' | 'skip' | 'failed'
  contentHash: string
  fileCount: number
  sourceIds: string[]
  departments: string[]
  agentNames: string[]
}

export interface ImportedMcpPreviewItem {
  serverName: string
  action: 'create' | 'update' | 'skip' | 'failed'
  contentHash?: string
  sourceIds: string[]
  consumerNames: string[]
  referencedOnly: boolean
}

export interface ImportedAvatarPreviewItem {
  sourceId: string
  ownerKind: 'agent' | 'team' | string
  ownerName: string
  fileName: string
  generated: boolean
}

export interface ImportWorkspaceAgentBundlePreview {
  departments: string[]
  departmentCount: number
  detectedAgentCount: number
  importableAgentCount: number
  detectedTeamCount: number
  importableTeamCount: number
  createCount: number
  updateCount: number
  skipCount: number
  failureCount: number
  uniqueSkillCount: number
  uniqueMcpCount: number
  avatarCount: number
  filteredFileCount: number
  agents: ImportedAgentPreviewItem[]
  teams: ImportedTeamPreviewItem[]
  skills: ImportedSkillPreviewItem[]
  mcps: ImportedMcpPreviewItem[]
  avatars: ImportedAvatarPreviewItem[]
  issues: ImportIssue[]
}

export interface ImportWorkspaceAgentBundleResult {
  departments: string[]
  departmentCount: number
  detectedAgentCount: number
  importableAgentCount: number
  detectedTeamCount: number
  importableTeamCount: number
  createCount: number
  updateCount: number
  skipCount: number
  failureCount: number
  uniqueSkillCount: number
  uniqueMcpCount: number
  avatarCount: number
  filteredFileCount: number
  agents: ImportedAgentPreviewItem[]
  teams: ImportedTeamPreviewItem[]
  skills: ImportedSkillPreviewItem[]
  mcps: ImportedMcpPreviewItem[]
  avatars: ImportedAvatarPreviewItem[]
  issues: ImportIssue[]
}

export interface ExportWorkspaceAgentBundleInput {
  mode: 'single' | 'batch' | string
  agentIds: string[]
  teamIds: string[]
}

export interface ExportWorkspaceAgentBundleResult {
  rootDirName: string
  fileCount: number
  agentCount: number
  teamCount: number
  skillCount: number
  mcpCount: number
  avatarCount: number
  files: WorkspaceDirectoryUploadEntry[]
  issues: ImportIssue[]
}
