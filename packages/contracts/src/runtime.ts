import type { ApprovalType, RunStatus, RunType } from './catalog'

export const approvalDecisionValues = ['approved', 'rejected'] as const
export const approvalStateValues = ['pending', 'approved', 'rejected', 'expired', 'cancelled'] as const
export const inboxStateValues = ['open', 'acknowledged', 'resolved', 'dismissed', 'expired'] as const

export type ApprovalDecision = (typeof approvalDecisionValues)[number]
export type ApprovalState = (typeof approvalStateValues)[number]
export type InboxState = (typeof inboxStateValues)[number]

export interface TaskSubmissionRequest {
  project_id: string
  title: string
  description: string | null
  requested_by: string
  requires_approval: boolean
}

export interface ApprovalResolutionRequest {
  decision: ApprovalDecision
  reviewed_by: string
}

export interface RunRecord {
  id: string
  project_id: string
  run_type: RunType
  status: RunStatus
  idempotency_key: string
  requested_by: string
  title: string
  checkpoint_token: string | null
  created_at: string
  updated_at: string
}

export interface ArtifactRecord {
  id: string
  project_id: string
  run_id: string
  version: number
  title: string
  content_ref: string
  state: string
  created_at: string
}

export interface ApprovalRequestRecord {
  id: string
  run_id: string
  approval_type: ApprovalType
  state: ApprovalState
  target_ref: string
  requested_at: string
  reviewed_by: string | null
}

export interface InboxItemRecord {
  id: string
  workspace_id: string
  owner_ref: string
  state: InboxState
  priority: string
  target_ref: string
  dedupe_key: string
}

export interface TraceEvent {
  name: string
  message: string
  occurred_at: string
}

export interface AuditEntry {
  action: string
  actor: string
  target_ref: string
  occurred_at: string
}

export interface RunDetailResponse {
  run: RunRecord
  artifact: ArtifactRecord | null
  approval: ApprovalRequestRecord | null
  inbox_item: InboxItemRecord | null
  trace: TraceEvent[]
  audit: AuditEntry[]
}

export interface ErrorResponse {
  message: string
}
