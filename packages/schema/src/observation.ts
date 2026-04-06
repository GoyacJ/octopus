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

export interface AuditRecord {
  id: string
  workspaceId: string
  projectId?: string
  actorType: string
  actorId: string
  action: string
  resource: string
  outcome: string
  createdAt: number
}

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
