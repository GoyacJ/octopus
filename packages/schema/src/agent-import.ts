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

export interface ImportWorkspaceAgentBundlePreview {
  departments: string[]
  departmentCount: number
  detectedAgentCount: number
  importableAgentCount: number
  createCount: number
  updateCount: number
  skipCount: number
  failureCount: number
  uniqueSkillCount: number
  filteredFileCount: number
  agents: ImportedAgentPreviewItem[]
  skills: ImportedSkillPreviewItem[]
  issues: ImportIssue[]
}

export interface ImportWorkspaceAgentBundleResult {
  departments: string[]
  departmentCount: number
  detectedAgentCount: number
  importableAgentCount: number
  createCount: number
  updateCount: number
  skipCount: number
  failureCount: number
  uniqueSkillCount: number
  filteredFileCount: number
  agents: ImportedAgentPreviewItem[]
  skills: ImportedSkillPreviewItem[]
  issues: ImportIssue[]
}
