import type {
  ConversationMemorySource,
  KnowledgeKind,
  KnowledgePlaneScope,
  KnowledgeSourceType,
  KnowledgeStatus,
  KnowledgeVisibilityMode,
  RiskLevel,
} from './shared'

export interface KnowledgeEntry {
  id: string
  workspaceId: string
  projectId?: string
  conversationId?: string
  title: string
  kind: KnowledgeKind
  scope: KnowledgePlaneScope
  status: KnowledgeStatus
  visibility: KnowledgeVisibilityMode
  sourceType: KnowledgeSourceType
  sourceId: string
  summary: string
  ownerUserId?: string
  lastUsedAt: number
  trustLevel: RiskLevel
  lineage: string[]
  createdByMessageId?: string
}

export interface KnowledgeCandidate extends KnowledgeEntry {
  reviewNote: string
}

export interface ConversationMemoryItem {
  id: string
  conversationId: string
  title: string
  summary: string
  source: ConversationMemorySource
  ownerId?: string
  createdAt: number
  createdByMessageId?: string
}
