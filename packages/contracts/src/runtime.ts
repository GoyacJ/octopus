import type { ApprovalType, KnowledgeStatus, RunStatus, RunType, TriggerSource, TrustLevel } from './catalog'

export const approvalDecisionValues = ['approved', 'rejected'] as const
export const approvalStateValues = ['pending', 'approved', 'rejected', 'expired', 'cancelled'] as const
export const inboxStateValues = ['open', 'acknowledged', 'resolved', 'dismissed', 'expired'] as const
export const automationStateValues = ['draft', 'active', 'paused', 'suspended', 'archived'] as const
export const triggerDeliveryStateValues = [
  'pending',
  'claimed',
  'dispatched',
  'succeeded',
  'failed',
  'retried',
  'dead_letter',
] as const

export type ApprovalDecision = (typeof approvalDecisionValues)[number]
export type ApprovalState = (typeof approvalStateValues)[number]
export type InboxState = (typeof inboxStateValues)[number]
export type AutomationState = (typeof automationStateValues)[number]
export type TriggerDeliveryState = (typeof triggerDeliveryStateValues)[number]

export interface McpBinding {
  server_name: string
  event_name: string
}

export interface TaskSubmissionRequest {
  workspace_id: string
  project_id: string
  title: string
  description: string | null
  requested_by: string
  requires_approval: boolean
}

export interface AutomationCreateRequest {
  workspace_id: string
  project_id: string
  name: string
  trigger_source: TriggerSource
  requested_by: string
  requires_approval: boolean
  mcp_binding?: McpBinding | null
}

export interface AutomationStateUpdateRequest {
  state: AutomationState
}

export interface TriggerDeliveryRequest {
  trigger_id: string
  dedupe_key: string
  requested_by: string
  title?: string | null
  description?: string | null
}

export interface McpEventDeliveryRequest {
  server_name: string
  event_name: string
  dedupe_key: string
  requested_by: string
  title?: string | null
  description?: string | null
}

export interface ApprovalResolutionRequest {
  decision: ApprovalDecision
  reviewed_by: string
}

export interface KnowledgeSpaceCreateRequest {
  workspace_id: string
  name: string
  owner_refs: string[]
  scope: string
}

export interface KnowledgeCandidateCreateRequest {
  run_id: string
  knowledge_space_id: string
  created_by: string
}

export interface KnowledgePromotionRequest {
  promoted_by: string
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

export interface AutomationRecord {
  id: string
  workspace_id: string
  project_id: string
  name: string
  trigger_ids: string[]
  state: AutomationState
  requires_approval: boolean
  last_run_id: string | null
  created_at: string
  updated_at: string
}

export interface TriggerRecord {
  id: string
  automation_id: string
  source_type: TriggerSource
  dedupe_key: string
  owner_ref: string
  state: string
  created_at: string
  mcp_binding: McpBinding | null
}

export interface TriggerDeliveryRecord {
  id: string
  trigger_id: string
  source_type: TriggerSource
  dedupe_key: string
  state: TriggerDeliveryState
  run_id: string | null
  failure_reason: string | null
  occurred_at: string
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

export interface KnowledgeSpaceRecord {
  id: string
  workspace_id: string
  name: string
  owner_refs: string[]
  scope: string
  state: string
  created_at: string
  updated_at: string
}

export interface KnowledgeCandidateRecord {
  id: string
  knowledge_space_id: string
  run_id: string
  artifact_id: string
  title: string
  summary: string
  status: KnowledgeStatus
  trust_level: TrustLevel
  source_ref: string
  created_by: string
  created_at: string
  promoted_asset_id: string | null
}

export interface KnowledgeAssetRecord {
  id: string
  knowledge_space_id: string
  title: string
  summary: string
  layer: string
  status: KnowledgeStatus
  trust_level: TrustLevel
  source_ref: string
  created_at: string
}

export interface RunDetailResponse {
  run: RunRecord
  artifact: ArtifactRecord | null
  approval: ApprovalRequestRecord | null
  inbox_item: InboxItemRecord | null
  trace: TraceEvent[]
  audit: AuditEntry[]
}

export interface RunListResponse {
  items: RunRecord[]
}

export interface InboxListResponse {
  items: InboxItemRecord[]
}

export interface AutomationDetailResponse {
  automation: AutomationRecord
  trigger: TriggerRecord
  latest_delivery: TriggerDeliveryRecord | null
  latest_run: RunDetailResponse | null
}

export interface AutomationListResponse {
  items: AutomationDetailResponse[]
}

export interface TriggerDeliveryResponse {
  delivery: TriggerDeliveryRecord
  run: RunDetailResponse | null
}

export interface McpEventDeliveryResponse {
  items: TriggerDeliveryResponse[]
}

export interface KnowledgeSpaceDetailResponse {
  space: KnowledgeSpaceRecord
  candidates: KnowledgeCandidateRecord[]
  assets: KnowledgeAssetRecord[]
}

export interface KnowledgeSpaceListResponse {
  items: KnowledgeSpaceDetailResponse[]
}

export interface KnowledgeAssetListResponse {
  items: KnowledgeAssetRecord[]
}

export interface KnowledgeCandidateResponse {
  candidate: KnowledgeCandidateRecord
}

export interface KnowledgePromotionResponse {
  candidate: KnowledgeCandidateRecord
  asset: KnowledgeAssetRecord
}

export interface RuntimeEventEnvelope {
  sequence: number
  topic: string
  occurred_at: string
  run_id?: string | null
  workspace_id?: string | null
  automation_id?: string | null
  trigger_id?: string | null
  candidate_id?: string | null
  asset_id?: string | null
}

export interface ErrorResponse {
  message: string
}
