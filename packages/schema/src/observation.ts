import type { AuditRecord as OpenApiAuditRecord } from './generated'

export interface TraceEventRecord {
  id: string
  workspaceId: string
  projectId?: string
  runId?: string
  sessionId?: string
  eventKind: string
  title: string
  detail: string
  createdAt: number
}

export type AuditRecord = OpenApiAuditRecord

export interface CostLedgerEntry {
  id: string
  workspaceId: string
  projectId?: string
  runId?: string
  metric: string
  amount: number
  unit: string
  createdAt: number
}
