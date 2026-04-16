import type {
  KnowledgeEntryRecord as OpenApiKnowledgeEntryRecord,
  KnowledgeRecord as OpenApiKnowledgeRecord,
} from './generated'
import type { ConversationMemorySource } from './shared'

export type KnowledgeEntry = OpenApiKnowledgeEntryRecord
export type KnowledgeRecord = OpenApiKnowledgeRecord

export interface KnowledgeCandidate extends OpenApiKnowledgeRecord {
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
