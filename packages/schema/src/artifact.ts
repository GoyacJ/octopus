export interface ArtifactRecord {
  id: string
  workspaceId: string
  projectId?: string
  title: string
  status: 'draft' | 'review' | 'approved' | 'published'
  latestVersion: number
  updatedAt: number
  storagePath?: string
  contentHash?: string
  byteSize?: number
  contentType?: string
}
